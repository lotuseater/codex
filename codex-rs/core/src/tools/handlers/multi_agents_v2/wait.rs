use super::*;
use crate::agent::status::is_final;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use codex_protocol::ThreadId;
use codex_protocol::error::CodexErr;
use codex_protocol::protocol::CollabAgentRef;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch::Receiver;
use tokio::time::Instant;
use tokio::time::timeout_at;

pub(crate) struct Handler;

impl ToolHandler for Handler {
    type Output = WaitAgentResult;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            payload,
            call_id,
            ..
        } = invocation;
        let arguments = function_arguments(payload)?;
        let args: WaitArgs = parse_arguments(&arguments)?;
        let timeout_ms = args.timeout_ms.unwrap_or(DEFAULT_WAIT_TIMEOUT_MS);
        let min_timeout_ms = turn
            .config
            .multi_agent_v2
            .min_wait_timeout_ms
            .clamp(1, MAX_WAIT_TIMEOUT_MS);
        let timeout_ms = match timeout_ms {
            ms if ms <= 0 => {
                return Err(FunctionCallError::RespondToModel(
                    "timeout_ms must be greater than zero".to_owned(),
                ));
            }
            ms => ms.clamp(min_timeout_ms, MAX_WAIT_TIMEOUT_MS),
        };

        if let Some(targets) = args.targets {
            return wait_for_target_agents(session, turn, call_id, targets, timeout_ms).await;
        }

        let mut mailbox_seq_rx = session.subscribe_mailbox_seq();

        session
            .send_event(
                &turn,
                CollabWaitingBeginEvent {
                    sender_thread_id: session.conversation_id,
                    receiver_thread_ids: Vec::new(),
                    receiver_agents: Vec::new(),
                    call_id: call_id.clone(),
                }
                .into(),
            )
            .await;

        let timed_out = if session.has_pending_mailbox_items().await {
            false
        } else {
            let deadline = Instant::now() + Duration::from_millis(timeout_ms as u64);
            !wait_for_mailbox_change(&mut mailbox_seq_rx, deadline).await
        };
        let result = WaitAgentResult::from_timed_out(timed_out);

        session
            .send_event(
                &turn,
                CollabWaitingEndEvent {
                    sender_thread_id: session.conversation_id,
                    call_id,
                    agent_statuses: Vec::new(),
                    statuses: HashMap::new(),
                }
                .into(),
            )
            .await;

        Ok(result)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WaitArgs {
    targets: Option<Vec<String>>,
    timeout_ms: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct WaitAgentResult {
    pub(crate) message: String,
    pub(crate) status: HashMap<String, AgentStatus>,
    pub(crate) timed_out: bool,
}

impl WaitAgentResult {
    fn from_timed_out(timed_out: bool) -> Self {
        let message = if timed_out {
            "Wait timed out."
        } else {
            "Wait completed."
        };
        Self {
            message: message.to_string(),
            status: HashMap::new(),
            timed_out,
        }
    }

    fn from_statuses(status: HashMap<String, AgentStatus>, timed_out: bool) -> Self {
        let message = if timed_out {
            "Wait timed out."
        } else {
            "Wait completed."
        };
        Self {
            message: message.to_string(),
            status,
            timed_out,
        }
    }
}

impl ToolOutput for WaitAgentResult {
    fn log_preview(&self) -> String {
        tool_output_json_text(self, "wait_agent")
    }

    fn success_for_logging(&self) -> bool {
        true
    }

    fn to_response_item(&self, call_id: &str, payload: &ToolPayload) -> ResponseInputItem {
        tool_output_response_item(call_id, payload, self, /*success*/ None, "wait_agent")
    }

    fn code_mode_result(&self, _payload: &ToolPayload) -> JsonValue {
        tool_output_code_mode_result(self, "wait_agent")
    }
}

async fn wait_for_mailbox_change(
    mailbox_seq_rx: &mut tokio::sync::watch::Receiver<u64>,
    deadline: Instant,
) -> bool {
    match timeout_at(deadline, mailbox_seq_rx.changed()).await {
        Ok(Ok(())) => true,
        Ok(Err(_)) | Err(_) => false,
    }
}

async fn wait_for_target_agents(
    session: Arc<Session>,
    turn: Arc<TurnContext>,
    call_id: String,
    targets: Vec<String>,
    timeout_ms: i64,
) -> Result<WaitAgentResult, FunctionCallError> {
    if targets.is_empty() {
        return Err(FunctionCallError::RespondToModel(
            "agent targets must be non-empty".to_string(),
        ));
    }

    let mut receiver_thread_ids = Vec::with_capacity(targets.len());
    let mut receiver_agents = Vec::with_capacity(targets.len());
    let mut target_by_thread_id = HashMap::with_capacity(targets.len());
    for target in targets {
        let receiver_thread_id = resolve_agent_target(&session, &turn, &target).await?;
        let agent_metadata = session
            .services
            .agent_control
            .get_agent_metadata(receiver_thread_id)
            .unwrap_or_default();
        target_by_thread_id.insert(
            receiver_thread_id,
            agent_metadata
                .agent_path
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or(target),
        );
        receiver_agents.push(CollabAgentRef {
            thread_id: receiver_thread_id,
            agent_nickname: agent_metadata.agent_nickname,
            agent_role: agent_metadata.agent_role,
        });
        receiver_thread_ids.push(receiver_thread_id);
    }

    session
        .send_event(
            &turn,
            CollabWaitingBeginEvent {
                sender_thread_id: session.conversation_id,
                receiver_thread_ids: receiver_thread_ids.clone(),
                receiver_agents: receiver_agents.clone(),
                call_id: call_id.clone(),
            }
            .into(),
        )
        .await;

    let mut status_rxs = Vec::with_capacity(receiver_thread_ids.len());
    let mut statuses = HashMap::new();
    for id in &receiver_thread_ids {
        match session.services.agent_control.subscribe_status(*id).await {
            Ok(rx) => {
                let status = rx.borrow().clone();
                if is_final(&status) {
                    statuses.insert(*id, status);
                } else {
                    status_rxs.push((*id, rx));
                }
            }
            Err(CodexErr::ThreadNotFound(_)) => {
                statuses.insert(*id, AgentStatus::NotFound);
            }
            Err(err) => {
                let mut statuses = HashMap::with_capacity(1);
                statuses.insert(*id, session.services.agent_control.get_status(*id).await);
                session
                    .send_event(
                        &turn,
                        CollabWaitingEndEvent {
                            sender_thread_id: session.conversation_id,
                            call_id: call_id.clone(),
                            agent_statuses: build_wait_agent_statuses(&statuses, &receiver_agents),
                            statuses,
                        }
                        .into(),
                    )
                    .await;
                return Err(collab_agent_error(*id, err));
            }
        }
    }

    let mut futures = FuturesUnordered::new();
    for (id, rx) in status_rxs {
        futures.push(wait_for_final_status(session.clone(), id, rx));
    }
    let deadline = Instant::now() + Duration::from_millis(timeout_ms as u64);
    while statuses.len() < receiver_thread_ids.len() {
        match timeout_at(deadline, futures.next()).await {
            Ok(Some(Some((id, status)))) => {
                statuses.insert(id, status);
            }
            Ok(Some(None)) => continue,
            Ok(None) | Err(_) => break,
        }
    }

    let timed_out = statuses.len() < receiver_thread_ids.len();
    let statuses_by_id = statuses.clone();
    let agent_statuses = build_wait_agent_statuses(&statuses_by_id, &receiver_agents);
    let status = statuses
        .into_iter()
        .filter_map(|(thread_id, status)| {
            target_by_thread_id
                .get(&thread_id)
                .cloned()
                .map(|target| (target, status))
        })
        .collect();
    let result = WaitAgentResult::from_statuses(status, timed_out);

    session
        .send_event(
            &turn,
            CollabWaitingEndEvent {
                sender_thread_id: session.conversation_id,
                call_id,
                agent_statuses,
                statuses: statuses_by_id,
            }
            .into(),
        )
        .await;

    Ok(result)
}

async fn wait_for_final_status(
    session: Arc<Session>,
    thread_id: ThreadId,
    mut status_rx: Receiver<AgentStatus>,
) -> Option<(ThreadId, AgentStatus)> {
    let mut status = status_rx.borrow().clone();
    if is_final(&status) {
        return Some((thread_id, status));
    }

    loop {
        if status_rx.changed().await.is_err() {
            let latest = session.services.agent_control.get_status(thread_id).await;
            return is_final(&latest).then_some((thread_id, latest));
        }
        status = status_rx.borrow().clone();
        if is_final(&status) {
            return Some((thread_id, status));
        }
    }
}
