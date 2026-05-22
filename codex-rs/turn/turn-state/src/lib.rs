//! In-memory state transitions for a turn.
//!
//! This crate owns state-machine vocabulary. It does not emit events or decide
//! whether a transition is allowed.

#![forbid(unsafe_code)]

use codex_turn_api::TurnId;

/// Lifecycle phase for a turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TurnPhase {
    Queued,
    Running,
    Waiting,
    Completed,
    Failed,
}

/// Current state for a turn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnState {
    phase: TurnPhase,
    revision: u64,
    turn_id: TurnId,
}

impl TurnState {
    /// Creates queued state for a new turn.
    #[must_use]
    pub fn new(turn_id: TurnId) -> Self {
        Self {
            phase: TurnPhase::Queued,
            revision: 0,
            turn_id,
        }
    }

    /// Returns the turn id for this state.
    #[must_use]
    pub const fn turn_id(&self) -> &TurnId {
        &self.turn_id
    }

    /// Returns the current lifecycle phase.
    #[must_use]
    pub const fn phase(&self) -> TurnPhase {
        self.phase
    }

    /// Returns the monotonically increasing state revision.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Moves the turn into a new phase and returns the observed transition.
    pub fn transition(&mut self, phase: TurnPhase) -> TurnTransition {
        let previous = self.phase;
        if previous != phase {
            self.phase = phase;
            self.revision += 1;
        }

        TurnTransition {
            current: self.phase,
            previous,
            revision: self.revision,
            turn_id: self.turn_id.clone(),
        }
    }
}

/// Description of a state transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnTransition {
    current: TurnPhase,
    previous: TurnPhase,
    revision: u64,
    turn_id: TurnId,
}

impl TurnTransition {
    /// Returns the turn id associated with this transition.
    #[must_use]
    pub const fn turn_id(&self) -> &TurnId {
        &self.turn_id
    }

    /// Returns the phase before the transition.
    #[must_use]
    pub const fn previous(&self) -> TurnPhase {
        self.previous
    }

    /// Returns the phase after the transition.
    #[must_use]
    pub const fn current(&self) -> TurnPhase {
        self.current
    }

    /// Returns the state revision after the transition.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn_id(value: u64) -> TurnId {
        TurnId::new(format!("turn-{value}"))
    }

    #[test]
    fn new_state_starts_queued_at_revision_zero() {
        let turn_id = turn_id(11);
        let state = TurnState::new(turn_id.clone());

        assert_eq!(&turn_id, state.turn_id());
        assert_eq!(TurnPhase::Queued, state.phase());
        assert_eq!(0, state.revision());
    }

    #[test]
    fn transition_records_phase_change_and_revision() {
        let turn_id = turn_id(13);
        let mut state = TurnState::new(turn_id.clone());

        let transition = state.transition(TurnPhase::Running);

        assert_eq!(&turn_id, transition.turn_id());
        assert_eq!(TurnPhase::Queued, transition.previous());
        assert_eq!(TurnPhase::Running, transition.current());
        assert_eq!(1, transition.revision());
        assert_eq!(TurnPhase::Running, state.phase());
        assert_eq!(1, state.revision());
    }

    #[test]
    fn transition_to_same_phase_keeps_revision() {
        let mut state = TurnState::new(turn_id(17));
        state.transition(TurnPhase::Running);

        let transition = state.transition(TurnPhase::Running);

        assert_eq!(TurnPhase::Running, transition.previous());
        assert_eq!(TurnPhase::Running, transition.current());
        assert_eq!(1, transition.revision());
        assert_eq!(1, state.revision());
    }
}
