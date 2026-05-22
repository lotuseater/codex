use codex_session_api::SessionLifecycleState;
use codex_session_input::SessionInput;
use codex_session_runtime_api::BackgroundTaskLifecycle;
use codex_session_runtime_api::BackgroundTaskState;
use codex_session_runtime_api::SessionRuntimeCommand;
use codex_session_runtime_api::SessionRuntimeCommandDecision;
use codex_session_runtime_api::SessionRuntimeLifecycle;

fn submit_input_command() -> SessionRuntimeCommand {
    SessionRuntimeCommand::SubmitInput {
        input: SessionInput::UserText {
            text: "hello".to_string(),
        },
    }
}

#[test]
fn shutdown_drains_session_and_rejects_new_input() {
    let mut lifecycle = SessionRuntimeLifecycle::created();

    assert_eq!(
        lifecycle.command_decision(&submit_input_command()),
        SessionRuntimeCommandDecision::Accepted
    );
    assert_eq!(lifecycle.mark_active().current(), SessionLifecycleState::Active);

    let shutdown = lifecycle.request_shutdown();

    assert!(shutdown.changed());
    assert_eq!(shutdown.previous(), SessionLifecycleState::Active);
    assert_eq!(shutdown.current(), SessionLifecycleState::Draining);
    assert_eq!(
        lifecycle.command_decision(&submit_input_command()),
        SessionRuntimeCommandDecision::Rejected(SessionLifecycleState::Draining)
    );
    assert!(!lifecycle.request_shutdown().changed());

    let completed = lifecycle.mark_completed();

    assert!(completed.changed());
    assert_eq!(completed.current(), SessionLifecycleState::Completed);
    assert!(!lifecycle.mark_failed().changed());
}

#[test]
fn background_task_abort_is_idempotent_until_finished() {
    let mut lifecycle = BackgroundTaskLifecycle::new();

    assert!(lifecycle.can_start());
    assert_eq!(lifecycle.start().current(), BackgroundTaskState::Running);
    assert!(!lifecycle.can_start());

    let aborting = lifecycle.request_abort();

    assert!(aborting.changed());
    assert_eq!(aborting.previous(), BackgroundTaskState::Running);
    assert_eq!(aborting.current(), BackgroundTaskState::Aborting);
    assert!(!lifecycle.request_abort().changed());

    assert_eq!(lifecycle.finish().current(), BackgroundTaskState::Finished);
    assert!(lifecycle.can_start());
}
