//! Fork-owned orchestration around the context-budget / auto-compact adapters.
//!
//! Upstream heavily rewrites `session/turn.rs` and `tasks/mod.rs` (their import
//! blocks and compaction logic in particular). To keep the fork's footprint in
//! those upstream-hot files small, the fork's context-budget call-site
//! orchestration lives here instead of inline. `turn.rs` and `tasks/mod.rs`
//! call into the functions below rather than holding the adapter imports plus
//! the inline decision/run logic themselves.
//!
//! This module only moves existing call-site logic; it does not change behavior.

use super::*;

use crate::compact::InitialContextInjection;
use crate::compact::is_compaction_max_output_tokens;
use crate::context_reduction_adapter::auto_compact_budget_mode;
use crate::context_reduction_adapter::context_reduction_reason_to_compaction_reason;
use crate::context_reduction_adapter::model_auto_compact_limits;
use crate::context_reduction_adapter::semantic_compact_input;
use crate::session::turn::run_auto_compact;
use codex_analytics::CompactionPhase;
use codex_context_reduction::AutoCompactTokenLimitInput;
use codex_context_reduction::ContextReductionReason;
use codex_context_reduction::PostSamplingAutoCompactAction;
use codex_context_reduction::PostSamplingAutoCompactInput;
use codex_context_reduction::SemanticCompactDecision;
use codex_context_reduction::auto_compact_token_limit_for_mode;
use codex_context_reduction::post_sampling_auto_compact_action;

/// Resolve the effective auto-compact token limit for the active turn.
///
/// Combines the model's configured limits, the runtime context window, and the
/// fork's context-budget mode through the context-reduction adapter.
pub(crate) fn auto_compact_token_limit(turn_context: &TurnContext) -> i64 {
    auto_compact_token_limit_for_mode(AutoCompactTokenLimitInput {
        model_limits: model_auto_compact_limits(&turn_context.model_info),
        runtime_context_window: turn_context.model_context_window(),
        budget_mode: auto_compact_budget_mode(turn_context.config.context_budget_mode),
    })
}

/// Outcome of the post-sampling auto-compact decision computed after a model
/// response. Mirrors the fields the turn loop previously computed inline.
pub(crate) struct PostSamplingCompactionDecision {
    /// Total active-context token usage observed after sampling.
    pub(crate) total_usage_tokens: i64,
    /// Whether the auto-compact token limit was reached.
    pub(crate) token_limit_reached: bool,
    /// Whether the early-context-pressure semantic threshold was reached.
    pub(crate) early_context_pressure_reached: bool,
    /// The raw post-sampling auto-compact action, if any.
    pub(crate) auto_compact_action: Option<PostSamplingAutoCompactAction>,
    /// The before-follow-up compaction reason, if a mid-turn compaction is due.
    pub(crate) compaction_reason: Option<CompactionReason>,
}

/// Compute the post-sampling auto-compact decision for the current turn.
///
/// This is the fork's context-budget decision logic lifted out of the turn loop
/// body; it performs the same token-usage reads and semantic-compact decision in
/// the same order, then returns the derived fields for the caller to act on.
pub(crate) async fn post_sampling_compaction_decision(
    sess: &Session,
    turn_context: &TurnContext,
    auto_compact_limit: i64,
    needs_follow_up: bool,
) -> PostSamplingCompactionDecision {
    let total_usage_tokens = sess.get_total_token_usage().await;
    let visible_context_percent_used = sess.visible_context_percent_used().await;
    let token_limit_reached = total_usage_tokens >= auto_compact_limit;
    let semantic_compact_decision = if token_limit_reached {
        SemanticCompactDecision::Skip
    } else {
        sess.semantic_compact_decision(semantic_compact_input(
            turn_context,
            total_usage_tokens,
            auto_compact_limit,
            visible_context_percent_used,
        ))
        .await
    };
    let early_context_pressure_reached = matches!(
        semantic_compact_decision,
        SemanticCompactDecision::Compact {
            reason: ContextReductionReason::EarlyContextPressure
        }
    );
    let auto_compact_action = post_sampling_auto_compact_action(PostSamplingAutoCompactInput {
        needs_follow_up,
        total_usage_tokens,
        auto_compact_limit,
        semantic_compact_decision,
    });
    let compaction_reason = match auto_compact_action {
        Some(PostSamplingAutoCompactAction::BeforeFollowUp(reason)) => {
            Some(context_reduction_reason_to_compaction_reason(reason))
        }
        Some(PostSamplingAutoCompactAction::AfterFinalResponse(_)) | None => None,
    };

    PostSamplingCompactionDecision {
        total_usage_tokens,
        token_limit_reached,
        early_context_pressure_reached,
        auto_compact_action,
        compaction_reason,
    }
}

impl Session {
    /// Run the fork's post-turn semantic / context-limit auto-compaction.
    ///
    /// Moved verbatim from `tasks/mod.rs` so the post-turn compaction
    /// orchestration (which is fork-only and absent upstream) no longer lives in
    /// that upstream-hot file. Behavior is unchanged.
    pub(crate) async fn maybe_run_post_turn_semantic_compact(
        self: &Arc<Self>,
        turn_context: &Arc<TurnContext>,
    ) -> anyhow::Result<()> {
        let total_usage_tokens = self.get_total_token_usage().await;
        let auto_compact_limit = auto_compact_token_limit(turn_context);
        let visible_context_percent_used = self.visible_context_percent_used().await;
        let reason = if total_usage_tokens >= auto_compact_limit {
            if self
                .is_post_turn_compact_max_output_suppressed(total_usage_tokens, auto_compact_limit)
                .await
            {
                return Ok(());
            }
            Some(CompactionReason::ContextLimit)
        } else {
            match self
                .semantic_compact_decision(semantic_compact_input(
                    turn_context,
                    total_usage_tokens,
                    auto_compact_limit,
                    visible_context_percent_used,
                ))
                .await
            {
                SemanticCompactDecision::Compact { reason } => {
                    Some(context_reduction_reason_to_compaction_reason(reason))
                }
                SemanticCompactDecision::Skip => None,
            }
        };

        let Some(reason) = reason else {
            return Ok(());
        };
        if self.has_pending_input().await {
            return Ok(());
        }
        let mut client_session = self.services.model_client.new_session();
        let compact_result = if reason == CompactionReason::SemanticCheckpoint {
            let git_outcome = self
                .semantic_checkpoint_git_sync(turn_context, reason)
                .await;
            if git_outcome.should_warn() {
                self.send_event(
                    turn_context.as_ref(),
                    EventMsg::Warning(WarningEvent {
                        message: git_outcome.summary(),
                    }),
                )
                .await;
            }
            let scratchpad = self
                .write_semantic_compact_scratchpad(turn_context, reason, &git_outcome.summary())
                .await;
            let compact_result = run_auto_compact(
                self,
                turn_context,
                &mut client_session,
                InitialContextInjection::DoNotInject,
                reason,
                CompactionPhase::PostTurn,
            )
            .await;
            self.cleanup_semantic_compact_scratchpad(scratchpad);
            compact_result
        } else {
            run_auto_compact(
                self,
                turn_context,
                &mut client_session,
                InitialContextInjection::DoNotInject,
                reason,
                CompactionPhase::PostTurn,
            )
            .await
        };
        if reason == CompactionReason::ContextLimit
            && let Err(err) = &compact_result
            && is_compaction_max_output_tokens(err)
        {
            self.record_post_turn_compact_max_output_suppression(
                total_usage_tokens,
                auto_compact_limit,
            )
            .await;
        }
        compact_result?;
        Ok(())
    }
}
