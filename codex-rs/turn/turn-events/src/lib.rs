//! Event vocabulary emitted by turn orchestration.
//!
//! This crate translates neutral state and output data into append-only events.

#![forbid(unsafe_code)]

use codex_turn_api::TurnId;
use codex_turn_api::TurnOutput;
use codex_turn_state::TurnPhase;
use codex_turn_state::TurnTransition;

/// Event emitted while a turn is processed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnEvent {
    kind: TurnEventKind,
    turn_id: TurnId,
}

impl TurnEvent {
    /// Builds an event for a turn.
    #[must_use]
    pub fn new(turn_id: TurnId, kind: TurnEventKind) -> Self {
        Self { kind, turn_id }
    }

    /// Builds an event from a state transition.
    #[must_use]
    pub fn phase_changed(transition: TurnTransition) -> Self {
        Self {
            kind: TurnEventKind::PhaseChanged {
                current: transition.current(),
                previous: transition.previous(),
                revision: transition.revision(),
            },
            turn_id: transition.turn_id().clone(),
        }
    }

    /// Returns the turn id associated with this event.
    #[must_use]
    pub const fn turn_id(&self) -> &TurnId {
        &self.turn_id
    }

    /// Returns the event payload.
    #[must_use]
    pub const fn kind(&self) -> &TurnEventKind {
        &self.kind
    }
}

/// Event payload for a turn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TurnEventKind {
    Started,
    PhaseChanged {
        previous: TurnPhase,
        current: TurnPhase,
        revision: u64,
    },
    PolicyDeferred {
        reason: String,
    },
    PolicyRejected {
        reason: String,
    },
    ToolRequested {
        tool_name: String,
    },
    Message {
        content: String,
    },
    Finished {
        output: TurnOutput,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_turn_api::TurnStatus;
    use codex_turn_state::TurnState;

    fn turn_id(value: u64) -> TurnId {
        TurnId::new(format!("turn-{value}"))
    }

    #[test]
    fn new_event_preserves_turn_id_and_payload() {
        let turn_id = turn_id(19);
        let kind = TurnEventKind::PolicyDeferred {
            reason: "queued behind another turn".to_string(),
        };

        let event = TurnEvent::new(turn_id.clone(), kind.clone());

        assert_eq!(&turn_id, event.turn_id());
        assert_eq!(&kind, event.kind());
    }

    #[test]
    fn phase_changed_event_maps_transition_fields() {
        let turn_id = turn_id(23);
        let mut state = TurnState::new(turn_id.clone());
        let transition = state.transition(TurnPhase::Running);

        let event = TurnEvent::phase_changed(transition);

        assert_eq!(&turn_id, event.turn_id());
        assert_eq!(
            &TurnEventKind::PhaseChanged {
                previous: TurnPhase::Queued,
                current: TurnPhase::Running,
                revision: 1,
            },
            event.kind()
        );
    }

    #[test]
    fn finished_event_can_carry_turn_output_without_core_types() {
        let turn_id = turn_id(29);
        let output = TurnOutput::new(turn_id.clone(), TurnStatus::Succeeded, "done");
        let event = TurnEvent::new(
            turn_id,
            TurnEventKind::Finished {
                output: output.clone(),
            },
        );

        assert_eq!(
            &TurnEventKind::Finished { output },
            event.kind()
        );
    }
}
