use super::*;
use crate::session::InputQueueActivity;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::handlers::multi_agents::parse_agent_id_targets;
use crate::tools::handlers::multi_agents_spec::WaitAgentTimeoutOptions;
use crate::tools::handlers::multi_agents_spec::create_wait_agent_tool_v2;
use crate::turn_timing::now_unix_timestamp_ms;
use codex_protocol::ThreadId;
use codex_protocol::protocol::CollabAgentRef;
use codex_tool_registry_api::ToolSpec;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use tokio::time::timeout_at;

#[derive(Default)]
pub(crate) struct Handler {
    options: WaitAgentTimeoutOptions,
}

impl Handler {
    pub(crate) fn new(options: WaitAgentTimeoutOptions) -> Self {
        Self { options }
    }
}

impl ToolExecutor<ToolInvocation> for Handler {
    type Output = Box<dyn crate::tools::context::ToolOutput>;

    fn tool_name(&self) -> ToolName {
        ToolName::plain("wait_agent")
    }

    fn spec(&self) -> Option<ToolSpec> {
        Some(create_wait_agent_tool_v2(self.options))
    }

    async fn handle(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn crate::tools::context::ToolOutput>, FunctionCallError> {
        self.handle_call(invocation).await
    }
}

impl Handler {
    async fn handle_call(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn crate::tools::context::ToolOutput>, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            payload,
            call_id,
            ..
        } = invocation;
        let arguments = function_arguments(payload)?;
        let args: WaitArgs = parse_arguments(&arguments)?;
        let min_timeout_ms = turn.config.multi_agent_v2.min_wait_timeout_ms;
        let max_timeout_ms = turn.config.multi_agent_v2.max_wait_timeout_ms;
        let default_timeout_ms = turn.config.multi_agent_v2.default_wait_timeout_ms;
        let timeout_ms = match args.timeout_ms {
            Some(ms) if ms < min_timeout_ms => {
                return Err(FunctionCallError::RespondToModel(format!(
                    "timeout_ms must be at least {min_timeout_ms}"
                )));
            }
            Some(ms) if ms > max_timeout_ms => {
                return Err(FunctionCallError::RespondToModel(format!(
                    "timeout_ms must be at most {max_timeout_ms}"
                )));
            }
            Some(ms) => ms,
            None => default_timeout_ms,
        };

        if args.targets.is_empty() {
            return self
                .wait_for_mailbox_activity(session, turn, call_id, timeout_ms)
                .await;
        }

        self.wait_for_targets(session, turn, call_id, args.targets, timeout_ms)
            .await
    }

    async fn wait_for_mailbox_activity(
        &self,
        session: Arc<Session>,
        turn: Arc<TurnContext>,
        call_id: String,
        timeout_ms: i64,
    ) -> Result<Box<dyn crate::tools::context::ToolOutput>, FunctionCallError> {
        let turn_state = session
            .input_queue
            .turn_state_for_sub_id(&session.active_turn, &turn.sub_id)
            .await;
        let (mut activity_rx, pending_activity) = session
            .input_queue
            .subscribe_activity(turn_state.as_deref())
            .await;

        session
            .emit_turn_item_started(
                &turn,
                &TurnItem::CollabAgentToolCall(CollabAgentToolCallItem {
                    id: call_id.clone(),
                    tool: CollabAgentTool::Wait,
                    status: CollabAgentToolCallStatus::InProgress,
                    sender_thread_id: session.thread_id,
                    receiver_thread_ids: Vec::new(),
                    receiver_agents: Vec::new(),
                    prompt: None,
                    model: None,
                    reasoning_effort: None,
                    agents_states: Default::default(),
                }),
            )
            .await;

        let deadline = Instant::now() + Duration::from_millis(timeout_ms as u64);
        let outcome = wait_for_activity(&mut activity_rx, pending_activity, deadline).await;
        let result = WaitAgentResult::from_outcome(outcome);

        session
            .emit_turn_item_completed(
                &turn,
                TurnItem::CollabAgentToolCall(CollabAgentToolCallItem {
                    id: call_id,
                    tool: CollabAgentTool::Wait,
                    status: CollabAgentToolCallStatus::Completed,
                    sender_thread_id: session.thread_id,
                    receiver_thread_ids: Vec::new(),
                    receiver_agents: Vec::new(),
                    prompt: None,
                    model: None,
                    reasoning_effort: None,
                    agents_states: HashMap::new(),
                }),
            )
            .await;

        Ok(boxed_tool_output(result))
    }

    async fn wait_for_targets(
        &self,
        session: Arc<Session>,
        turn: Arc<TurnContext>,
        call_id: String,
        targets: Vec<String>,
        timeout_ms: i64,
    ) -> Result<Box<dyn crate::tools::context::ToolOutput>, FunctionCallError> {
        let receiver_thread_ids = parse_agent_id_targets(targets)?;
        let mut receiver_agents = Vec::with_capacity(receiver_thread_ids.len());
        let mut target_by_thread_id = HashMap::with_capacity(receiver_thread_ids.len());
        for receiver_thread_id in &receiver_thread_ids {
            let agent_metadata = session
                .services
                .agent_control
                .get_agent_metadata(*receiver_thread_id)
                .unwrap_or_default();
            target_by_thread_id.insert(
                *receiver_thread_id,
                agent_metadata
                    .agent_path
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| receiver_thread_id.to_string()),
            );
            receiver_agents.push(CollabAgentRef {
                thread_id: *receiver_thread_id,
                agent_nickname: agent_metadata.agent_nickname,
                agent_role: agent_metadata.agent_role,
            });
        }

        session
            .send_event(
                &turn,
                CollabWaitingBeginEvent {
                    started_at_ms: now_unix_timestamp_ms(),
                    sender_thread_id: session.thread_id,
                    receiver_thread_ids: receiver_thread_ids.clone(),
                    receiver_agents: receiver_agents.clone(),
                    call_id: call_id.clone(),
                }
                .into(),
            )
            .await;

        let pairs =
            wait_for_targets_final_status(&session, &receiver_thread_ids, timeout_ms).await?;

        let timed_out = pairs.is_empty();
        let statuses_by_id = pairs.iter().cloned().collect::<HashMap<ThreadId, _>>();
        let agent_statuses = build_wait_agent_statuses(&statuses_by_id, &receiver_agents);
        let status = pairs
            .into_iter()
            .filter_map(|(thread_id, status)| {
                target_by_thread_id
                    .get(&thread_id)
                    .cloned()
                    .map(|target| (target, status))
            })
            .collect::<HashMap<String, AgentStatus>>();
        let message = if timed_out {
            "Wait timed out."
        } else {
            "Wait completed."
        };
        let result = WaitAgentResult {
            message: message.to_string(),
            timed_out,
            status,
        };

        session
            .send_event(
                &turn,
                CollabWaitingEndEvent {
                    sender_thread_id: session.thread_id,
                    call_id,
                    completed_at_ms: now_unix_timestamp_ms(),
                    agent_statuses,
                    statuses: statuses_by_id,
                }
                .into(),
            )
            .await;

        Ok(boxed_tool_output(result))
    }
}

impl CoreToolRuntime for Handler {
    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WaitArgs {
    #[serde(default)]
    targets: Vec<String>,
    timeout_ms: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct WaitAgentResult {
    pub(crate) message: String,
    pub(crate) timed_out: bool,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) status: HashMap<String, AgentStatus>,
}

impl WaitAgentResult {
    fn from_outcome(outcome: WaitOutcome) -> Self {
        let message = match outcome {
            WaitOutcome::MailboxActivity => "Wait completed.",
            WaitOutcome::Steered => "Wait interrupted by new input.",
            WaitOutcome::TimedOut => "Wait timed out.",
        };
        Self {
            message: message.to_string(),
            timed_out: outcome == WaitOutcome::TimedOut,
            status: HashMap::new(),
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

    fn to_response_item(
        &self,
        call_id: &str,
        payload: &dyn ToolOutputPayload,
    ) -> ResponseInputItem {
        tool_output_response_item(call_id, payload, self, /*success*/ None, "wait_agent")
    }

    fn code_mode_result(&self, _payload: &dyn ToolOutputPayload) -> JsonValue {
        tool_output_code_mode_result(self, "wait_agent")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WaitOutcome {
    MailboxActivity,
    Steered,
    TimedOut,
}

async fn wait_for_activity(
    activity_rx: &mut tokio::sync::watch::Receiver<InputQueueActivity>,
    pending_activity: Option<InputQueueActivity>,
    deadline: Instant,
) -> WaitOutcome {
    if let Some(activity) = pending_activity {
        return match activity {
            InputQueueActivity::Mailbox => WaitOutcome::MailboxActivity,
            InputQueueActivity::Steer => WaitOutcome::Steered,
        };
    }
    match timeout_at(deadline, activity_rx.changed()).await {
        Ok(Ok(())) => match *activity_rx.borrow_and_update() {
            InputQueueActivity::Mailbox => WaitOutcome::MailboxActivity,
            InputQueueActivity::Steer => WaitOutcome::Steered,
        },
        Ok(Err(_)) | Err(_) => WaitOutcome::TimedOut,
    }
}
