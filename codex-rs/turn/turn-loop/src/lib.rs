//! Minimal turn loop adapter.
//!
//! This crate composes the turn-specific policy, state, event, and tool
//! boundaries without depending on the existing core runtime.

#![forbid(unsafe_code)]

use codex_turn_api::TurnOutput;
use codex_turn_api::TurnStatus;
use codex_turn_events::TurnEvent;
use codex_turn_events::TurnEventKind;
use codex_turn_loop_api::TurnLoop;
use codex_turn_loop_api::TurnLoopRequest;
use codex_turn_loop_api::TurnLoopResult;
use codex_turn_policy::AllowAllPolicy;
use codex_turn_policy::TurnPolicy;
use codex_turn_policy::TurnPolicyContext;
use codex_turn_policy::TurnPolicyDecision;
use codex_turn_state::TurnPhase;
use codex_turn_state::TurnState;
use codex_turn_tool_bridge::ToolBridgeError;
use codex_turn_tool_bridge::ToolBridgeResult;
use codex_turn_tool_bridge::ToolRequest;
use codex_turn_tool_bridge::TurnToolBridge;

/// Basic turn loop implementation for early integration.
#[derive(Clone, Debug)]
pub struct BasicTurnLoop<P = AllowAllPolicy, B = NoopToolBridge> {
    policy: P,
    tool_bridge: B,
}

impl Default for BasicTurnLoop<AllowAllPolicy, NoopToolBridge> {
    fn default() -> Self {
        Self {
            policy: AllowAllPolicy,
            tool_bridge: NoopToolBridge,
        }
    }
}

impl<P, B> BasicTurnLoop<P, B> {
    /// Creates a turn loop from explicit dependencies.
    #[must_use]
    pub const fn new(policy: P, tool_bridge: B) -> Self {
        Self {
            policy,
            tool_bridge,
        }
    }

    /// Returns the policy used by this loop.
    #[must_use]
    pub const fn policy(&self) -> &P {
        &self.policy
    }

    /// Returns the tool bridge used by this loop.
    #[must_use]
    pub const fn tool_bridge(&self) -> &B {
        &self.tool_bridge
    }

    /// Returns the mutable tool bridge used by this loop.
    #[must_use]
    pub const fn tool_bridge_mut(&mut self) -> &mut B {
        &mut self.tool_bridge
    }
}

impl<P, B> TurnLoop for BasicTurnLoop<P, B>
where
    P: TurnPolicy,
    B: TurnToolBridge,
{
    fn run_turn(&mut self, request: TurnLoopRequest) -> TurnLoopResult {
        let input = request.into_input();
        let turn_id = input.turn_id();
        let mut state = TurnState::new(turn_id);
        let mut events = vec![TurnEvent::new(turn_id, TurnEventKind::Started)];

        match self
            .policy
            .evaluate(&TurnPolicyContext::new(&input, &state))
        {
            TurnPolicyDecision::Allow => {
                let transition = state.transition(TurnPhase::Running);
                events.push(TurnEvent::phase_changed(transition));

                let transition = state.transition(TurnPhase::Completed);
                events.push(TurnEvent::phase_changed(transition));

                let output = TurnOutput::new(turn_id, TurnStatus::Succeeded, input.prompt());
                events.push(TurnEvent::new(
                    turn_id,
                    TurnEventKind::Finished {
                        output: output.clone(),
                    },
                ));

                TurnLoopResult::new(output, state, events)
            }
            TurnPolicyDecision::Defer { reason } => {
                let transition = state.transition(TurnPhase::Waiting);
                events.push(TurnEvent::phase_changed(transition));
                events.push(TurnEvent::new(
                    turn_id,
                    TurnEventKind::PolicyDeferred {
                        reason: reason.clone(),
                    },
                ));

                let output = TurnOutput::new(turn_id, TurnStatus::Deferred, reason);
                TurnLoopResult::new(output, state, events)
            }
            TurnPolicyDecision::Reject { reason } => {
                let transition = state.transition(TurnPhase::Failed);
                events.push(TurnEvent::phase_changed(transition));
                events.push(TurnEvent::new(
                    turn_id,
                    TurnEventKind::PolicyRejected {
                        reason: reason.clone(),
                    },
                ));

                let output = TurnOutput::new(turn_id, TurnStatus::Rejected, reason);
                TurnLoopResult::new(output, state, events)
            }
        }
    }
}

/// Tool bridge that reports every tool as unavailable.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopToolBridge;

impl TurnToolBridge for NoopToolBridge {
    fn invoke(&mut self, request: ToolRequest) -> ToolBridgeResult {
        Err(ToolBridgeError::Unavailable {
            tool_name: request.name().to_string(),
        })
    }
}
