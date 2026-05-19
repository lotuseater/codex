//! Policy and prompts for automatic context reduction.

/// Default fraction of the usable context window that triggers context reduction.
pub const DEFAULT_TRIGGER_CONTEXT_PERCENT: u8 = 20;

/// Default number of completed regular turns to wait after a reduction.
pub const DEFAULT_TURN_COOLDOWN: u32 = 24;

/// Prompt used for prune-style context reduction.
pub const PRUNE_NUDGE_PROMPT: &str = "here is the context of other llm model. Please remove from the context all not needed for further task implementation by the model. preserve all that may be useful\n\nReturn only the reduced context. Do not explain your method.";

/// Tunable policy for automatic context reduction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextReductionPolicy {
    trigger_context_percent: u8,
    cooldown_turns: u32,
}

impl ContextReductionPolicy {
    pub const fn new(trigger_context_percent: u8, cooldown_turns: u32) -> Self {
        Self {
            trigger_context_percent,
            cooldown_turns,
        }
    }

    pub const fn trigger_context_percent(self) -> u8 {
        self.trigger_context_percent
    }

    pub const fn cooldown_turns(self) -> u32 {
        self.cooldown_turns
    }
}

impl Default for ContextReductionPolicy {
    fn default() -> Self {
        Self::new(DEFAULT_TRIGGER_CONTEXT_PERCENT, DEFAULT_TURN_COOLDOWN)
    }
}

/// Runtime inputs needed to decide whether context should be reduced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextReductionInput {
    pub total_usage_tokens: i64,
    pub auto_compact_limit: i64,
}

/// Decision returned by [`ContextReductionState`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextReductionDecision {
    Skip,
    Reduce { threshold_tokens: i64 },
}

/// Per-session state for automatic context reduction.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ContextReductionState {
    turns_since_last_reduction: u32,
    cooldown_turns_remaining: u32,
}

impl ContextReductionState {
    pub fn record_regular_turn_finished(&mut self) {
        self.turns_since_last_reduction = self.turns_since_last_reduction.saturating_add(1);
        self.cooldown_turns_remaining = self.cooldown_turns_remaining.saturating_sub(1);
    }

    pub fn record_reduction_finished(&mut self, policy: ContextReductionPolicy) {
        self.turns_since_last_reduction = 0;
        self.cooldown_turns_remaining = policy.cooldown_turns();
    }

    pub fn decide(
        &self,
        policy: ContextReductionPolicy,
        input: ContextReductionInput,
    ) -> ContextReductionDecision {
        let Some(threshold_tokens) = trigger_threshold_tokens(policy, input.auto_compact_limit)
        else {
            return ContextReductionDecision::Skip;
        };
        if input.total_usage_tokens >= input.auto_compact_limit
            || self.cooldown_turns_remaining > 0
            || input.total_usage_tokens < threshold_tokens
        {
            return ContextReductionDecision::Skip;
        }
        ContextReductionDecision::Reduce { threshold_tokens }
    }
}

pub fn trigger_threshold_tokens(
    policy: ContextReductionPolicy,
    auto_compact_limit: i64,
) -> Option<i64> {
    if auto_compact_limit <= 0 || policy.trigger_context_percent() == 0 {
        return None;
    }
    let trigger_percent = i64::from(policy.trigger_context_percent().min(100));
    let numerator = auto_compact_limit.saturating_mul(trigger_percent);
    Some(numerator.saturating_add(99) / 100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn input(total_usage_tokens: i64) -> ContextReductionInput {
        ContextReductionInput {
            total_usage_tokens,
            auto_compact_limit: 100_000,
        }
    }

    #[test]
    fn default_policy_matches_prune_nudge_defaults() {
        assert_eq!(
            ContextReductionPolicy::default(),
            ContextReductionPolicy::new(20, 24)
        );
        assert_eq!(
            PRUNE_NUDGE_PROMPT,
            "here is the context of other llm model. Please remove from the context all not needed for further task implementation by the model. preserve all that may be useful\n\nReturn only the reduced context. Do not explain your method."
        );
    }

    #[test]
    fn triggers_at_configured_context_percentage() {
        let state = ContextReductionState::default();
        let policy = ContextReductionPolicy::default();

        assert_eq!(
            state.decide(policy, input(19_999)),
            ContextReductionDecision::Skip
        );
        assert_eq!(
            state.decide(policy, input(20_000)),
            ContextReductionDecision::Reduce {
                threshold_tokens: 20_000,
            }
        );
    }

    #[test]
    fn threshold_rounds_up_to_avoid_triggering_early() {
        assert_eq!(
            trigger_threshold_tokens(ContextReductionPolicy::new(20, 24), 101),
            Some(21)
        );
    }

    #[test]
    fn skips_when_context_limit_compaction_should_handle_it() {
        let state = ContextReductionState::default();
        let policy = ContextReductionPolicy::default();

        assert_eq!(
            state.decide(policy, input(100_000)),
            ContextReductionDecision::Skip
        );
    }

    #[test]
    fn cooldown_blocks_reduction_until_enough_regular_turns_finish() {
        let mut state = ContextReductionState::default();
        let policy = ContextReductionPolicy::default();
        state.record_reduction_finished(policy);

        for _ in 0..23 {
            state.record_regular_turn_finished();
            assert_eq!(
                state.decide(policy, input(20_000)),
                ContextReductionDecision::Skip
            );
        }

        state.record_regular_turn_finished();
        assert_eq!(
            state.decide(policy, input(20_000)),
            ContextReductionDecision::Reduce {
                threshold_tokens: 20_000,
            }
        );
    }
}
