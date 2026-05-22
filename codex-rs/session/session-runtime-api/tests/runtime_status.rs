use codex_session_api::SessionDescriptor;
use codex_session_api::SessionIdentity;
use codex_session_api::SessionLifecycleState;
use codex_session_runtime_api::SessionRuntimeStatus;

fn identity() -> SessionIdentity {
    SessionIdentity::new(
        "67e55044-10b1-426f-9247-bb680e5fe0c7"
            .try_into()
            .expect("session id parses"),
    )
}

#[test]
fn runtime_status_projects_descriptor_and_lifecycle_predicates() {
    let identity = identity();
    let status = SessionRuntimeStatus::new(identity.clone(), SessionLifecycleState::Active);

    assert_eq!(
        status.descriptor(),
        SessionDescriptor::new(identity, SessionLifecycleState::Active)
    );
    assert!(status.accepts_input());
    assert!(!status.is_terminal());
}

#[test]
fn runtime_status_reports_terminal_lifecycles() {
    let failed = SessionRuntimeStatus::new(identity(), SessionLifecycleState::Failed);
    assert!(!failed.accepts_input());
    assert!(failed.is_terminal());
}
