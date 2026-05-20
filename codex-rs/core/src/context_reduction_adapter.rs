use crate::session::turn_context::TurnContext;
use codex_analytics::CompactionReason;
use codex_context_reduction::AutoCompactBudgetMode;
use codex_context_reduction::ContextReductionReason;
use codex_context_reduction::ModelAutoCompactLimits;
use codex_context_reduction::SemanticCompactInput;
use codex_context_reduction::SemanticCompactTurnInput;
use codex_features::Feature;
use codex_protocol::config_types::ContextBudgetMode;
use codex_protocol::config_types::ModeKind;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::protocol::TokenUsage;

pub(crate) fn context_reduction_reason_to_compaction_reason(
    reason: ContextReductionReason,
) -> CompactionReason {
    match reason {
        ContextReductionReason::ContextLimit => CompactionReason::ContextLimit,
        ContextReductionReason::SemanticCheckpoint => CompactionReason::SemanticCheckpoint,
        ContextReductionReason::EarlyContextPressure => CompactionReason::EarlyContextPressure,
    }
}

pub(crate) fn compaction_reason_to_context_reduction_reason(
    reason: CompactionReason,
) -> Option<ContextReductionReason> {
    match reason {
        CompactionReason::ContextLimit => Some(ContextReductionReason::ContextLimit),
        CompactionReason::SemanticCheckpoint => Some(ContextReductionReason::SemanticCheckpoint),
        CompactionReason::EarlyContextPressure => {
            Some(ContextReductionReason::EarlyContextPressure)
        }
        CompactionReason::UserRequested
        | CompactionReason::ModelDownshift
        | CompactionReason::RestoredSession => None,
    }
}

pub(crate) fn auto_compact_budget_mode(mode: ContextBudgetMode) -> AutoCompactBudgetMode {
    match mode {
        ContextBudgetMode::Slow => AutoCompactBudgetMode::Slow,
        ContextBudgetMode::Standard => AutoCompactBudgetMode::Standard,
    }
}

pub(crate) fn model_auto_compact_limits(model_info: &ModelInfo) -> ModelAutoCompactLimits {
    ModelAutoCompactLimits {
        auto_compact_token_limit: model_info.auto_compact_token_limit(),
        context_window: model_info.context_window,
    }
}

pub(crate) fn semantic_auto_compact_enabled(turn_context: &TurnContext) -> bool {
    turn_context.features.enabled(Feature::SemanticAutoCompact)
        && turn_context.collaboration_mode.mode != ModeKind::Plan
}

pub(crate) fn semantic_compact_input(
    turn_context: &TurnContext,
    total_usage_tokens: i64,
    auto_compact_limit: i64,
    visible_context_percent_used: Option<i64>,
) -> SemanticCompactInput {
    SemanticCompactInput {
        enabled: semantic_auto_compact_enabled(turn_context),
        total_usage_tokens,
        auto_compact_limit,
        visible_context_percent_used,
    }
}

pub(crate) fn token_context_percent_used(
    context_tokens: i64,
    model_context_window: Option<i64>,
) -> Option<i64> {
    let model_context_window = model_context_window?;
    if model_context_window <= 0 {
        return None;
    }
    Some(
        ((context_tokens.max(0) as f64 / model_context_window as f64) * 100.0)
            .clamp(0.0, 100.0)
            .round() as i64,
    )
}

pub(crate) fn semantic_compact_turn_input(
    token_usage: &TokenUsage,
    tool_calls: u64,
    git_commit_observed: bool,
    is_continuation_turn: bool,
) -> SemanticCompactTurnInput {
    SemanticCompactTurnInput {
        non_cached_input_tokens: token_usage.non_cached_input(),
        output_tokens: token_usage.output_tokens,
        tool_calls,
        git_commit_observed,
        is_continuation_turn,
    }
}
