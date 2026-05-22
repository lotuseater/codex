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
        let turn_id = input.turn_id().clone();
        let mut state = TurnState::new(turn_id.clone());
        let mut events = vec![TurnEvent::new(turn_id.clone(), TurnEventKind::Started)];

        match self
            .policy
            .evaluate(&TurnPolicyContext::new(&input, &state))
        {
            TurnPolicyDecision::Allow => {
                let transition = state.transition(TurnPhase::Running);
                events.push(TurnEvent::phase_changed(transition));

                let transition = state.transition(TurnPhase::Completed);
                events.push(TurnEvent::phase_changed(transition));

                let output =
                    TurnOutput::new(turn_id.clone(), TurnStatus::Succeeded, input.prompt());
                events.push(TurnEvent::new(
                    turn_id.clone(),
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
                    turn_id.clone(),
                    TurnEventKind::PolicyDeferred {
                        reason: reason.clone(),
                    },
                ));

                let output = TurnOutput::new(turn_id.clone(), TurnStatus::Deferred, reason);
                TurnLoopResult::new(output, state, events)
            }
            TurnPolicyDecision::Reject { reason } => {
                let transition = state.transition(TurnPhase::Failed);
                events.push(TurnEvent::phase_changed(transition));
                events.push(TurnEvent::new(
                    turn_id.clone(),
                    TurnEventKind::PolicyRejected {
                        reason: reason.clone(),
                    },
                ));

                let output = TurnOutput::new(turn_id.clone(), TurnStatus::Rejected, reason);
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

#[cfg(test)]
mod tests {
    use super::*;
    use codex_turn_api::TurnId;
    use codex_turn_events::TurnEventKind;
    use codex_turn_policy::TurnPolicyContext;

    fn turn_id(value: u64) -> TurnId {
        TurnId::new(format!("turn-{value}"))
    }

    fn request(turn_id: TurnId, prompt: &str) -> TurnLoopRequest {
        TurnLoopRequest::new(TurnInput::new(turn_id, prompt))
    }

    struct FixedPolicy {
        decision: TurnPolicyDecision,
    }

    impl TurnPolicy for FixedPolicy {
        fn evaluate(&self, _context: &TurnPolicyContext<'_>) -> TurnPolicyDecision {
            self.decision.clone()
        }
    }

    #[test]
    fn allow_flow_completes_turn_and_emits_realtime_events() {
        let turn_id = turn_id(59);
        let mut turn_loop = BasicTurnLoop::default();

        let result = turn_loop.run_turn(request(turn_id, "ship it"));

        assert_eq!(TurnStatus::Succeeded, result.output().status());
        assert_eq!("ship it", result.output().message());
        assert_eq!(TurnPhase::Completed, result.final_state().phase());
        assert_eq!(2, result.final_state().revision());
        assert_eq!(4, result.events().len());
        assert_eq!(&TurnEventKind::Started, result.events()[0].kind());
        assert_eq!(
            &TurnEventKind::PhaseChanged {
                previous: TurnPhase::Queued,
                current: TurnPhase::Running,
                revision: 1,
            },
            result.events()[1].kind()
        );
        assert_eq!(
            &TurnEventKind::PhaseChanged {
                previous: TurnPhase::Running,
                current: TurnPhase::Completed,
                revision: 2,
            },
            result.events()[2].kind()
        );
        assert_eq!(
            &TurnEventKind::Finished {
                output: result.output().clone(),
            },
            result.events()[3].kind()
        );
    }

    #[test]
    fn defer_flow_waits_with_policy_reason() {
        let turn_id = turn_id(61);
        let policy = FixedPolicy {
            decision: TurnPolicyDecision::Defer {
                reason: "another turn is running".to_string(),
            },
        };
        let mut turn_loop = BasicTurnLoop::new(policy, NoopToolBridge);

        let result = turn_loop.run_turn(request(turn_id, "later"));

        assert_eq!(TurnStatus::Deferred, result.output().status());
        assert_eq!("another turn is running", result.output().message());
        assert_eq!(TurnPhase::Waiting, result.final_state().phase());
        assert_eq!(1, result.final_state().revision());
        assert_eq!(3, result.events().len());
        assert_eq!(
            &TurnEventKind::PolicyDeferred {
                reason: "another turn is running".to_string(),
            },
            result.events()[2].kind()
        );
    }

    #[test]
    fn reject_flow_fails_with_policy_reason() {
        let turn_id = turn_id(67);
        let policy = FixedPolicy {
            decision: TurnPolicyDecision::Reject {
                reason: "blocked by policy".to_string(),
            },
        };
        let mut turn_loop = BasicTurnLoop::new(policy, NoopToolBridge);

        let result = turn_loop.run_turn(request(turn_id, "stop"));

        assert_eq!(TurnStatus::Rejected, result.output().status());
        assert_eq!("blocked by policy", result.output().message());
        assert_eq!(TurnPhase::Failed, result.final_state().phase());
        assert_eq!(1, result.final_state().revision());
        assert_eq!(3, result.events().len());
        assert_eq!(
            &TurnEventKind::PolicyRejected {
                reason: "blocked by policy".to_string(),
            },
            result.events()[2].kind()
        );
    }

    #[test]
    fn noop_tool_bridge_reports_requested_tool_unavailable() {
        let turn_id = turn_id(71);
        let mut bridge = NoopToolBridge;

        let error = bridge
            .invoke(ToolRequest::new(turn_id, "shell", "{}"))
            .expect_err("noop bridge should not invoke tools");

        assert_eq!(
            ToolBridgeError::Unavailable {
                tool_name: "shell".to_string(),
            },
            error
        );
    }
}
