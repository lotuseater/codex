use codex_analytics::CompactionReason;
use codex_context_reduction::ContextReductionDecision;
use codex_context_reduction::ContextReductionInput;
use codex_context_reduction::ContextReductionPolicy;
use codex_context_reduction::ContextReductionState;
use codex_protocol::protocol::TokenUsage;

const MIN_CONTINUATION_TURNS: u32 = 8;
const MIN_SEMANTIC_TOKENS: i64 = 80_000;
const WORK_CHECKPOINT_TURNS: u32 = 6;
const WORK_CHECKPOINT_TOKENS: i64 = 32_000;
const WORK_CHECKPOINT_MIN_TOTAL_TOKENS: i64 = 50_000;
const COMMIT_CHECKPOINT_MIN_TOTAL_TOKENS: i64 = 20_000;
const TOOL_CHECKPOINT_CALLS: u64 = 12;
const TOOL_CHECKPOINT_MIN_TOTAL_TOKENS: i64 = 40_000;
const SEMANTIC_COOLDOWN_TURNS: u32 = 4;
const EARLY_PRESSURE_NUMERATOR: i64 = 4;
const EARLY_PRESSURE_DENOMINATOR: i64 = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SemanticCompactDecision {
    Skip,
    Compact { reason: CompactionReason },
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SemanticCompactInput {
    pub(crate) semantic_feature_enabled: bool,
    pub(crate) context_reduction_enabled: bool,
    pub(crate) total_usage_tokens: i64,
    pub(crate) auto_compact_limit: i64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SemanticCompactTurnInput<'a> {
    pub(crate) token_usage: &'a TokenUsage,
    pub(crate) tool_calls: u64,
    pub(crate) git_commit_observed: bool,
    pub(crate) is_continuation_turn: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SemanticCompactState {
    regular_turns_since_last_compact: u32,
    continuation_turns_since_last_compact: u32,
    work_tokens_since_last_compact: i64,
    tool_calls_since_last_compact: u64,
    git_commit_observed_since_last_compact: bool,
    semantic_cooldown_turns_remaining: u32,
    context_reduction: ContextReductionState,
}

impl SemanticCompactState {
    pub(crate) fn record_regular_turn_finished(&mut self, input: SemanticCompactTurnInput<'_>) {
        self.regular_turns_since_last_compact =
            self.regular_turns_since_last_compact.saturating_add(1);
        if input.is_continuation_turn {
            self.continuation_turns_since_last_compact =
                self.continuation_turns_since_last_compact.saturating_add(1);
        }
        self.work_tokens_since_last_compact = self
            .work_tokens_since_last_compact
            .saturating_add(turn_work_tokens(input.token_usage));
        self.tool_calls_since_last_compact = self
            .tool_calls_since_last_compact
            .saturating_add(input.tool_calls);
        self.git_commit_observed_since_last_compact |= input.git_commit_observed;
        self.semantic_cooldown_turns_remaining =
            self.semantic_cooldown_turns_remaining.saturating_sub(1);
        self.context_reduction.record_regular_turn_finished();
    }

    pub(crate) fn record_compaction_finished(&mut self, reason: Option<CompactionReason>) {
        self.regular_turns_since_last_compact = 0;
        self.continuation_turns_since_last_compact = 0;
        self.work_tokens_since_last_compact = 0;
        self.tool_calls_since_last_compact = 0;
        self.git_commit_observed_since_last_compact = false;
        self.semantic_cooldown_turns_remaining = SEMANTIC_COOLDOWN_TURNS;
        if reason == Some(CompactionReason::EarlyContextPressure) {
            self.context_reduction
                .record_reduction_finished(ContextReductionPolicy::default());
        }
    }

    pub(crate) fn decide(&self, input: SemanticCompactInput) -> SemanticCompactDecision {
        if input.auto_compact_limit <= 0 || input.total_usage_tokens >= input.auto_compact_limit {
            return SemanticCompactDecision::Skip;
        }

        if input.context_reduction_enabled
            && self.context_reduction.decide(
                ContextReductionPolicy::default(),
                ContextReductionInput {
                    total_usage_tokens: input.total_usage_tokens,
                    auto_compact_limit: input.auto_compact_limit,
                },
            ) != ContextReductionDecision::Skip
        {
            return SemanticCompactDecision::Compact {
                reason: CompactionReason::EarlyContextPressure,
            };
        }

        if !input.semantic_feature_enabled || self.semantic_cooldown_turns_remaining > 0 {
            return SemanticCompactDecision::Skip;
        }

        if input.total_usage_tokens >= early_pressure_threshold(input.auto_compact_limit)
            || (self.continuation_turns_since_last_compact >= MIN_CONTINUATION_TURNS
                && input.total_usage_tokens >= MIN_SEMANTIC_TOKENS)
            || (self.regular_turns_since_last_compact >= WORK_CHECKPOINT_TURNS
                && self.work_tokens_since_last_compact >= WORK_CHECKPOINT_TOKENS
                && input.total_usage_tokens >= WORK_CHECKPOINT_MIN_TOTAL_TOKENS)
            || (self.git_commit_observed_since_last_compact
                && input.total_usage_tokens >= COMMIT_CHECKPOINT_MIN_TOTAL_TOKENS)
            || (self.tool_calls_since_last_compact >= TOOL_CHECKPOINT_CALLS
                && input.total_usage_tokens >= TOOL_CHECKPOINT_MIN_TOTAL_TOKENS)
        {
            SemanticCompactDecision::Compact {
                reason: CompactionReason::SemanticCheckpoint,
            }
        } else {
            SemanticCompactDecision::Skip
        }
    }
}

fn turn_work_tokens(turn_token_usage: &TokenUsage) -> i64 {
    turn_token_usage
        .non_cached_input()
        .saturating_add(turn_token_usage.output_tokens.max(0))
}

fn early_pressure_threshold(auto_compact_limit: i64) -> i64 {
    auto_compact_limit.saturating_mul(EARLY_PRESSURE_NUMERATOR) / EARLY_PRESSURE_DENOMINATOR
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn input(total_usage_tokens: i64) -> SemanticCompactInput {
        SemanticCompactInput {
            semantic_feature_enabled: true,
            context_reduction_enabled: true,
            total_usage_tokens,
            auto_compact_limit: 100_000,
        }
    }

    fn semantic_only_input(total_usage_tokens: i64) -> SemanticCompactInput {
        SemanticCompactInput {
            semantic_feature_enabled: true,
            context_reduction_enabled: false,
            total_usage_tokens,
            auto_compact_limit: 100_000,
        }
    }

    fn reduction_only_input(total_usage_tokens: i64) -> SemanticCompactInput {
        SemanticCompactInput {
            semantic_feature_enabled: false,
            context_reduction_enabled: true,
            total_usage_tokens,
            auto_compact_limit: 100_000,
        }
    }

    fn finished_turn(
        token_usage: &TokenUsage,
        tool_calls: u64,
        git_commit_observed: bool,
    ) -> SemanticCompactTurnInput<'_> {
        SemanticCompactTurnInput {
            token_usage,
            tool_calls,
            git_commit_observed,
            is_continuation_turn: false,
        }
    }

    fn finished_continuation_turn(
        token_usage: &TokenUsage,
        tool_calls: u64,
        git_commit_observed: bool,
    ) -> SemanticCompactTurnInput<'_> {
        SemanticCompactTurnInput {
            token_usage,
            tool_calls,
            git_commit_observed,
            is_continuation_turn: true,
        }
    }

    fn token_usage(input_tokens: i64, cached_input_tokens: i64, output_tokens: i64) -> TokenUsage {
        TokenUsage {
            input_tokens,
            cached_input_tokens,
            output_tokens,
            ..Default::default()
        }
    }

    #[test]
    fn early_context_pressure_triggers_at_twenty_percent_without_semantic_feature() {
        let state = SemanticCompactState::default();

        assert_eq!(
            state.decide(reduction_only_input(19_999)),
            SemanticCompactDecision::Skip
        );
        assert_eq!(
            state.decide(reduction_only_input(20_000)),
            SemanticCompactDecision::Compact {
                reason: CompactionReason::EarlyContextPressure,
            }
        );
    }

    #[test]
    fn early_context_pressure_uses_twenty_four_regular_turn_cooldown_after_compaction() {
        let mut state = SemanticCompactState::default();
        state.record_compaction_finished(Some(CompactionReason::EarlyContextPressure));

        for _ in 0..23 {
            state.record_regular_turn_finished(finished_turn(&TokenUsage::default(), 0, false));
            assert_eq!(
                state.decide(reduction_only_input(20_000)),
                SemanticCompactDecision::Skip
            );
        }

        state.record_regular_turn_finished(finished_turn(&TokenUsage::default(), 0, false));
        assert_eq!(
            state.decide(reduction_only_input(20_000)),
            SemanticCompactDecision::Compact {
                reason: CompactionReason::EarlyContextPressure,
            }
        );
    }

    #[test]
    fn semantic_early_pressure_checkpoint_still_works() {
        let state = SemanticCompactState::default();

        assert_eq!(
            state.decide(semantic_only_input(79_999)),
            SemanticCompactDecision::Skip
        );
        assert_eq!(
            state.decide(semantic_only_input(80_000)),
            SemanticCompactDecision::Compact {
                reason: CompactionReason::SemanticCheckpoint,
            }
        );
    }

    #[test]
    fn other_compactions_do_not_start_early_context_pressure_cooldown() {
        let mut state = SemanticCompactState::default();
        state.record_compaction_finished(Some(CompactionReason::ContextLimit));

        assert_eq!(
            state.decide(reduction_only_input(20_000)),
            SemanticCompactDecision::Compact {
                reason: CompactionReason::EarlyContextPressure,
            }
        );
    }

    #[test]
    fn semantic_continuation_checkpoint_still_works() {
        let mut state = SemanticCompactState::default();
        for _ in 0..MIN_CONTINUATION_TURNS {
            state.record_regular_turn_finished(finished_continuation_turn(
                &TokenUsage::default(),
                0,
                false,
            ));
        }

        assert_eq!(
            state.decide(semantic_only_input(MIN_SEMANTIC_TOKENS)),
            SemanticCompactDecision::Compact {
                reason: CompactionReason::SemanticCheckpoint,
            }
        );
    }

    #[test]
    fn semantic_work_checkpoint_still_works() {
        let mut state = SemanticCompactState::default();
        let usage = token_usage(6_000, 1_000, 1_000);
        for _ in 0..WORK_CHECKPOINT_TURNS {
            state.record_regular_turn_finished(finished_turn(&usage, 0, false));
        }

        assert_eq!(
            state.decide(semantic_only_input(WORK_CHECKPOINT_MIN_TOTAL_TOKENS)),
            SemanticCompactDecision::Compact {
                reason: CompactionReason::SemanticCheckpoint,
            }
        );
    }

    #[test]
    fn semantic_tool_checkpoint_still_works() {
        let mut state = SemanticCompactState::default();
        state.record_regular_turn_finished(finished_turn(
            &TokenUsage::default(),
            TOOL_CHECKPOINT_CALLS,
            false,
        ));

        assert_eq!(
            state.decide(semantic_only_input(TOOL_CHECKPOINT_MIN_TOTAL_TOKENS)),
            SemanticCompactDecision::Compact {
                reason: CompactionReason::SemanticCheckpoint,
            }
        );
    }

    #[test]
    fn semantic_git_commit_checkpoint_still_works() {
        let mut state = SemanticCompactState::default();
        state.record_regular_turn_finished(finished_turn(&TokenUsage::default(), 0, true));

        assert_eq!(
            state.decide(semantic_only_input(COMMIT_CHECKPOINT_MIN_TOTAL_TOKENS)),
            SemanticCompactDecision::Compact {
                reason: CompactionReason::SemanticCheckpoint,
            }
        );
    }

    #[test]
    fn early_context_pressure_takes_precedence_over_semantic_checkpoint() {
        let state = SemanticCompactState::default();

        assert_eq!(
            state.decide(input(80_000)),
            SemanticCompactDecision::Compact {
                reason: CompactionReason::EarlyContextPressure,
            }
        );
    }

    #[test]
    fn semantic_checkpoint_keeps_short_cooldown_after_compaction() {
        let mut state = SemanticCompactState::default();
        state.record_compaction_finished(Some(CompactionReason::SemanticCheckpoint));

        for _ in 0..SEMANTIC_COOLDOWN_TURNS - 1 {
            state.record_regular_turn_finished(finished_turn(&TokenUsage::default(), 0, false));
            assert_eq!(
                state.decide(semantic_only_input(80_000)),
                SemanticCompactDecision::Skip
            );
        }

        state.record_regular_turn_finished(finished_turn(&TokenUsage::default(), 0, false));
        assert_eq!(
            state.decide(semantic_only_input(80_000)),
            SemanticCompactDecision::Compact {
                reason: CompactionReason::SemanticCheckpoint,
            }
        );
    }
}
