use super::*;
use crate::StateDbHandle;
use crate::agent::AgentMetadata;
use crate::agent::exceeds_thread_spawn_depth_limit;
use crate::agent::next_thread_spawn_depth;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::handlers::multi_agents_spec::create_resume_agent_tool_v2;
use crate::turn_timing::now_unix_timestamp_ms;
use codex_protocol::ThreadId;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_tool_registry_api::ToolSpec;
use std::sync::Arc;

pub(crate) struct Handler;

impl ToolExecutor<ToolInvocation> for Handler {
    type Output = ResumeAgentResult;

    fn tool_name(&self) -> ToolName {
        ToolName::plain("resume_agent")
    }

    fn spec(&self) -> Option<ToolSpec> {
        Some(create_resume_agent_tool_v2())
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
        let args: ResumeAgentArgs = parse_arguments(&arguments)?;
        let ResumeTarget {
            thread_id: receiver_thread_id,
            resolved_agent_path,
        } = resolve_resume_target(&session, &turn, &args.target).await?;
        let persisted_agent =
            persisted_agent_metadata(&session, receiver_thread_id, resolved_agent_path.clone())
                .await;
        let receiver_agent = session
            .services
            .agent_control
            .get_agent_metadata(receiver_thread_id)
            .or(persisted_agent)
            .unwrap_or_default();
        if receiver_agent
            .agent_path
            .as_ref()
            .is_some_and(AgentPath::is_root)
        {
            return Err(FunctionCallError::RespondToModel(
                "root is not a spawned agent".to_string(),
            ));
        }

        session
            .send_event(
                &turn,
                CollabResumeBeginEvent {
                    call_id: call_id.clone(),
                    started_at_ms: now_unix_timestamp_ms(),
                    sender_thread_id: session.thread_id,
                    receiver_thread_id,
                    receiver_agent_nickname: receiver_agent.agent_nickname.clone(),
                    receiver_agent_role: receiver_agent.agent_role.clone(),
                }
                .into(),
            )
            .await;

        let mut status = session
            .services
            .agent_control
            .get_status(receiver_thread_id)
            .await;
        let (receiver_agent, error) = if matches!(status, AgentStatus::NotFound) {
            match Box::pin(try_resume_closed_agent(
                &session,
                &turn,
                receiver_thread_id,
                receiver_agent.agent_path.clone(),
            ))
            .await
            {
                Ok(()) => {
                    status = session
                        .services
                        .agent_control
                        .get_status(receiver_thread_id)
                        .await;
                    (
                        session
                            .services
                            .agent_control
                            .get_agent_metadata(receiver_thread_id)
                            .unwrap_or(receiver_agent),
                        None,
                    )
                }
                Err(err) => {
                    status = session
                        .services
                        .agent_control
                        .get_status(receiver_thread_id)
                        .await;
                    (receiver_agent, Some(err))
                }
            }
        } else {
            (receiver_agent, None)
        };
        session
            .send_event(
                &turn,
                CollabResumeEndEvent {
                    call_id,
                    completed_at_ms: now_unix_timestamp_ms(),
                    sender_thread_id: session.thread_id,
                    receiver_thread_id,
                    receiver_agent_nickname: receiver_agent.agent_nickname,
                    receiver_agent_role: receiver_agent.agent_role,
                    status: status.clone(),
                }
                .into(),
            )
            .await;

        if let Some(err) = error {
            return Err(err);
        }
        turn.session_telemetry
            .counter("codex.multi_agent.resume", /*inc*/ 1, &[]);

        Ok(ResumeAgentResult { status })
    }
}

impl CoreToolRuntime for Handler {
    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResumeAgentArgs {
    target: String,
}

#[derive(Debug)]
pub(super) struct ResumeTarget {
    pub(super) thread_id: ThreadId,
    pub(super) resolved_agent_path: Option<AgentPath>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct ResumeAgentResult {
    pub(crate) status: AgentStatus,
}

impl ToolOutput for ResumeAgentResult {
    fn log_preview(&self) -> String {
        tool_output_json_text(self, "resume_agent")
    }

    fn success_for_logging(&self) -> bool {
        true
    }

    fn to_response_item(
        &self,
        call_id: &str,
        payload: &dyn ToolOutputPayload,
    ) -> ResponseInputItem {
        tool_output_response_item(call_id, payload, self, Some(true), "resume_agent")
    }

    fn code_mode_result(&self, _payload: &dyn ToolOutputPayload) -> JsonValue {
        tool_output_code_mode_result(self, "resume_agent")
    }
}

pub(super) async fn resolve_resume_target(
    session: &Arc<Session>,
    turn: &Arc<TurnContext>,
    target: &str,
) -> Result<ResumeTarget, FunctionCallError> {
    let target = target.trim();
    if target.is_empty() {
        return Err(FunctionCallError::RespondToModel(
            "target must not be empty".to_string(),
        ));
    }

    if let Ok(thread_id) = resolve_agent_target(session, turn, target).await {
        return Ok(ResumeTarget {
            thread_id,
            resolved_agent_path: None,
        });
    }
    if let Ok(thread_id) = ThreadId::from_string(target) {
        return Ok(ResumeTarget {
            thread_id,
            resolved_agent_path: None,
        });
    }

    let current_agent_path = turn
        .session_source
        .get_agent_path()
        .unwrap_or_else(AgentPath::root);
    let agent_path = current_agent_path
        .resolve(target)
        .map_err(FunctionCallError::RespondToModel)?;
    if agent_path.is_root() {
        return Err(FunctionCallError::RespondToModel(
            "root is not a spawned agent".to_string(),
        ));
    }

    let state_db = session.state_db().ok_or_else(|| {
        FunctionCallError::RespondToModel(
            "resume_agent requires persisted thread state to resolve closed task names".to_string(),
        )
    })?;
    let root_thread_id = persisted_spawn_root_thread_id(session, turn, &state_db).await?;
    let thread_id = state_db
        .find_thread_spawn_descendant_by_path(root_thread_id, agent_path.as_str())
        .await
        .map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "failed to resolve agent path `{}`: {err}",
                agent_path.as_str()
            ))
        })?
        .ok_or_else(|| {
            FunctionCallError::RespondToModel(format!(
                "agent path `{}` not found",
                agent_path.as_str()
            ))
        })?;

    Ok(ResumeTarget {
        thread_id,
        resolved_agent_path: Some(agent_path),
    })
}

async fn persisted_spawn_root_thread_id(
    session: &Arc<Session>,
    turn: &Arc<TurnContext>,
    state_db: &StateDbHandle,
) -> Result<ThreadId, FunctionCallError> {
    if !matches!(
        turn.session_source,
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. })
    ) {
        return Ok(session.thread_id);
    }

    if let Some(root_thread_id) = state_db
        .find_thread_spawn_root_for_descendant(session.thread_id)
        .await
        .map_err(|err| {
            FunctionCallError::RespondToModel(format!("failed to load persisted agent root: {err}"))
        })?
    {
        return Ok(root_thread_id);
    }

    match &turn.session_source {
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id, ..
        }) => Ok(*parent_thread_id),
        _ => Ok(session.thread_id),
    }
}

pub(super) async fn persisted_agent_metadata(
    session: &Arc<Session>,
    thread_id: ThreadId,
    fallback_agent_path: Option<AgentPath>,
) -> Option<AgentMetadata> {
    let metadata = session.state_db()?.get_thread(thread_id).await.ok()??;
    let source = parse_session_source(metadata.source.as_str());
    let source_agent_path = source.as_ref().and_then(SessionSource::get_agent_path);
    let source_agent_nickname = source.as_ref().and_then(SessionSource::get_nickname);
    let source_agent_role = source.as_ref().and_then(SessionSource::get_agent_role);
    let agent_path = metadata
        .agent_path
        .as_deref()
        .and_then(|path| AgentPath::try_from(path).ok())
        .or(source_agent_path)
        .or(fallback_agent_path);
    Some(AgentMetadata {
        agent_id: Some(thread_id),
        agent_path,
        agent_nickname: metadata.agent_nickname.or(source_agent_nickname),
        agent_role: metadata.agent_role.or(source_agent_role),
        model: metadata.model,
        reasoning_effort: metadata.reasoning_effort,
        last_task_message: None,
    })
}

pub(super) async fn try_resume_closed_agent(
    session: &Arc<Session>,
    turn: &Arc<TurnContext>,
    receiver_thread_id: ThreadId,
    fallback_agent_path: Option<AgentPath>,
) -> Result<(), FunctionCallError> {
    let state_db = session.state_db();
    let persisted_source = match state_db.as_ref() {
        Some(state_db) => {
            persisted_thread_spawn_source(
                state_db,
                receiver_thread_id,
                fallback_agent_path,
                session.thread_id,
                next_thread_spawn_depth(&turn.session_source),
            )
            .await?
        }
        None => PersistedThreadSpawnSource {
            parent_thread_id: session.thread_id,
            depth: next_thread_spawn_depth(&turn.session_source),
            agent_path: fallback_agent_path,
            agent_role: None,
        },
    };
    if exceeds_thread_spawn_depth_limit(persisted_source.depth, turn.config.agent_max_depth) {
        return Err(FunctionCallError::RespondToModel(
            "Agent depth limit reached. Solve the task yourself.".to_string(),
        ));
    }

    let config = build_agent_resume_config(turn.as_ref())?;
    let source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: persisted_source.parent_thread_id,
        depth: persisted_source.depth,
        agent_path: persisted_source.agent_path,
        agent_nickname: None,
        agent_role: persisted_source.agent_role,
    });
    Box::pin(session.services.agent_control.resume_agent_from_rollout(
        config,
        receiver_thread_id,
        source,
    ))
    .await
    .map(|_| ())
    .map_err(|err| collab_agent_error(receiver_thread_id, err))
}

#[derive(Debug)]
struct PersistedThreadSpawnSource {
    parent_thread_id: ThreadId,
    depth: i32,
    agent_path: Option<AgentPath>,
    agent_role: Option<String>,
}

async fn persisted_thread_spawn_source(
    state_db: &StateDbHandle,
    receiver_thread_id: ThreadId,
    fallback_agent_path: Option<AgentPath>,
    fallback_parent_thread_id: ThreadId,
    fallback_depth: i32,
) -> Result<PersistedThreadSpawnSource, FunctionCallError> {
    let metadata = state_db
        .get_thread(receiver_thread_id)
        .await
        .map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "failed to load persisted agent metadata: {err}"
            ))
        })?;
    let source = metadata
        .as_ref()
        .and_then(|metadata| parse_session_source(metadata.source.as_str()));
    let (source_parent_thread_id, source_depth, source_agent_path, source_agent_role) = match source
    {
        Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id,
            depth,
            agent_path,
            agent_role,
            ..
        })) => (Some(parent_thread_id), Some(depth), agent_path, agent_role),
        _ => (None, None, None, None),
    };
    let edge_parent_thread_id = state_db
        .find_thread_spawn_parent(receiver_thread_id)
        .await
        .map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "failed to load persisted agent parent: {err}"
            ))
        })?;
    let metadata_agent_path = metadata
        .as_ref()
        .and_then(|metadata| metadata.agent_path.as_deref())
        .and_then(|path| AgentPath::try_from(path).ok());

    Ok(PersistedThreadSpawnSource {
        parent_thread_id: edge_parent_thread_id
            .or(source_parent_thread_id)
            .unwrap_or(fallback_parent_thread_id),
        depth: source_depth.unwrap_or(fallback_depth),
        agent_path: metadata_agent_path
            .or(source_agent_path)
            .or(fallback_agent_path),
        agent_role: metadata
            .and_then(|metadata| metadata.agent_role)
            .or(source_agent_role),
    })
}

fn parse_session_source(source: &str) -> Option<SessionSource> {
    serde_json::from_str(source)
        .or_else(|_| serde_json::from_value(serde_json::Value::String(source.to_string())))
        .ok()
}
