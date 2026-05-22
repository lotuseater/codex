//! Public boundary for running turns.
//!
//! This crate defines the loop-facing request and result types without owning a
//! concrete executor, policy, or tool runtime.

#![forbid(unsafe_code)]

use codex_turn_api::TurnInput;
use codex_turn_api::TurnOutput;
use codex_turn_events::TurnEvent;
use codex_turn_state::TurnState;

/// Request accepted by a turn loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnLoopRequest {
    input: TurnInput,
}

impl TurnLoopRequest {
    /// Builds a loop request from turn input.
    #[must_use]
    pub fn new(input: TurnInput) -> Self {
        Self { input }
    }

    /// Returns the input for this request.
    #[must_use]
    pub const fn input(&self) -> &TurnInput {
        &self.input
    }

    /// Consumes the request and returns the input.
    #[must_use]
    pub fn into_input(self) -> TurnInput {
        self.input
    }
}

/// Result produced by a turn loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnLoopResult {
    events: Vec<TurnEvent>,
    final_state: TurnState,
    output: TurnOutput,
}

impl TurnLoopResult {
    /// Builds a loop result.
    #[must_use]
    pub fn new(output: TurnOutput, final_state: TurnState, events: Vec<TurnEvent>) -> Self {
        Self {
            events,
            final_state,
            output,
        }
    }

    /// Returns the output produced by the turn.
    #[must_use]
    pub const fn output(&self) -> &TurnOutput {
        &self.output
    }

    /// Returns the final state after execution.
    #[must_use]
    pub const fn final_state(&self) -> &TurnState {
        &self.final_state
    }

    /// Returns the events emitted while processing the turn.
    #[must_use]
    pub fn events(&self) -> &[TurnEvent] {
        &self.events
    }
}

/// Runs a single turn from request to terminal output.
///
/// Implementations own orchestration only. Policy, state mutation, event
/// emission, and tool invocation should stay behind their dedicated crates.
pub trait TurnLoop {
    fn run_turn(&mut self, request: TurnLoopRequest) -> TurnLoopResult;
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_turn_api::TurnId;
    use codex_turn_api::TurnStatus;
    use codex_turn_events::TurnEventKind;

    fn turn_id(value: u64) -> TurnId {
        TurnId::new(format!("turn-{value}"))
    }

    #[test]
    fn loop_request_preserves_and_releases_input() {
        let input = TurnInput::new(turn_id(47), "run loop");
        let request = TurnLoopRequest::new(input.clone());

        assert_eq!(&input, request.input());
        assert_eq!(input, request.into_input());
    }

    #[test]
    fn loop_result_preserves_output_final_state_and_events() {
        let turn_id = turn_id(53);
        let output = TurnOutput::new(turn_id.clone(), TurnStatus::Succeeded, "complete");
        let final_state = TurnState::new(turn_id.clone());
        let events = vec![TurnEvent::new(turn_id, TurnEventKind::Started)];

        let result = TurnLoopResult::new(output.clone(), final_state.clone(), events.clone());

        assert_eq!(&output, result.output());
        assert_eq!(&final_state, result.final_state());
        assert_eq!(events.as_slice(), result.events());
    }
}
