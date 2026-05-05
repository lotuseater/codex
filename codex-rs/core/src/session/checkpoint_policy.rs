use codex_analytics::CompactionReason;
use codex_protocol::protocol::TokenUsage;

const MIN_CONTINUATION_TURNS: u32 = 8;
const MIN_SEMANTIC_TOKENS: i64 = 80_000;
const WORK_CHECKPOINT_TURNS: u32 = 6;
const WORK_CHECKPOINT_TOKENS: i64 = 32_000;
const WORK_CHECKPOINT_MIN_TOTAL_TOKENS: i64 = 50_000;
const COOLDOWN_TURNS: u32 = 4;
const EARLY_PRESSURE_NUMERATOR: i64 = 4;
const EARLY_PRESSURE_DENOMINATOR: i64 = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SemanticCompactDecision {
    Skip,
    Compact { reason: CompactionReason },
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SemanticCompactInput {
    pub(crate) feature_enabled: bool,
    pub(crate) total_usage_tokens: i64,
    pub(crate) auto_compact_limit: i64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SemanticCompactState {
    regular_turns_since_last_compact: u32,
    work_tokens_since_last_compact: i64,
    cooldown_turns_remaining: u32,
}

impl SemanticCompactState {
    pub(crate) fn record_regular_turn_finished(&mut self, turn_token_usage: &TokenUsage) {
        self.regular_turns_since_last_compact =
            self.regular_turns_since_last_compact.saturating_add(1);
        self.work_tokens_since_last_compact = self
            .work_tokens_since_last_compact
            .saturating_add(turn_work_tokens(turn_token_usage));
        self.cooldown_turns_remaining = self.cooldown_turns_remaining.saturating_sub(1);
    }

    pub(crate) fn record_compaction_finished(&mut self) {
        self.regular_turns_since_last_compact = 0;
        self.work_tokens_since_last_compact = 0;
        self.cooldown_turns_remaining = COOLDOWN_TURNS;
    }

    pub(crate) fn decide(&self, input: SemanticCompactInput) -> SemanticCompactDecision {
        if !input.feature_enabled
            || input.auto_compact_limit <= 0
            || input.total_usage_tokens >= input.auto_compact_limit
            || self.cooldown_turns_remaining > 0
        {
            return SemanticCompactDecision::Skip;
        }

        if input.total_usage_tokens >= early_pressure_threshold(input.auto_compact_limit)
            || (self.regular_turns_since_last_compact >= MIN_CONTINUATION_TURNS
                && input.total_usage_tokens >= MIN_SEMANTIC_TOKENS)
            || (self.regular_turns_since_last_compact >= WORK_CHECKPOINT_TURNS
                && self.work_tokens_since_last_compact >= WORK_CHECKPOINT_TOKENS
                && input.total_usage_tokens >= WORK_CHECKPOINT_MIN_TOTAL_TOKENS)
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

    fn input(total_usage_tokens: i64) -> SemanticCompactInput {
        SemanticCompactInput {
            feature_enabled: true,
            total_usage_tokens,
            auto_compact_limit: 100_000,
        }
    }

    #[test]
    fn semantic_compact_decision_is_feature_gated() {
        let mut state = SemanticCompactState::default();
        for _ in 0..MIN_CONTINUATION_TURNS {
            state.record_regular_turn_finished(&TokenUsage::default());
        }

        assert_eq!(
            state.decide(SemanticCompactInput {
                feature_enabled: false,
                ..input(MIN_SEMANTIC_TOKENS)
            }),
            SemanticCompactDecision::Skip
        );
    }

    #[test]
    fn semantic_compact_triggers_after_long_continuation() {
        let mut state = SemanticCompactState::default();
        for _ in 0..MIN_CONTINUATION_TURNS {
            state.record_regular_turn_finished(&TokenUsage::default());
        }

        assert_eq!(
            state.decide(input(MIN_SEMANTIC_TOKENS)),
            SemanticCompactDecision::Compact {
                reason: CompactionReason::SemanticCheckpoint,
            }
        );
    }

    #[test]
    fn semantic_compact_triggers_before_context_limit_pressure() {
        let state = SemanticCompactState::default();

        assert_eq!(
            state.decide(input(early_pressure_threshold(100_000))),
            SemanticCompactDecision::Compact {
                reason: CompactionReason::SemanticCheckpoint,
            }
        );
    }

    #[test]
    fn semantic_compact_leaves_hard_limit_to_existing_path() {
        let mut state = SemanticCompactState::default();
        for _ in 0..MIN_CONTINUATION_TURNS {
            state.record_regular_turn_finished(&TokenUsage::default());
        }

        assert_eq!(state.decide(input(100_000)), SemanticCompactDecision::Skip);
    }

    #[test]
    fn semantic_compact_cooldown_resets_after_compaction() {
        let mut state = SemanticCompactState::default();
        for _ in 0..MIN_CONTINUATION_TURNS {
            state.record_regular_turn_finished(&TokenUsage::default());
        }
        state.record_compaction_finished();

        assert_eq!(
            state.decide(input(MIN_SEMANTIC_TOKENS)),
            SemanticCompactDecision::Skip
        );

        for _ in 0..COOLDOWN_TURNS {
            state.record_regular_turn_finished(&TokenUsage::default());
        }

        assert_eq!(
            state.decide(input(MIN_SEMANTIC_TOKENS)),
            SemanticCompactDecision::Compact {
                reason: CompactionReason::SemanticCheckpoint,
            }
        );
    }

    #[test]
    fn semantic_compact_triggers_after_sustained_work_checkpoint() {
        let mut state = SemanticCompactState::default();
        for _ in 0..WORK_CHECKPOINT_TURNS {
            state.record_regular_turn_finished(&TokenUsage {
                input_tokens: 6_000,
                cached_input_tokens: 1_000,
                output_tokens: 500,
                ..TokenUsage::default()
            });
        }

        assert_eq!(
            state.decide(input(WORK_CHECKPOINT_MIN_TOTAL_TOKENS)),
            SemanticCompactDecision::Compact {
                reason: CompactionReason::SemanticCheckpoint,
            }
        );
    }

    #[test]
    fn semantic_compact_work_checkpoint_requires_meaningful_total_context() {
        let mut state = SemanticCompactState::default();
        for _ in 0..WORK_CHECKPOINT_TURNS {
            state.record_regular_turn_finished(&TokenUsage {
                input_tokens: 6_000,
                output_tokens: 500,
                ..TokenUsage::default()
            });
        }

        assert_eq!(
            state.decide(input(WORK_CHECKPOINT_MIN_TOTAL_TOKENS - 1)),
            SemanticCompactDecision::Skip
        );
    }
}
