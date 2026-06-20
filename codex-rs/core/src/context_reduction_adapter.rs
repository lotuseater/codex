use crate::session::turn_context::TurnContext;
use codex_analytics::CompactionReason;
use codex_compaction_policy as compaction_domain;
use codex_context_reduction as context_reduction;
use codex_features::Feature;
use codex_protocol::config_types::ContextBudgetMode;
use codex_protocol::config_types::ModeKind;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::protocol::TokenUsage;

pub(crate) fn context_reduction_reason_to_compaction_reason(
    reason: context_reduction::ContextReductionReason,
) -> CompactionReason {
    let reason = match reason {
        context_reduction::ContextReductionReason::ContextLimit => {
            compaction_domain::ContextReductionReason::ContextLimit
        }
        context_reduction::ContextReductionReason::SemanticCheckpoint => {
            compaction_domain::ContextReductionReason::SemanticCheckpoint
        }
        context_reduction::ContextReductionReason::EarlyContextPressure => {
            compaction_domain::ContextReductionReason::EarlyContextPressure
        }
    };
    match reason {
        compaction_domain::ContextReductionReason::ContextLimit => CompactionReason::ContextLimit,
        compaction_domain::ContextReductionReason::SemanticCheckpoint => {
            CompactionReason::SemanticCheckpoint
        }
        compaction_domain::ContextReductionReason::EarlyContextPressure => {
            CompactionReason::EarlyContextPressure
        }
    }
}

pub(crate) fn compaction_reason_to_context_reduction_reason(
    reason: CompactionReason,
) -> Option<context_reduction::ContextReductionReason> {
    let reason = match reason {
        CompactionReason::ContextLimit => compaction_domain::ContextReductionReason::ContextLimit,
        CompactionReason::SemanticCheckpoint => {
            compaction_domain::ContextReductionReason::SemanticCheckpoint
        }
        CompactionReason::EarlyContextPressure => {
            compaction_domain::ContextReductionReason::EarlyContextPressure
        }
        CompactionReason::UserRequested
        | CompactionReason::ModelDownshift
        | CompactionReason::RestoredSession => return None,
    };
    Some(match reason {
        compaction_domain::ContextReductionReason::ContextLimit => {
            context_reduction::ContextReductionReason::ContextLimit
        }
        compaction_domain::ContextReductionReason::SemanticCheckpoint => {
            context_reduction::ContextReductionReason::SemanticCheckpoint
        }
        compaction_domain::ContextReductionReason::EarlyContextPressure => {
            context_reduction::ContextReductionReason::EarlyContextPressure
        }
    })
}

pub(crate) fn auto_compact_budget_mode(
    mode: ContextBudgetMode,
) -> context_reduction::AutoCompactBudgetMode {
    let mode = match mode {
        ContextBudgetMode::Slow => compaction_domain::AutoCompactBudgetMode::Slow,
        ContextBudgetMode::Standard => compaction_domain::AutoCompactBudgetMode::Standard,
    };
    match mode {
        compaction_domain::AutoCompactBudgetMode::Standard => {
            context_reduction::AutoCompactBudgetMode::Standard
        }
        compaction_domain::AutoCompactBudgetMode::Slow => {
            context_reduction::AutoCompactBudgetMode::Slow
        }
    }
}

pub(crate) fn model_auto_compact_limits(
    model_info: &ModelInfo,
) -> context_reduction::ModelAutoCompactLimits {
    let limits = compaction_domain::ModelAutoCompactLimits {
        auto_compact_token_limit: model_info.auto_compact_token_limit(),
        context_window: model_info.context_window,
    };
    context_reduction::ModelAutoCompactLimits {
        auto_compact_token_limit: limits.auto_compact_token_limit,
        context_window: limits.context_window,
    }
}

pub(crate) fn semantic_auto_compact_enabled(turn_context: &TurnContext) -> bool {
    turn_context.features.enabled(Feature::SemanticAutoCompact)
        && turn_context.collaboration_mode.mode != ModeKind::Plan
}

fn domain_context_reduction_policy(
    trigger_percent: u8,
) -> compaction_domain::ContextReductionPolicy {
    compaction_domain::ContextReductionPolicy::new(
        trigger_percent,
        compaction_domain::DEFAULT_TURN_COOLDOWN,
    )
}

pub(crate) fn context_reduction_policy(
    trigger_percent: u8,
) -> context_reduction::ContextReductionPolicy {
    context_reduction_policy_from_domain(domain_context_reduction_policy(trigger_percent))
}

pub(crate) fn semantic_compact_input(
    turn_context: &TurnContext,
    total_usage_tokens: i64,
    auto_compact_limit: i64,
    visible_context_percent_used: Option<i64>,
) -> context_reduction::SemanticCompactInput {
    let trigger_percent = turn_context.config.model_compact_percentage;
    let input = compaction_domain::SemanticCompactInput {
        enabled: semantic_auto_compact_enabled(turn_context),
        policy: domain_context_reduction_policy(trigger_percent),
        total_usage_tokens,
        auto_compact_limit,
        context_window: turn_context.model_context_window(),
        visible_context_percent_used,
    };
    context_reduction::SemanticCompactInput {
        enabled: input.enabled,
        policy: context_reduction_policy_from_domain(input.policy),
        total_usage_tokens: input.total_usage_tokens,
        auto_compact_limit: input.auto_compact_limit,
        context_window: input.context_window,
        visible_context_percent_used: input.visible_context_percent_used,
    }
}

fn context_reduction_policy_from_domain(
    policy: compaction_domain::ContextReductionPolicy,
) -> context_reduction::ContextReductionPolicy {
    context_reduction::ContextReductionPolicy::new(
        policy.trigger_context_percent(),
        policy.turn_cooldown(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_compaction_policy::DEFAULT_TRIGGER_CONTEXT_PERCENT;
    use codex_compaction_policy::DEFAULT_TURN_COOLDOWN;

    #[test]
    fn context_reduction_policy_is_fixed_twenty_percent_with_twenty_four_turn_cooldown() {
        let policy = domain_context_reduction_policy(DEFAULT_TRIGGER_CONTEXT_PERCENT);

        assert_eq!(
            policy.trigger_context_percent(),
            DEFAULT_TRIGGER_CONTEXT_PERCENT
        );
        assert_eq!(policy.turn_cooldown(), DEFAULT_TURN_COOLDOWN);
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
    let usage = TokenUsage {
        total_tokens: context_tokens.max(0),
        ..TokenUsage::default()
    };
    Some((100 - usage.percent_of_context_window_remaining(model_context_window)).clamp(0, 100))
}

pub(crate) fn semantic_compact_turn_input(
    token_usage: &TokenUsage,
    tool_calls: u64,
    git_commit_observed: bool,
    is_continuation_turn: bool,
) -> context_reduction::SemanticCompactTurnInput {
    let input = compaction_domain::SemanticCompactTurnInput {
        non_cached_input_tokens: token_usage.non_cached_input(),
        output_tokens: token_usage.output_tokens,
        tool_calls,
        git_commit_observed,
        is_continuation_turn,
    };
    context_reduction::SemanticCompactTurnInput {
        non_cached_input_tokens: input.non_cached_input_tokens,
        output_tokens: input.output_tokens,
        tool_calls: input.tool_calls,
        git_commit_observed: input.git_commit_observed,
        is_continuation_turn: input.is_continuation_turn,
    }
}
