use std::collections::HashSet;
use std::sync::Arc;
use std::sync::OnceLock;

use crate::Prompt;
use crate::client::CompactConversationRequestSettings;
use crate::compact::CompactionAnalyticsAttempt;
use crate::compact::CompactionAnalyticsDetails;
use crate::compact::InitialContextInjection;
use crate::compact::build_compaction_initial_context;
use crate::compact::compaction_status_from_result;
use crate::compact::insert_initial_context_before_last_real_user_or_summary;
use crate::context::world_state::WorldState;
use crate::context_manager::ContextManager;
use crate::hook_runtime::PostCompactHookOutcome;
use crate::hook_runtime::PreCompactHookOutcome;
use crate::hook_runtime::run_post_compact_hooks;
use crate::hook_runtime::run_pre_compact_hooks;
use crate::responses_metadata::CodexResponsesRequestKind;
use crate::responses_metadata::CompactionTurnMetadata;
use crate::session::session::Session;
use crate::session::step_context::StepContext;
use crate::session::turn::built_tools;
use crate::session::turn_context::TurnContext;
use codex_analytics::CompactionImplementation;
use codex_analytics::CompactionPhase;
use codex_analytics::CompactionReason;
use codex_analytics::CompactionTrigger;
use codex_prompt_reducer::PromptReductionStats;
use codex_protocol::auth::AuthMode;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::items::ContextCompactionItem;
use codex_protocol::items::TurnItem;
use codex_protocol::models::BaseInstructions;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::CompactedItem;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::TurnStartedEvent;
use codex_rollout_trace::CompactionCheckpointTracePayload;
use codex_utils_output_truncation::approx_token_count;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing::warn;

const CONTEXT_WINDOW_TRUNCATED_OUTPUT_MESSAGE: &str =
    "Output exceeded the available model context and was truncated";

pub(crate) async fn run_inline_remote_auto_compact_task(
    sess: Arc<Session>,
    step_context: Arc<StepContext>,
    fallback_step_context: Option<Arc<StepContext>>,
    turn_state: Arc<OnceLock<String>>,
    initial_context_injection: InitialContextInjection,
    reason: CompactionReason,
    phase: CompactionPhase,
) -> CodexResult<()> {
    let compaction_metadata = CompactionTurnMetadata::new(
        CompactionTrigger::Auto,
        reason,
        CompactionImplementation::ResponsesCompact,
        phase,
    );
    run_remote_compact_task_inner(
        &sess,
        &step_context,
        fallback_step_context.as_ref(),
        Some(turn_state),
        initial_context_injection,
        compaction_metadata,
    )
    .await?;
    Ok(())
}

pub(crate) async fn run_remote_compact_task(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
) -> CodexResult<()> {
    // Standalone compaction is its own request boundary, so it captures a fresh step.
    let step_context = sess.capture_step_context(Arc::clone(&turn_context)).await;
    let start_event = EventMsg::TurnStarted(TurnStartedEvent {
        turn_id: turn_context.sub_id.clone(),
        trace_id: turn_context.trace_id.clone(),
        started_at: turn_context.turn_timing_state.started_at_unix_secs().await,
        model_context_window: turn_context.model_context_window(),
        collaboration_mode_kind: turn_context.collaboration_mode.mode,
    });
    sess.send_event(&turn_context, start_event).await;

    let compaction_metadata = CompactionTurnMetadata::new(
        CompactionTrigger::Manual,
        CompactionReason::UserRequested,
        CompactionImplementation::ResponsesCompact,
        CompactionPhase::StandaloneTurn,
    );
    run_remote_compact_task_inner(
        &sess,
        &step_context,
        /*fallback_step_context*/ None,
        /*turn_state*/ None,
        InitialContextInjection::DoNotInject,
        compaction_metadata,
    )
    .await?;
    Ok(())
}

async fn run_remote_compact_task_inner(
    sess: &Arc<Session>,
    step_context: &Arc<StepContext>,
    fallback_step_context: Option<&Arc<StepContext>>,
    turn_state: Option<Arc<OnceLock<String>>>,
    initial_context_injection: InitialContextInjection,
    compaction_metadata: CompactionTurnMetadata,
) -> CodexResult<()> {
    let turn_context = &step_context.turn;
    let trigger = compaction_metadata.trigger();
    let reason = compaction_metadata.reason();
    let implementation = compaction_metadata.implementation();
    let phase = compaction_metadata.phase();
    let mut analytics_details = CompactionAnalyticsDetails {
        active_context_tokens_before: Some(sess.get_total_token_usage().await),
        ..Default::default()
    };
    let attempt = CompactionAnalyticsAttempt::begin(
        sess.as_ref(),
        turn_context.as_ref(),
        trigger,
        reason,
        implementation,
        phase,
    )
    .await;
    let pre_compact_outcome = run_pre_compact_hooks(sess, turn_context, trigger).await;
    match pre_compact_outcome {
        PreCompactHookOutcome::Continue => {}
        PreCompactHookOutcome::Stopped => {
            let error = CodexErr::TurnAborted;
            attempt
                .track(
                    sess.as_ref(),
                    codex_analytics::CompactionStatus::Interrupted,
                    Some(&error),
                    analytics_details,
                )
                .await;
            return Err(error);
        }
    }
    let result = run_remote_compact_task_inner_impl(
        sess,
        step_context,
        fallback_step_context,
        turn_state,
        initial_context_injection,
        compaction_metadata,
        &mut analytics_details,
    )
    .await;
    let status = compaction_status_from_result(&result);
    let codex_error = result.as_ref().err();
    if result.is_ok() {
        let post_compact_outcome = run_post_compact_hooks(sess, turn_context, trigger).await;
        if let PostCompactHookOutcome::Stopped = post_compact_outcome {
            attempt
                .track(sess.as_ref(), status, codex_error, analytics_details)
                .await;
            return Err(CodexErr::TurnAborted);
        }
    }
    attempt
        .track(sess.as_ref(), status, codex_error, analytics_details)
        .await;
    if let Err(err) = result {
        sess.track_turn_codex_error(turn_context, &err);
        let event = EventMsg::Error(
            err.to_error_event(Some("Error running remote compact task".to_string())),
        );
        sess.send_event(turn_context, event).await;
        return Err(err);
    }
    Ok(())
}

async fn run_remote_compact_task_inner_impl(
    sess: &Arc<Session>,
    step_context: &Arc<StepContext>,
    fallback_step_context: Option<&Arc<StepContext>>,
    turn_state: Option<Arc<OnceLock<String>>>,
    initial_context_injection: InitialContextInjection,
    compaction_metadata: CompactionTurnMetadata,
    // fork-local: the budget-based remote compaction loop below does not adjust the
    // caller's local-token estimate the way upstream's single-shot path did, so the
    // analytics_details handle is currently unused here. Keep upstream's type so the
    // caller (`run_remote_compact_task_inner`) passes `&mut analytics_details` unchanged.
    _analytics_details: &mut CompactionAnalyticsDetails,
) -> CodexResult<()> {
    let turn_context = &step_context.turn;
    let context_compaction_item = ContextCompactionItem::new();
    let compaction_id = context_compaction_item.id.clone();
    // Use the UI compaction item ID as the trace compaction ID so protocol lifecycle events,
    // endpoint attempts, and the installed history checkpoint all have one join key.
    let compaction_trace = sess.services.rollout_thread_trace.compaction_trace_context(
        turn_context.sub_id.as_str(),
        compaction_id.as_str(),
        turn_context.model_info.slug.as_str(),
        turn_context.provider.info().name.as_str(),
    );
    let compaction_item = TurnItem::ContextCompaction(context_compaction_item);
    sess.emit_turn_item_started(turn_context, &compaction_item)
        .await;
    let history = sess.clone_history().await;
    let base_instructions = sess.get_base_instructions().await;
    let budgets = remote_compaction_token_budgets(turn_context.model_context_window());
    for (attempt_index, token_budget) in budgets.iter().copied().enumerate() {
        let prepared = prepare_remote_compaction_prompt(
            history.clone(),
            turn_context.as_ref(),
            &base_instructions,
            false,
            token_budget,
        );
        log_remote_compaction_prompt_fit(
            turn_context.as_ref(),
            "remote",
            attempt_index,
            token_budget,
            &prepared,
        );
        // This is the history selected for remote compaction, after any trimming required to fit the
        // compact endpoint. The checkpoint below records it separately from the next sampling request,
        // whose prompt will repeat current developer/context prefix items.
        let trace_input_history = prepared.history.raw_items().to_vec();
        // fork seam: task-memory orchestration lives in task_memory.rs / codex-task-memory
        let mut task_memory =
            crate::task_memory::CompactionTaskMemory::from_history(&trace_input_history);
        let tool_router = built_tools(
            sess.as_ref(),
            step_context.as_ref(),
            // Compaction builds the full tool router; it has no user-mention input to drive
            // per-input connector enablement, no explicit per-turn connectors, and loads no
            // skills (session-level connector selection is still applied inside `built_tools`).
            &[],
            &HashSet::new(),
            None,
            &CancellationToken::new(),
        )
        .await?;
        let prompt = Prompt {
            input: prepared.input,
            tools: tool_router.model_visible_specs(),
            parallel_tool_calls: turn_context.model_info.supports_parallel_tool_calls,
            base_instructions: base_instructions.clone(),
            personality: turn_context.personality,
            output_schema: None,
            output_schema_strict: true,
        };
        let is_api_key_auth = sess.services.auth_manager.auth_mode() == Some(AuthMode::ApiKey);
        let window_id = sess.current_window_id().await;
        // SYMBOL MOVE: `current_header_value_for_compaction` -> `to_responses_metadata`
        // (turn_metadata::CompactionTurnMetadata -> responses_metadata::CompactionTurnMetadata).
        // `compaction_metadata` is `Copy`, so it can be reused across budget attempts.
        let responses_metadata = turn_context.turn_metadata_state.to_responses_metadata(
            sess.installation_id.clone(),
            window_id,
            CodexResponsesRequestKind::Compaction(compaction_metadata),
        );
        let result = sess
            .services
            .model_client
            .compact_conversation_history(
                &prompt,
                &turn_context.model_info,
                turn_state.clone(),
                CompactConversationRequestSettings {
                    effort: turn_context.reasoning_effort.clone(),
                    summary: turn_context.reasoning_summary,
                    service_tier: if is_api_key_auth {
                        None
                    } else {
                        turn_context.config.service_tier.clone()
                    },
                },
                &turn_context.session_telemetry,
                &compaction_trace,
                &responses_metadata,
            )
            .await;

        let new_history = match result {
            Ok(new_history) => new_history,
            Err(err) => {
                if matches!(err, CodexErr::ContextWindowExceeded)
                    && attempt_index + 1 < budgets.len()
                {
                    continue;
                }
                return Err(err);
            }
        };

        let (new_window_number, new_window_ids) = sess.advance_auto_compact_window().await;
        let (new_history, _world_state_baseline) = process_compacted_history(
            sess.as_ref(),
            turn_context.as_ref(),
            new_history,
            &initial_context_injection,
            &mut task_memory,
        )
        .await;

        let reference_context_item = match initial_context_injection {
            InitialContextInjection::DoNotInject => None,
            InitialContextInjection::BeforeLastUserMessage(_) => {
                Some(turn_context.to_turn_context_item())
            }
        };
        let compacted_item = CompactedItem {
            message: String::new(),
            replacement_history: Some(new_history.clone()),
            window_number: Some(new_window_number),
            first_window_id: Some(new_window_ids.first_window_id.to_string()),
            previous_window_id: new_window_ids.previous_window_id.map(|id| id.to_string()),
            window_id: Some(new_window_ids.window_id.to_string()),
        };
        // Install is the semantic boundary where the compact endpoint's output becomes live
        // thread history. Keep it distinct from the later inference request so the reducer can
        // still represent repeated developer/context prefix items exactly as the model saw them.
        compaction_trace.record_installed(&CompactionCheckpointTracePayload {
            input_history: &trace_input_history,
            replacement_history: &new_history,
        });
        sess.replace_compacted_history(new_history, reference_context_item, compacted_item)
            .await;
        // fork seam: task-memory orchestration lives in task_memory.rs / codex-task-memory
        sess.reset_task_memory_throttle_after_compaction(task_memory.digest())
            .await;
        sess.recompute_token_usage(turn_context).await;

        sess.emit_turn_item_completed(turn_context, compaction_item)
            .await;
        return Ok(());
    }

    Err(CodexErr::ContextWindowExceeded)
}

pub(crate) async fn process_compacted_history(
    sess: &Session,
    turn_context: &TurnContext,
    mut compacted_history: Vec<ResponseItem>,
    initial_context_injection: &InitialContextInjection,
    task_memory: &mut crate::task_memory::CompactionTaskMemory,
) -> (Vec<ResponseItem>, Option<Arc<WorldState>>) {
    // Mid-turn compaction is the only path that must inject initial context above the last user
    // message in the replacement history. Pre-turn compaction instead injects context after the
    // compaction item, but mid-turn compaction keeps the compaction item last for model training.
    let (mut injected_context, world_state_baseline) =
        build_compaction_initial_context(sess, turn_context, initial_context_injection).await;
    // fork seam: task-memory orchestration lives in task_memory.rs / codex-task-memory
    task_memory.push_into_replacement_context(&mut injected_context);

    crate::task_memory::CompactionTaskMemory::remove_from_history(&mut compacted_history);
    compacted_history.retain(should_keep_compacted_history_item);
    (
        insert_initial_context_before_last_real_user_or_summary(
            compacted_history,
            injected_context,
        ),
        world_state_baseline,
    )
}

/// Returns whether an item from remote compaction output should be preserved.
///
/// Called while processing the model-provided compacted transcript, before we
/// append fresh canonical context from the current session.
///
/// We drop:
/// - `developer` messages because remote output can include stale/duplicated
///   instruction content.
/// - non-user-content `user` messages (session prefix/instruction wrappers),
///   while preserving real user messages and persisted hook prompts.
///
/// This intentionally keeps:
/// - `assistant` messages (future remote compaction models may emit them)
/// - `user`-role warnings that parse as `TurnItem::UserMessage` and compaction-generated summary
///   messages. Legacy warning fragments are filtered by `parse_turn_item` before they reach this
///   check.
pub(crate) fn should_keep_compacted_history_item(item: &ResponseItem) -> bool {
    match item {
        ResponseItem::Message { role, .. } if role == "developer" => false,
        ResponseItem::Message { role, .. } if role == "user" => {
            matches!(
                crate::event_mapping::parse_turn_item(item),
                Some(TurnItem::UserMessage(_) | TurnItem::HookPrompt(_))
            )
        }
        ResponseItem::Message { role, .. } if role == "assistant" => true,
        ResponseItem::Message { .. } => false,
        ResponseItem::AgentMessage { .. } => true,
        ResponseItem::Compaction { .. } | ResponseItem::ContextCompaction { .. } => true,
        ResponseItem::CompactionTrigger { .. } => false,
        ResponseItem::AdditionalTools { .. }
        | ResponseItem::Reasoning { .. }
        | ResponseItem::LocalShellCall { .. }
        | ResponseItem::FunctionCall { .. }
        | ResponseItem::ToolSearchCall { .. }
        | ResponseItem::FunctionCallOutput { .. }
        | ResponseItem::ToolSearchOutput { .. }
        | ResponseItem::CustomToolCall { .. }
        | ResponseItem::CustomToolCallOutput { .. }
        | ResponseItem::WebSearchCall { .. }
        | ResponseItem::ImageGenerationCall { .. }
        | ResponseItem::Other => false,
    }
}

pub(crate) fn trim_function_call_history_to_fit_context_window(
    history: &mut ContextManager,
    turn_context: &TurnContext,
    base_instructions: &BaseInstructions,
) -> (usize, i64) {
    let Some(context_window) = turn_context.model_context_window() else {
        return (0, 0);
    };
    let mut rewritten_outputs = 0usize;
    let mut estimated_deleted_tokens = 0i64;
    let item_count = history.raw_items().len();

    for index in (0..item_count).rev() {
        let Some(estimated_tokens_before) =
            history.estimate_token_count_with_base_instructions(base_instructions)
        else {
            break;
        };
        if estimated_tokens_before <= context_window {
            break;
        }
        let Some(rewritten_item) = history
            .raw_items()
            .get(index)
            .and_then(rewritten_output_for_context_window)
        else {
            break;
        };
        let mut items = history.raw_items().to_vec();
        items[index] = rewritten_item;
        history.replace(items);
        let estimated_tokens_after = history
            .estimate_token_count_with_base_instructions(base_instructions)
            .unwrap_or_default();
        rewritten_outputs += 1;
        estimated_deleted_tokens = estimated_deleted_tokens
            .saturating_add(estimated_tokens_before.saturating_sub(estimated_tokens_after));
    }

    (rewritten_outputs, estimated_deleted_tokens)
}

pub(crate) struct PreparedRemoteCompactionPrompt {
    pub(crate) history: ContextManager,
    pub(crate) input: Vec<ResponseItem>,
    pub(crate) deleted_items: usize,
    pub(crate) reduction_stats: Option<PromptReductionStats>,
    pub(crate) estimated_tokens: i64,
}

pub(crate) fn remote_compaction_token_budgets(context_window: Option<i64>) -> Vec<i64> {
    let Some(context_window) = context_window else {
        return vec![i64::MAX];
    };

    [95, 80, 65, 50]
        .into_iter()
        .map(|percent| context_window.saturating_mul(percent) / 100)
        .map(|budget| budget.max(context_window / 2))
        .fold(Vec::new(), |mut budgets, budget| {
            if budgets.last().copied() != Some(budget) {
                budgets.push(budget);
            }
            budgets
        })
}

pub(crate) fn prepare_remote_compaction_prompt(
    mut history: ContextManager,
    turn_context: &TurnContext,
    base_instructions: &BaseInstructions,
    include_context_compaction_item: bool,
    token_budget: i64,
) -> PreparedRemoteCompactionPrompt {
    let mut deleted_items = trim_remote_compaction_history_to_token_budget(
        &mut history,
        base_instructions,
        token_budget,
    );

    loop {
        let mut input = history
            .clone()
            .for_prompt(&turn_context.model_info.input_modalities);
        if include_context_compaction_item {
            input.push(ResponseItem::ContextCompaction {
                id: None,
                encrypted_content: None,
                internal_chat_message_metadata_passthrough: None,
            });
        }
        let reduction_stats = reduce_remote_compaction_prompt_input(&mut input, turn_context);
        let estimated_tokens = estimate_remote_compaction_prompt_tokens(&input, base_instructions);
        if estimated_tokens <= token_budget {
            return PreparedRemoteCompactionPrompt {
                history,
                input,
                deleted_items,
                reduction_stats,
                estimated_tokens,
            };
        }

        let before = history.raw_items().len();
        if before == 0 {
            return PreparedRemoteCompactionPrompt {
                history,
                input,
                deleted_items,
                reduction_stats,
                estimated_tokens,
            };
        }
        history.remove_first_item();
        let removed = before.saturating_sub(history.raw_items().len());
        if removed == 0 {
            return PreparedRemoteCompactionPrompt {
                history,
                input,
                deleted_items,
                reduction_stats,
                estimated_tokens,
            };
        }
        deleted_items += removed;
    }
}

fn trim_remote_compaction_history_to_token_budget(
    history: &mut ContextManager,
    base_instructions: &BaseInstructions,
    token_budget: i64,
) -> usize {
    let mut deleted_items = 0usize;
    while history
        .estimate_token_count_with_base_instructions(base_instructions)
        .is_some_and(|estimated_tokens| estimated_tokens > token_budget)
    {
        let Some(last_item) = history.raw_items().last() else {
            break;
        };
        if !is_codex_generated_item(last_item) {
            break;
        }
        // fork-local: upstream removed `ContextManager::remove_last_item`; pop the
        // trailing item via `replace` (the only public API that can drop from the
        // back) to keep the budget-based trailing-trim behavior.
        let mut items = history.raw_items().to_vec();
        if items.pop().is_none() {
            break;
        }
        history.replace(items);
        deleted_items += 1;
    }

    while history
        .estimate_token_count_with_base_instructions(base_instructions)
        .is_some_and(|estimated_tokens| estimated_tokens > token_budget)
    {
        let before = history.raw_items().len();
        if before == 0 {
            break;
        }
        history.remove_first_item();
        let removed = before.saturating_sub(history.raw_items().len());
        if removed == 0 {
            break;
        }
        deleted_items += removed;
    }

    deleted_items
}

// fork-local: upstream removed `context_manager::is_codex_generated_item` during the
// merge; the fork's budget-based remote compaction still needs it to decide which
// trailing items may be dropped, so it is restored locally here.
fn is_codex_generated_item(item: &ResponseItem) -> bool {
    matches!(
        item,
        ResponseItem::FunctionCallOutput { .. }
            | ResponseItem::ToolSearchOutput { .. }
            | ResponseItem::CustomToolCallOutput { .. }
    ) || matches!(item, ResponseItem::Message { role, .. } if role == "developer")
}

fn rewritten_output_for_context_window(item: &ResponseItem) -> Option<ResponseItem> {
    Some(match item {
        ResponseItem::FunctionCallOutput {
            id,
            call_id,
            output,
            internal_chat_message_metadata_passthrough: metadata,
        } => ResponseItem::FunctionCallOutput {
            id: id.clone(),
            call_id: call_id.clone(),
            output: truncated_output_payload(output),
            internal_chat_message_metadata_passthrough: metadata.clone(),
        },
        ResponseItem::CustomToolCallOutput {
            id,
            call_id,
            name,
            output,
            internal_chat_message_metadata_passthrough: metadata,
        } => ResponseItem::CustomToolCallOutput {
            id: id.clone(),
            call_id: call_id.clone(),
            name: name.clone(),
            output: truncated_output_payload(output),
            internal_chat_message_metadata_passthrough: metadata.clone(),
        },
        ResponseItem::ToolSearchOutput {
            call_id,
            status,
            execution,
            internal_chat_message_metadata_passthrough: metadata,
            ..
        } => ResponseItem::ToolSearchOutput {
            id: item.id().map(str::to_string),
            call_id: call_id.clone(),
            status: status.clone(),
            execution: execution.clone(),
            tools: Vec::new(),
            internal_chat_message_metadata_passthrough: metadata.clone(),
        },
        _ => return None,
    })
}

fn truncated_output_payload(output: &FunctionCallOutputPayload) -> FunctionCallOutputPayload {
    FunctionCallOutputPayload {
        body: FunctionCallOutputBody::Text(CONTEXT_WINDOW_TRUNCATED_OUTPUT_MESSAGE.to_string()),
        success: output.success,
    }
}

fn remote_compaction_input_token_budget(context_window: i64) -> i64 {
    let reserve = (context_window / 20).clamp(512, 8_000);
    context_window
        .saturating_sub(reserve)
        .max(context_window / 2)
}

fn reduce_remote_compaction_prompt_input(
    input: &mut Vec<ResponseItem>,
    turn_context: &TurnContext,
) -> Option<PromptReductionStats> {
    let config = crate::session::turn::prompt_reduction::reduction_config_for_turn(turn_context)?;
    match codex_prompt_reducer::reduce_prompt_items(input, &config) {
        Ok(stats) if stats.reductions > 0 => Some(stats),
        Ok(_) => None,
        Err(error) => {
            warn!(
                turn_id = %turn_context.sub_id,
                %error,
                "failed to reduce remote compaction prompt input"
            );
            None
        }
    }
}

fn estimate_remote_compaction_prompt_tokens(
    input: &[ResponseItem],
    base_instructions: &BaseInstructions,
) -> i64 {
    let base_tokens =
        i64::try_from(approx_token_count(&base_instructions.text)).unwrap_or(i64::MAX);
    input
        .iter()
        .map(estimate_response_item_model_visible_bytes)
        .map(|bytes| bytes.saturating_add(3) / 4)
        .fold(base_tokens, i64::saturating_add)
}

// Upstream PR #27106 made `context_manager::estimate_response_item_model_visible_bytes`
// private to the context manager. The fork still needs an approximate per-item
// model-visible byte count for remote compaction budget estimates, so estimate from the
// serialized item length here. This matches the upstream estimator's default arm for the
// text items remote compaction sees; image/encrypted payloads are not discounted (a
// conservative over-estimate that only tightens the token budget).
fn estimate_response_item_model_visible_bytes(item: &ResponseItem) -> i64 {
    serde_json::to_string(item)
        .map(|serialized| i64::try_from(serialized.len()).unwrap_or(i64::MAX))
        .unwrap_or_default()
}

pub(crate) fn log_remote_compaction_prompt_fit(
    turn_context: &TurnContext,
    implementation: &'static str,
    attempt_index: usize,
    token_budget: i64,
    prepared: &PreparedRemoteCompactionPrompt,
) {
    let Some(stats) = prepared.reduction_stats.as_ref() else {
        if prepared.deleted_items > 0 || attempt_index > 0 {
            info!(
                turn_id = %turn_context.sub_id,
                implementation,
                attempt = attempt_index + 1,
                token_budget,
                estimated_tokens = prepared.estimated_tokens,
                deleted_items = prepared.deleted_items,
                "fit remote compaction prompt input"
            );
        }
        return;
    };

    info!(
        turn_id = %turn_context.sub_id,
        implementation,
        attempt = attempt_index + 1,
        token_budget,
        estimated_tokens = prepared.estimated_tokens,
        deleted_items = prepared.deleted_items,
        reductions = stats.reductions,
        saved_tokens = stats.saved_tokens,
        artifacts = stats.artifacts,
        "reduced and fit remote compaction prompt input"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::models::ContentItem;
    use codex_utils_output_truncation::TruncationPolicy;

    fn message(role: &str, text: impl Into<String>) -> ResponseItem {
        ResponseItem::Message {
            id: None,
            role: role.to_string(),
            content: vec![ContentItem::InputText { text: text.into() }],
            phase: None,
        }
    }

    #[test]
    fn remote_compaction_trim_drops_oldest_items_when_generated_tail_is_not_enough() {
        let old_large = message("user", "old context ".repeat(400));
        let latest_user = message("user", "latest request");
        let mut history = ContextManager::default();
        history.record_items(
            [&old_large, &latest_user],
            TruncationPolicy::Bytes(usize::MAX),
        );

        let deleted_items = trim_remote_compaction_history_to_token_budget(
            &mut history,
            &BaseInstructions {
                text: String::new(),
            },
            100,
        );

        assert_eq!(deleted_items, 1);
        assert_eq!(history.raw_items(), &[latest_user]);
    }
}
