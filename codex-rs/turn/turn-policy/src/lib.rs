//! Policy boundary for deciding whether a turn may run.
//!
//! This crate owns policy decisions only. It should not execute tools or mutate
//! turn state directly.

#![forbid(unsafe_code)]

use codex_turn_api::TurnInput;
use codex_turn_state::TurnState;

/// Immutable inputs available to turn policy implementations.
#[derive(Clone, Copy, Debug)]
pub struct TurnPolicyContext<'a> {
    input: &'a TurnInput,
    state: &'a TurnState,
}

impl<'a> TurnPolicyContext<'a> {
    /// Creates policy context for a turn.
    #[must_use]
    pub const fn new(input: &'a TurnInput, state: &'a TurnState) -> Self {
        Self { input, state }
    }

    /// Returns the input being evaluated.
    #[must_use]
    pub const fn input(&self) -> &'a TurnInput {
        self.input
    }

    /// Returns the current turn state.
    #[must_use]
    pub const fn state(&self) -> &'a TurnState {
        self.state
    }
}

/// Policy outcome for a turn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TurnPolicyDecision {
    Allow,
    Defer { reason: String },
    Reject { reason: String },
}

/// Decides whether a turn is allowed to execute.
///
/// Implementations should be deterministic for a given context and should
/// return the narrowest decision that explains why execution cannot proceed.
pub trait TurnPolicy {
    fn evaluate(&self, context: &TurnPolicyContext<'_>) -> TurnPolicyDecision;
}

/// Policy implementation that allows every turn.
#[derive(Clone, Copy, Debug, Default)]
pub struct AllowAllPolicy;

impl TurnPolicy for AllowAllPolicy {
    fn evaluate(&self, _context: &TurnPolicyContext<'_>) -> TurnPolicyDecision {
        TurnPolicyDecision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_turn_state::TurnPhase;

    fn turn_id(value: u64) -> TurnId {
        TurnId::new(format!("turn-{value}"))
    }

    #[test]
    fn policy_context_exposes_input_and_state() {
        let turn_id = turn_id(31);
        let input = TurnInput::new(turn_id.clone(), "continue");
        let state = TurnState::new(turn_id.clone());

        let context = TurnPolicyContext::new(&input, &state);

        assert_eq!(&turn_id, context.input().turn_id());
        assert_eq!("continue", context.input().prompt());
        assert_eq!(&turn_id, context.state().turn_id());
        assert_eq!(TurnPhase::Queued, context.state().phase());
    }

    #[test]
    fn allow_all_policy_allows_turn_without_core_runtime() {
        let turn_id = turn_id(37);
        let input = TurnInput::new(turn_id.clone(), "run");
        let state = TurnState::new(turn_id);
        let context = TurnPolicyContext::new(&input, &state);

        assert_eq!(TurnPolicyDecision::Allow, AllowAllPolicy.evaluate(&context));
    }
}
