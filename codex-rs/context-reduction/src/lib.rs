//! Policy and prompts for automatic context reduction.

/// Default fraction of the usable context window that triggers context reduction.
pub const DEFAULT_TRIGGER_CONTEXT_PERCENT: u8 = 20;

/// Default number of completed regular turns to wait after a reduction.
pub const DEFAULT_TURN_COOLDOWN: u32 = 24;
pub const RESTORED_SESSION_AUTO_COMPACT_TOKEN_LIMIT: i64 = 80_000;

const MIN_CONTINUATION_TURNS: u32 = 8;
const MIN_SEMANTIC_TOKENS: i64 = 80_000;
const WORK_CHECKPOINT_TURNS: u32 = 6;
const WORK_CHECKPOINT_TOKENS: i64 = 32_000;
const WORK_CHECKPOINT_MIN_TOTAL_TOKENS: i64 = 50_000;
const COMMIT_CHECKPOINT_MIN_TOTAL_TOKENS: i64 = 20_000;
const TOOL_CHECKPOINT_CALLS: u64 = 12;
const TOOL_CHECKPOINT_MIN_TOTAL_TOKENS: i64 = 40_000;
const SEMANTIC_COOLDOWN_TURNS: u32 = 4;

pub const PRUNE_NUDGE_PROMPT: &str = "\
Here is the context left by another LLM model. Reduce it for the next model that will continue the same task.

Preserve everything needed to continue implementation without rediscovery:
- The user's goal and any explicit constraints, preferences, or requested workflow.
- The active plan, its current implementation stage, which items are completed, which item is in progress, and which items are not started.
- The next concrete actions to take, including file paths, commands, tests, artifacts, or logs needed to resume.
- Important decisions, assumptions, blockers, risks, verification results, and dirty or user-owned worktree changes that must not be overwritten.
- Any important code/session details that would be costly or unsafe to rediscover.

Remove obsolete exploration, repeated tool output, dead ends, and low-signal narration. Do not omit unresolved work or collapse it into vague phrases such as \"continue the plan\"; name the exact remaining steps.

Return only the reduced context. Do not explain your method.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextReductionPolicy {
    trigger_context_percent: u8,
    turn_cooldown: u32,
}

impl ContextReductionPolicy {
    pub fn new(trigger_context_percent: u8, turn_cooldown: u32) -> Self {
        Self {
            trigger_context_percent,
            turn_cooldown,
        }
    }

    pub fn trigger_context_percent(self) -> u8 {
        self.trigger_context_percent
    }

    pub fn turn_cooldown(self) -> u32 {
        self.turn_cooldown
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
    pub context_window: Option<i64>,
    pub visible_context_percent_used: Option<i64>,
}

/// Decision returned by [`ContextReductionState`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextReductionDecision {
    Skip,
    Reduce { threshold_tokens: i64 },
}

#[derive(Debug, Default)]
pub struct ContextReductionState {
    cooldown_remaining_turns: u32,
    visible_threshold_observed: bool,
}

impl ContextReductionState {
    pub fn record_regular_turn_finished(&mut self) {
        self.cooldown_remaining_turns = self.cooldown_remaining_turns.saturating_sub(1);
    }

    pub fn observe_visible_context_percent(
        &mut self,
        policy: ContextReductionPolicy,
        visible_context_percent_used: Option<i64>,
    ) {
        if visible_context_percent_used
            .is_some_and(|percent| percent >= i64::from(policy.trigger_context_percent()))
        {
            self.visible_threshold_observed = true;
        }
    }

    pub fn clear_observed_visible_threshold(&mut self) {
        self.visible_threshold_observed = false;
    }

    pub fn record_reduction_finished(&mut self, policy: ContextReductionPolicy) {
        self.clear_observed_visible_threshold();
        self.cooldown_remaining_turns = policy.turn_cooldown();
    }

    pub fn decide(
        &self,
        policy: ContextReductionPolicy,
        input: ContextReductionInput,
    ) -> ContextReductionDecision {
        let threshold_context_window = input
            .context_window
            .filter(|context_window| *context_window > 0)
            .unwrap_or(input.auto_compact_limit);
        let Some(threshold_tokens) = trigger_threshold_tokens(policy, threshold_context_window)
        else {
            return ContextReductionDecision::Skip;
        };
        let visible_threshold_reached = self.visible_threshold_observed
            || input
                .visible_context_percent_used
                .is_some_and(|percent| percent >= i64::from(policy.trigger_context_percent()));

        if input.total_usage_tokens >= input.auto_compact_limit
            || self.cooldown_remaining_turns > 0
            || (!visible_threshold_reached && input.total_usage_tokens < threshold_tokens)
        {
            return ContextReductionDecision::Skip;
        }
        ContextReductionDecision::Reduce { threshold_tokens }
    }
}

pub fn trigger_threshold_tokens(
    policy: ContextReductionPolicy,
    threshold_context_window: i64,
) -> Option<i64> {
    if threshold_context_window <= 0 || policy.trigger_context_percent() == 0 {
        return None;
    }
    let trigger_percent = i64::from(policy.trigger_context_percent().min(100));
    let numerator = threshold_context_window.saturating_mul(trigger_percent);
    Some(numerator.saturating_add(99) / 100)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextReductionReason {
    ContextLimit,
    SemanticCheckpoint,
    EarlyContextPressure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticCompactDecision {
    Skip,
    Compact { reason: ContextReductionReason },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticCompactInput {
    pub enabled: bool,
    pub policy: ContextReductionPolicy,
    pub total_usage_tokens: i64,
    pub auto_compact_limit: i64,
    pub context_window: Option<i64>,
    pub visible_context_percent_used: Option<i64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticCompactTurnInput {
    pub non_cached_input_tokens: i64,
    pub output_tokens: i64,
    pub tool_calls: u64,
    pub git_commit_observed: bool,
    pub is_continuation_turn: bool,
}

#[derive(Debug, Default)]
pub struct SemanticCompactState {
    semantic_cooldown_remaining_turns: u32,
    regular_turns_since_last_compact: u32,
    continuation_turns_since_last_compact: u32,
    work_tokens_since_last_compact: i64,
    tool_calls_since_last_compact: u64,
    git_commit_observed_since_last_compact: bool,
    early_context_pressure_state: ContextReductionState,
}

impl SemanticCompactState {
    pub fn observe_visible_context_percent(
        &mut self,
        policy: ContextReductionPolicy,
        visible_context_percent_used: Option<i64>,
    ) {
        self.early_context_pressure_state
            .observe_visible_context_percent(policy, visible_context_percent_used);
    }

    pub fn record_regular_turn_finished(&mut self, input: SemanticCompactTurnInput) {
        self.semantic_cooldown_remaining_turns =
            self.semantic_cooldown_remaining_turns.saturating_sub(1);
        self.early_context_pressure_state
            .record_regular_turn_finished();
        self.regular_turns_since_last_compact =
            self.regular_turns_since_last_compact.saturating_add(1);
        if input.is_continuation_turn {
            self.continuation_turns_since_last_compact =
                self.continuation_turns_since_last_compact.saturating_add(1);
        }
        self.work_tokens_since_last_compact = self
            .work_tokens_since_last_compact
            .saturating_add(turn_work_tokens(input));
        self.tool_calls_since_last_compact = self
            .tool_calls_since_last_compact
            .saturating_add(input.tool_calls);
        self.git_commit_observed_since_last_compact |= input.git_commit_observed;
    }

    pub fn record_compaction_finished(&mut self, _reason: Option<ContextReductionReason>) {
        self.semantic_cooldown_remaining_turns = SEMANTIC_COOLDOWN_TURNS;
        self.regular_turns_since_last_compact = 0;
        self.continuation_turns_since_last_compact = 0;
        self.work_tokens_since_last_compact = 0;
        self.tool_calls_since_last_compact = 0;
        self.git_commit_observed_since_last_compact = false;
        self.early_context_pressure_state
            .record_reduction_finished(ContextReductionPolicy::default());
    }

    pub fn decide(&self, input: SemanticCompactInput) -> SemanticCompactDecision {
        if input.auto_compact_limit <= 0 || input.total_usage_tokens >= input.auto_compact_limit {
            return SemanticCompactDecision::Skip;
        }

        if matches!(
            self.early_context_pressure_state.decide(
                input.policy,
                ContextReductionInput {
                    total_usage_tokens: input.total_usage_tokens,
                    auto_compact_limit: input.auto_compact_limit,
                    context_window: input.context_window,
                    visible_context_percent_used: input.visible_context_percent_used,
                },
            ),
            ContextReductionDecision::Reduce { .. }
        ) {
            return SemanticCompactDecision::Compact {
                reason: ContextReductionReason::EarlyContextPressure,
            };
        }

        if !input.enabled || self.semantic_cooldown_remaining_turns > 0 {
            return SemanticCompactDecision::Skip;
        }

        if (self.continuation_turns_since_last_compact >= MIN_CONTINUATION_TURNS
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
                reason: ContextReductionReason::SemanticCheckpoint,
            }
        } else {
            SemanticCompactDecision::Skip
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PostSamplingAutoCompactAction {
    BeforeFollowUp(ContextReductionReason),
    AfterFinalResponse(ContextReductionReason),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PostSamplingAutoCompactInput {
    pub needs_follow_up: bool,
    pub total_usage_tokens: i64,
    pub auto_compact_limit: i64,
    pub semantic_compact_decision: SemanticCompactDecision,
}

pub fn post_sampling_auto_compact_action(
    input: PostSamplingAutoCompactInput,
) -> Option<PostSamplingAutoCompactAction> {
    if input.auto_compact_limit <= 0 {
        return None;
    }

    let reason = if input.total_usage_tokens >= input.auto_compact_limit {
        ContextReductionReason::ContextLimit
    } else {
        match input.semantic_compact_decision {
            SemanticCompactDecision::Skip => return None,
            SemanticCompactDecision::Compact { reason } => reason,
        }
    };

    Some(if input.needs_follow_up {
        PostSamplingAutoCompactAction::BeforeFollowUp(reason)
    } else {
        PostSamplingAutoCompactAction::AfterFinalResponse(reason)
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutoCompactBudgetMode {
    Standard,
    Slow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelAutoCompactLimits {
    pub auto_compact_token_limit: Option<i64>,
    pub context_window: Option<i64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutoCompactTokenLimitInput {
    pub model_limits: ModelAutoCompactLimits,
    pub runtime_context_window: Option<i64>,
    pub budget_mode: AutoCompactBudgetMode,
}

pub fn auto_compact_token_limit_for_mode(input: AutoCompactTokenLimitInput) -> i64 {
    let limit = auto_compact_token_limit_from_model_info(input.model_limits);
    if input.budget_mode != AutoCompactBudgetMode::Slow {
        return limit;
    }
    input
        .runtime_context_window
        .map(|context_window| limit.min(context_window.saturating_mul(3) / 4))
        .unwrap_or(limit)
}

pub fn auto_compact_token_limit_from_model_info(model_limits: ModelAutoCompactLimits) -> i64 {
    model_limits
        .auto_compact_token_limit
        .or_else(|| {
            model_limits
                .context_window
                .map(|window| window.saturating_mul(4) / 5)
        })
        .unwrap_or(i64::MAX)
}

pub fn restored_session_auto_compact_token_limit(auto_compact_limit: i64) -> i64 {
    auto_compact_limit.clamp(1, RESTORED_SESSION_AUTO_COMPACT_TOKEN_LIMIT)
}

fn turn_work_tokens(input: SemanticCompactTurnInput) -> i64 {
    input
        .non_cached_input_tokens
        .max(0)
        .saturating_add(input.output_tokens.max(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn input(total_usage_tokens: i64) -> ContextReductionInput {
        ContextReductionInput {
            total_usage_tokens,
            auto_compact_limit: 100_000,
            context_window: Some(100_000),
            visible_context_percent_used: None,
        }
    }

    fn semantic_input(enabled: bool, total_usage_tokens: i64) -> SemanticCompactInput {
        SemanticCompactInput {
            enabled,
            policy: ContextReductionPolicy::default(),
            total_usage_tokens,
            auto_compact_limit: 100_000,
            context_window: Some(100_000),
            visible_context_percent_used: None,
        }
    }

    fn semantic_input_with_visible_percent(
        enabled: bool,
        total_usage_tokens: i64,
        visible_context_percent_used: i64,
    ) -> SemanticCompactInput {
        SemanticCompactInput {
            enabled,
            policy: ContextReductionPolicy::default(),
            total_usage_tokens,
            auto_compact_limit: 100_000,
            context_window: Some(100_000),
            visible_context_percent_used: Some(visible_context_percent_used),
        }
    }

    fn turn_input(non_cached_input_tokens: i64) -> SemanticCompactTurnInput {
        SemanticCompactTurnInput {
            non_cached_input_tokens,
            output_tokens: 0,
            tool_calls: 0,
            git_commit_observed: false,
            is_continuation_turn: false,
        }
    }

    #[test]
    fn prune_nudge_prompt_uses_custom_reduction_prompt() {
        for required_text in [
            "Reduce it for the next model",
            "active plan",
            "current implementation stage",
            "next concrete actions",
            "dirty or user-owned worktree changes",
            "Do not omit unresolved work",
            "exact remaining steps",
        ] {
            assert!(
                PRUNE_NUDGE_PROMPT.contains(required_text),
                "prune nudge prompt must mention {required_text:?}"
            );
        }
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
    fn trigger_threshold_rounds_up() {
        assert_eq!(
            trigger_threshold_tokens(
                ContextReductionPolicy::new(20, DEFAULT_TURN_COOLDOWN),
                100_001
            ),
            Some(20_001)
        );
    }

    #[test]
    fn token_pressure_uses_context_window_not_auto_compact_limit() {
        let state = ContextReductionState::default();
        let policy = ContextReductionPolicy::default();

        assert_eq!(
            state.decide(
                policy,
                ContextReductionInput {
                    total_usage_tokens: 10_000,
                    auto_compact_limit: 50_000,
                    context_window: Some(100_000),
                    visible_context_percent_used: None,
                }
            ),
            ContextReductionDecision::Skip
        );
        assert_eq!(
            state.decide(
                policy,
                ContextReductionInput {
                    total_usage_tokens: 20_000,
                    auto_compact_limit: 50_000,
                    context_window: Some(100_000),
                    visible_context_percent_used: None,
                }
            ),
            ContextReductionDecision::Reduce {
                threshold_tokens: 20_000,
            }
        );
    }

    #[test]
    fn semantic_early_pressure_uses_configured_policy() {
        let state = SemanticCompactState::default();
        let policy = ContextReductionPolicy::new(35, DEFAULT_TURN_COOLDOWN);

        assert_eq!(
            state.decide(SemanticCompactInput {
                enabled: true,
                policy,
                total_usage_tokens: 34_999,
                auto_compact_limit: 100_000,
                context_window: Some(100_000),
                visible_context_percent_used: None,
            }),
            SemanticCompactDecision::Skip
        );
        assert_eq!(
            state.decide(SemanticCompactInput {
                enabled: true,
                policy,
                total_usage_tokens: 35_000,
                auto_compact_limit: 100_000,
                context_window: Some(100_000),
                visible_context_percent_used: None,
            }),
            SemanticCompactDecision::Compact {
                reason: ContextReductionReason::EarlyContextPressure,
            }
        );
    }

    #[test]
    fn observed_visible_percent_uses_configured_policy() {
        let mut state = SemanticCompactState::default();
        let policy = ContextReductionPolicy::new(35, DEFAULT_TURN_COOLDOWN);

        state.observe_visible_context_percent(policy, Some(34));
        assert_eq!(
            state.decide(SemanticCompactInput {
                enabled: true,
                policy,
                total_usage_tokens: 1,
                auto_compact_limit: 100_000,
                context_window: Some(100_000),
                visible_context_percent_used: None,
            }),
            SemanticCompactDecision::Skip
        );

        state.observe_visible_context_percent(policy, Some(35));
        assert_eq!(
            state.decide(SemanticCompactInput {
                enabled: true,
                policy,
                total_usage_tokens: 1,
                auto_compact_limit: 100_000,
                context_window: Some(100_000),
                visible_context_percent_used: None,
            }),
            SemanticCompactDecision::Compact {
                reason: ContextReductionReason::EarlyContextPressure,
            }
        );
    }

    #[test]
    fn zero_percent_policy_disables_semantic_early_pressure() {
        let state = SemanticCompactState::default();

        assert_eq!(
            state.decide(SemanticCompactInput {
                enabled: true,
                policy: ContextReductionPolicy::new(0, DEFAULT_TURN_COOLDOWN),
                total_usage_tokens: 99_999,
                auto_compact_limit: 100_000,
                context_window: Some(100_000),
                visible_context_percent_used: Some(100),
            }),
            SemanticCompactDecision::Skip
        );
    }

    #[test]
    fn skips_at_full_auto_compact_limit() {
        let state = ContextReductionState::default();
        let policy = ContextReductionPolicy::default();

        assert_eq!(
            state.decide(policy, input(100_000)),
            ContextReductionDecision::Skip
        );
    }

    #[test]
    fn first_reduction_arms_turn_cooldown() {
        let mut state = ContextReductionState::default();
        let policy = ContextReductionPolicy::default();

        state.record_reduction_finished(policy);
        assert_eq!(
            state.decide(policy, input(20_000)),
            ContextReductionDecision::Skip
        );
        for _ in 0..23 {
            state.record_regular_turn_finished();
        }
        assert_eq!(
            state.decide(policy, input(20_000)),
            ContextReductionDecision::Skip
        );
        state.record_regular_turn_finished();
        assert_eq!(
            state.decide(policy, input(20_000)),
            ContextReductionDecision::Reduce {
                threshold_tokens: 20_000,
            }
        );
    }

    #[test]
    fn semantic_policy_uses_twenty_percent_early_pressure_without_feature_flag() {
        let state = SemanticCompactState::default();

        assert_eq!(
            state.decide(semantic_input(false, 20_000)),
            SemanticCompactDecision::Compact {
                reason: ContextReductionReason::EarlyContextPressure,
            }
        );
    }

    #[test]
    fn semantic_policy_does_not_have_a_second_eighty_percent_guard() {
        let state = SemanticCompactState::default();

        assert_eq!(
            state.decide(semantic_input(false, 19_999)),
            SemanticCompactDecision::Skip
        );
        assert_eq!(
            state.decide(semantic_input(false, 20_000)),
            SemanticCompactDecision::Compact {
                reason: ContextReductionReason::EarlyContextPressure,
            }
        );
    }

    #[test]
    fn visible_context_percent_triggers_at_twenty_percent() {
        let state = SemanticCompactState::default();

        assert_eq!(
            state.decide(semantic_input_with_visible_percent(false, 1, 19)),
            SemanticCompactDecision::Skip
        );
        assert_eq!(
            state.decide(semantic_input_with_visible_percent(false, 1, 20)),
            SemanticCompactDecision::Compact {
                reason: ContextReductionReason::EarlyContextPressure,
            }
        );
    }

    #[test]
    fn observed_visible_context_percent_latches_until_decision() {
        let mut state = SemanticCompactState::default();
        state.observe_visible_context_percent(
            ContextReductionPolicy::default(),
            Some(i64::from(DEFAULT_TRIGGER_CONTEXT_PERCENT)),
        );
        state.observe_visible_context_percent(
            ContextReductionPolicy::default(),
            Some(i64::from(DEFAULT_TRIGGER_CONTEXT_PERCENT) - 1),
        );

        assert_eq!(
            state.decide(semantic_input(false, 1)),
            SemanticCompactDecision::Compact {
                reason: ContextReductionReason::EarlyContextPressure,
            }
        );
    }

    #[test]
    fn non_early_compaction_clears_observed_visible_context_percent() {
        let mut state = SemanticCompactState::default();
        state.observe_visible_context_percent(
            ContextReductionPolicy::default(),
            Some(i64::from(DEFAULT_TRIGGER_CONTEXT_PERCENT)),
        );
        state.record_compaction_finished(Some(ContextReductionReason::SemanticCheckpoint));

        assert_eq!(
            state.decide(semantic_input(false, 1)),
            SemanticCompactDecision::Skip
        );
    }

    #[test]
    fn early_pressure_cooldown_requires_twenty_four_model_moves() {
        let mut state = SemanticCompactState::default();
        state.record_compaction_finished(Some(ContextReductionReason::EarlyContextPressure));

        for _ in 0..23 {
            state.record_regular_turn_finished(turn_input(1));
            assert_eq!(
                state.decide(semantic_input(false, 20_000)),
                SemanticCompactDecision::Skip
            );
        }

        state.record_regular_turn_finished(turn_input(1));
        assert_eq!(
            state.decide(semantic_input(false, 20_000)),
            SemanticCompactDecision::Compact {
                reason: ContextReductionReason::EarlyContextPressure,
            }
        );
    }

    #[test]
    fn early_pressure_cooldown_also_blocks_visible_percent_trigger() {
        let mut state = SemanticCompactState::default();
        state.record_compaction_finished(Some(ContextReductionReason::EarlyContextPressure));

        for _ in 0..23 {
            state.record_regular_turn_finished(turn_input(1));
            assert_eq!(
                state.decide(semantic_input_with_visible_percent(false, 1, 20)),
                SemanticCompactDecision::Skip
            );
        }

        state.record_regular_turn_finished(turn_input(1));
        assert_eq!(
            state.decide(semantic_input_with_visible_percent(false, 1, 20)),
            SemanticCompactDecision::Compact {
                reason: ContextReductionReason::EarlyContextPressure,
            }
        );
    }

    #[test]
    fn other_compactions_start_early_pressure_cooldown() {
        let mut state = SemanticCompactState::default();
        state.record_compaction_finished(Some(ContextReductionReason::SemanticCheckpoint));

        for _ in 0..23 {
            state.record_regular_turn_finished(turn_input(1));
            assert_eq!(
                state.decide(semantic_input(false, 20_000)),
                SemanticCompactDecision::Skip
            );
        }

        state.record_regular_turn_finished(turn_input(1));
        assert_eq!(
            state.decide(semantic_input(false, 20_000)),
            SemanticCompactDecision::Compact {
                reason: ContextReductionReason::EarlyContextPressure,
            }
        );
    }

    #[test]
    fn other_compactions_reset_early_pressure_cooldown() {
        let mut state = SemanticCompactState::default();
        state.record_compaction_finished(Some(ContextReductionReason::EarlyContextPressure));

        for _ in 0..12 {
            state.record_regular_turn_finished(turn_input(1));
        }

        state.record_compaction_finished(Some(ContextReductionReason::ContextLimit));

        for _ in 0..23 {
            state.record_regular_turn_finished(turn_input(1));
            assert_eq!(
                state.decide(semantic_input_with_visible_percent(false, 1, 20)),
                SemanticCompactDecision::Skip
            );
        }

        state.record_regular_turn_finished(turn_input(1));
        assert_eq!(
            state.decide(semantic_input_with_visible_percent(false, 1, 20)),
            SemanticCompactDecision::Compact {
                reason: ContextReductionReason::EarlyContextPressure,
            }
        );
    }

    #[test]
    fn semantic_checkpoint_still_fires_after_work_checkpoint() {
        let mut state = SemanticCompactState::default();
        for _ in 0..6 {
            state.record_regular_turn_finished(turn_input(6_000));
        }

        assert_eq!(
            state.decide(SemanticCompactInput {
                enabled: true,
                policy: ContextReductionPolicy::default(),
                total_usage_tokens: 50_000,
                auto_compact_limit: 1_000_000,
                context_window: Some(1_000_000),
                visible_context_percent_used: None,
            }),
            SemanticCompactDecision::Compact {
                reason: ContextReductionReason::SemanticCheckpoint,
            }
        );
    }

    #[test]
    fn early_pressure_takes_precedence_over_semantic_checkpoint() {
        let mut state = SemanticCompactState::default();
        for _ in 0..6 {
            state.record_regular_turn_finished(SemanticCompactTurnInput {
                tool_calls: 12,
                git_commit_observed: true,
                ..turn_input(6_000)
            });
        }

        assert_eq!(
            state.decide(semantic_input(true, 50_000)),
            SemanticCompactDecision::Compact {
                reason: ContextReductionReason::EarlyContextPressure,
            }
        );
    }

    #[test]
    fn post_sampling_action_compacts_before_follow_up() {
        assert_eq!(
            post_sampling_auto_compact_action(PostSamplingAutoCompactInput {
                needs_follow_up: true,
                total_usage_tokens: 20_000,
                auto_compact_limit: 100_000,
                semantic_compact_decision: SemanticCompactDecision::Compact {
                    reason: ContextReductionReason::EarlyContextPressure,
                },
            }),
            Some(PostSamplingAutoCompactAction::BeforeFollowUp(
                ContextReductionReason::EarlyContextPressure
            ))
        );
    }

    #[test]
    fn post_sampling_action_compacts_after_final_response() {
        assert_eq!(
            post_sampling_auto_compact_action(PostSamplingAutoCompactInput {
                needs_follow_up: false,
                total_usage_tokens: 20_000,
                auto_compact_limit: 100_000,
                semantic_compact_decision: SemanticCompactDecision::Compact {
                    reason: ContextReductionReason::EarlyContextPressure,
                },
            }),
            Some(PostSamplingAutoCompactAction::AfterFinalResponse(
                ContextReductionReason::EarlyContextPressure
            ))
        );
    }

    #[test]
    fn post_sampling_context_limit_compacts_after_final_response() {
        assert_eq!(
            post_sampling_auto_compact_action(PostSamplingAutoCompactInput {
                needs_follow_up: false,
                total_usage_tokens: 100_000,
                auto_compact_limit: 100_000,
                semantic_compact_decision: SemanticCompactDecision::Skip,
            }),
            Some(PostSamplingAutoCompactAction::AfterFinalResponse(
                ContextReductionReason::ContextLimit
            ))
        );
    }

    #[test]
    fn post_sampling_context_limit_compacts_before_follow_up() {
        assert_eq!(
            post_sampling_auto_compact_action(PostSamplingAutoCompactInput {
                needs_follow_up: true,
                total_usage_tokens: 100_000,
                auto_compact_limit: 100_000,
                semantic_compact_decision: SemanticCompactDecision::Skip,
            }),
            Some(PostSamplingAutoCompactAction::BeforeFollowUp(
                ContextReductionReason::ContextLimit
            ))
        );
    }

    #[test]
    fn post_sampling_skips_without_auto_compact_limit() {
        assert_eq!(
            post_sampling_auto_compact_action(PostSamplingAutoCompactInput {
                needs_follow_up: false,
                total_usage_tokens: 100_000,
                auto_compact_limit: 0,
                semantic_compact_decision: SemanticCompactDecision::Compact {
                    reason: ContextReductionReason::EarlyContextPressure,
                },
            }),
            None
        );
    }

    #[test]
    fn slow_budget_caps_runtime_context_at_seventy_five_percent() {
        assert_eq!(
            auto_compact_token_limit_for_mode(AutoCompactTokenLimitInput {
                model_limits: ModelAutoCompactLimits {
                    auto_compact_token_limit: Some(90_000),
                    context_window: Some(100_000),
                },
                runtime_context_window: Some(80_000),
                budget_mode: AutoCompactBudgetMode::Slow,
            }),
            60_000
        );
    }

    #[test]
    fn restored_session_limit_is_capped_at_eighty_thousand() {
        assert_eq!(restored_session_auto_compact_token_limit(750_000), 80_000);
    }

    #[test]
    fn restored_session_limit_preserves_lower_model_limit() {
        assert_eq!(restored_session_auto_compact_token_limit(32_000), 32_000);
    }
}
