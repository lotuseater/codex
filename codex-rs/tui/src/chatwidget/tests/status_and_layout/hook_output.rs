use super::super::*;
use super::common::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn user_prompt_submit_app_server_hook_notifications_render_snapshot() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.handle_server_notification(
        ServerNotification::HookStarted(AppServerHookStartedNotification {
            thread_id: ThreadId::new().to_string(),
            turn_id: Some("turn-1".to_string()),
            run: AppServerHookRunSummary {
                id: "user-prompt-submit:0:/tmp/hooks.json".to_string(),
                event_name: AppServerHookEventName::UserPromptSubmit,
                handler_type: AppServerHookHandlerType::Command,
                execution_mode: AppServerHookExecutionMode::Sync,
                scope: AppServerHookScope::Turn,
                source_path: PathBuf::from(test_path_display("/tmp/hooks.json")).abs(),
                source: codex_app_server_protocol::HookSource::User,
                display_order: 0,
                status: AppServerHookRunStatus::Running,
                status_message: Some("checking go-workflow input policy".to_string()),
                started_at: 1,
                completed_at: None,
                duration_ms: None,
                entries: Vec::new(),
            },
        }),
        /*replay_kind*/ None,
    );
    chat.handle_server_notification(
        ServerNotification::HookCompleted(AppServerHookCompletedNotification {
            thread_id: ThreadId::new().to_string(),
            turn_id: Some("turn-1".to_string()),
            run: AppServerHookRunSummary {
                id: "user-prompt-submit:0:/tmp/hooks.json".to_string(),
                event_name: AppServerHookEventName::UserPromptSubmit,
                handler_type: AppServerHookHandlerType::Command,
                execution_mode: AppServerHookExecutionMode::Sync,
                scope: AppServerHookScope::Turn,
                source_path: PathBuf::from(test_path_display("/tmp/hooks.json")).abs(),
                source: codex_app_server_protocol::HookSource::User,
                display_order: 0,
                status: AppServerHookRunStatus::Stopped,
                status_message: Some("checking go-workflow input policy".to_string()),
                started_at: 1,
                completed_at: Some(11),
                duration_ms: Some(10),
                entries: vec![
                    AppServerHookOutputEntry {
                        kind: AppServerHookOutputEntryKind::Warning,
                        text: "go-workflow must start from PlanMode".to_string(),
                    },
                    AppServerHookOutputEntry {
                        kind: AppServerHookOutputEntryKind::Stop,
                        text: "prompt blocked".to_string(),
                    },
                ],
            },
        }),
        /*replay_kind*/ None,
    );

    let cells = drain_insert_history(&mut rx);
    let combined = cells
        .iter()
        .map(|lines| lines_to_single_string(lines))
        .collect::<String>();
    assert_chatwidget_snapshot!(
        "user_prompt_submit_app_server_hook_notifications_render_snapshot",
        combined
    );
    assert!(!chat.bottom_pane.status_indicator_visible());
}

#[tokio::test]
async fn pre_tool_use_hook_events_render_snapshot() {
    assert_hook_events_snapshot(
        codex_app_server_protocol::HookEventName::PreToolUse,
        "pre-tool-use:0:/tmp/hooks.json",
        "warming the shell",
        "pre_tool_use_hook_events_render_snapshot",
    )
    .await;
}

#[tokio::test]
async fn post_tool_use_hook_events_render_snapshot() {
    assert_hook_events_snapshot(
        codex_app_server_protocol::HookEventName::PostToolUse,
        "post-tool-use:0:/tmp/hooks.json",
        "warming the shell",
        "post_tool_use_hook_events_render_snapshot",
    )
    .await;
}

#[tokio::test]
async fn blocked_hooks_render_feedback_and_tool_hook_failures_stay_out_of_history() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    handle_hook_completed(
        &mut chat,
        hook_completed_run(
            "pre-tool-use:0:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::PreToolUse,
            codex_app_server_protocol::HookRunStatus::Blocked,
            vec![codex_app_server_protocol::HookOutputEntry {
                kind: codex_app_server_protocol::HookOutputEntryKind::Feedback,
                text: "run tests before touching the fixture".to_string(),
            }],
        ),
    );
    handle_hook_completed(
        &mut chat,
        hook_completed_run(
            "post-tool-use:1:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::PostToolUse,
            codex_app_server_protocol::HookRunStatus::Failed,
            vec![codex_app_server_protocol::HookOutputEntry {
                kind: codex_app_server_protocol::HookOutputEntryKind::Error,
                text: "hook exited with code 7".to_string(),
            }],
        ),
    );
    handle_hook_completed(
        &mut chat,
        hook_completed_run(
            "pre-tool-use:2:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::PreToolUse,
            codex_app_server_protocol::HookRunStatus::Failed,
            vec![codex_app_server_protocol::HookOutputEntry {
                kind: codex_app_server_protocol::HookOutputEntryKind::Error,
                text: "hook exited with code 1".to_string(),
            }],
        ),
    );

    let rendered = drain_insert_history(&mut rx)
        .iter()
        .map(|lines| lines_to_single_string(lines))
        .collect::<String>();
    assert_chatwidget_snapshot!("hook_blocked_failed_feedback_history_snapshot", rendered);
    assert!(
        rendered.contains(
            "PreToolUse hook (blocked)\n  feedback: run tests before touching the fixture"
        ),
        "expected blocked hook feedback: {rendered:?}"
    );
    assert!(
        !rendered.contains("PostToolUse hook (failed)")
            && !rendered.contains("PreToolUse hook (failed)"),
        "non-blocking tool-hook failures should stay out of conversation history: {rendered:?}"
    );
}

#[tokio::test]
async fn completed_hook_with_output_flushes_immediately() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    handle_hook_started(
        &mut chat,
        hook_started_run(
            "pre-tool-use:0:/tmp/hooks.json:tool-call-1",
            codex_app_server_protocol::HookEventName::PreToolUse,
            Some("checking command"),
        ),
    );
    reveal_running_hooks(&mut chat);
    let running_snapshot = hook_live_and_history_snapshot(&chat, "running", "");

    handle_hook_completed(
        &mut chat,
        hook_completed_run(
            "pre-tool-use:0:/tmp/hooks.json:tool-call-1",
            codex_app_server_protocol::HookEventName::PreToolUse,
            codex_app_server_protocol::HookRunStatus::Blocked,
            vec![codex_app_server_protocol::HookOutputEntry {
                kind: codex_app_server_protocol::HookOutputEntryKind::Feedback,
                text: "command blocked by policy".to_string(),
            }],
        ),
    );
    let history = drain_insert_history(&mut rx)
        .iter()
        .map(|lines| lines_to_single_string(lines))
        .collect::<String>();
    let completed_snapshot = hook_live_and_history_snapshot(&chat, "completed", &history);

    assert_chatwidget_snapshot!(
        "completed_hook_with_output_flushes_immediately_snapshot",
        format!("{running_snapshot}\n\n{completed_snapshot}")
    );
}

#[tokio::test]
async fn completed_hook_output_precedes_following_assistant_message() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    handle_hook_started(
        &mut chat,
        hook_started_run(
            "pre-tool-use:0:/tmp/hooks.json:tool-call-1",
            codex_app_server_protocol::HookEventName::PreToolUse,
            Some("checking command"),
        ),
    );
    reveal_running_hooks(&mut chat);

    handle_hook_completed(
        &mut chat,
        hook_completed_run(
            "pre-tool-use:0:/tmp/hooks.json:tool-call-1",
            codex_app_server_protocol::HookEventName::PreToolUse,
            codex_app_server_protocol::HookRunStatus::Blocked,
            vec![codex_app_server_protocol::HookOutputEntry {
                kind: codex_app_server_protocol::HookOutputEntryKind::Feedback,
                text: "command blocked by policy".to_string(),
            }],
        ),
    );

    complete_assistant_message(
        &mut chat,
        "msg-after-hook",
        "The hook feedback was applied.",
        /*phase*/ None,
    );

    let history = drain_insert_history(&mut rx)
        .iter()
        .map(|lines| lines_to_single_string(lines))
        .collect::<String>();
    assert_chatwidget_snapshot!(
        "completed_hook_output_precedes_following_assistant_message_snapshot",
        format!(
            "active hooks:\n{}history:\n{history}",
            active_hook_blob(&chat)
        )
    );
    let hook_index = history
        .find("PreToolUse hook (blocked)")
        .expect("hook feedback should be in history");
    let assistant_index = history
        .find("The hook feedback was applied.")
        .expect("assistant message should be in history");
    assert!(
        hook_index < assistant_index,
        "hook output should precede later assistant text: {history:?}"
    );
}

#[tokio::test]
async fn completed_same_id_hook_output_survives_restart() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let hook_id = "stop:0:/tmp/hooks.json";

    handle_hook_started(
        &mut chat,
        hook_started_run(
            hook_id,
            codex_app_server_protocol::HookEventName::Stop,
            Some("checking stop condition"),
        ),
    );
    reveal_running_hooks(&mut chat);
    handle_hook_completed(
        &mut chat,
        hook_completed_run(
            hook_id,
            codex_app_server_protocol::HookEventName::Stop,
            codex_app_server_protocol::HookRunStatus::Stopped,
            vec![codex_app_server_protocol::HookOutputEntry {
                kind: codex_app_server_protocol::HookOutputEntryKind::Stop,
                text: "continue with more context".to_string(),
            }],
        ),
    );
    handle_hook_started(
        &mut chat,
        hook_started_run(
            hook_id,
            codex_app_server_protocol::HookEventName::Stop,
            Some("checking stop condition"),
        ),
    );
    reveal_running_hooks(&mut chat);

    let history = drain_insert_history(&mut rx)
        .iter()
        .map(|lines| lines_to_single_string(lines))
        .collect::<String>();
    assert_chatwidget_snapshot!(
        "completed_same_id_hook_output_survives_restart_snapshot",
        format!(
            "active hooks:\n{}history:\n{history}",
            active_hook_blob(&chat)
        )
    );
    assert!(
        history.contains("Stop hook (stopped)\n  stop: continue with more context"),
        "first hook output should not be overwritten: {history:?}"
    );
}

#[tokio::test]
async fn identical_parallel_running_hooks_collapse_to_count() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    for tool_call_id in ["tool-call-1", "tool-call-2", "tool-call-3"] {
        handle_hook_started(
            &mut chat,
            hook_started_run(
                &format!("pre-tool-use:0:/tmp/hooks.json:{tool_call_id}"),
                codex_app_server_protocol::HookEventName::PreToolUse,
                Some("checking command policy"),
            ),
        );
    }
    reveal_running_hooks(&mut chat);

    assert_chatwidget_snapshot!(
        "identical_parallel_running_hooks_collapse_to_count_snapshot",
        hook_live_and_history_snapshot(&chat, "running", "")
    );
}

#[tokio::test]
async fn overlapping_hook_live_cell_tracks_parallel_quiet_hooks() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.set_status_header("Thinking".to_string());
    chat.bottom_pane.ensure_status_indicator();

    handle_hook_started(
        &mut chat,
        hook_started_run(
            "pre-tool-use:0:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::PreToolUse,
            Some("checking command policy"),
        ),
    );
    assert_eq!(chat.status_state.current_status.header, "Thinking");
    reveal_running_hooks(&mut chat);
    let first_running_snapshot = hook_live_and_history_snapshot(&chat, "pre running", "");

    handle_hook_started(
        &mut chat,
        hook_started_run(
            "post-tool-use:1:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::PostToolUse,
            Some("checking output policy"),
        ),
    );
    assert_eq!(chat.status_state.current_status.header, "Thinking");
    reveal_running_hooks(&mut chat);
    let second_running_snapshot = hook_live_and_history_snapshot(&chat, "post running", "");

    handle_hook_completed(
        &mut chat,
        hook_completed_run(
            "pre-tool-use:0:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::PreToolUse,
            codex_app_server_protocol::HookRunStatus::Completed,
            Vec::new(),
        ),
    );
    assert_eq!(chat.status_state.current_status.header, "Thinking");
    let older_completed_snapshot =
        hook_live_and_history_snapshot(&chat, "pre completed lingering", "");
    expire_quiet_hook_linger(&mut chat);
    let older_completed_expired_snapshot =
        hook_live_and_history_snapshot(&chat, "pre completed after linger", "");

    handle_hook_completed(
        &mut chat,
        hook_completed_run(
            "post-tool-use:1:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::PostToolUse,
            codex_app_server_protocol::HookRunStatus::Completed,
            Vec::new(),
        ),
    );
    assert_eq!(chat.status_state.current_status.header, "Thinking");
    assert!(chat.bottom_pane.status_indicator_visible());
    assert!(drain_insert_history(&mut rx).is_empty());
    let all_completed_lingering_snapshot =
        hook_live_and_history_snapshot(&chat, "all completed lingering", "");
    expire_quiet_hook_linger(&mut chat);
    let all_completed_snapshot = hook_live_and_history_snapshot(&chat, "all completed", "");
    assert_chatwidget_snapshot!(
        "overlapping_hook_live_cell_snapshot",
        format!(
            "{first_running_snapshot}\n\n{second_running_snapshot}\n\n{older_completed_snapshot}\n\n{older_completed_expired_snapshot}\n\n{all_completed_lingering_snapshot}\n\n{all_completed_snapshot}"
        )
    );
}

#[tokio::test]
async fn session_start_hook_events_render_snapshot() {
    assert_hook_events_snapshot(
        codex_app_server_protocol::HookEventName::SessionStart,
        "session-start:0:/tmp/hooks.json",
        "warming the shell",
        "session_start_hook_events_render_snapshot",
    )
    .await;
}
