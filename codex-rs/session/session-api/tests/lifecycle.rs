use codex_session_api::SessionLifecycleState;

#[test]
fn lifecycle_predicates_identify_input_and_terminal_states() {
    assert!(SessionLifecycleState::Created.accepts_input());
    assert!(SessionLifecycleState::Active.accepts_input());
    assert!(!SessionLifecycleState::Draining.accepts_input());
    assert!(!SessionLifecycleState::Completed.accepts_input());
    assert!(!SessionLifecycleState::Failed.accepts_input());

    assert!(!SessionLifecycleState::Created.is_terminal());
    assert!(!SessionLifecycleState::Active.is_terminal());
    assert!(!SessionLifecycleState::Draining.is_terminal());
    assert!(SessionLifecycleState::Completed.is_terminal());
    assert!(SessionLifecycleState::Failed.is_terminal());
}
