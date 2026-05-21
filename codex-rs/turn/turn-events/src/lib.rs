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
    pub const fn new(turn_id: TurnId, kind: TurnEventKind) -> Self {
        Self { kind, turn_id }
    }

    /// Builds an event from a state transition.
    #[must_use]
    pub const fn phase_changed(transition: TurnTransition) -> Self {
        Self {
            kind: TurnEventKind::PhaseChanged {
                current: transition.current(),
                previous: transition.previous(),
                revision: transition.revision(),
            },
            turn_id: transition.turn_id(),
        }
    }

    /// Returns the turn id associated with this event.
    #[must_use]
    pub const fn turn_id(&self) -> TurnId {
        self.turn_id
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
