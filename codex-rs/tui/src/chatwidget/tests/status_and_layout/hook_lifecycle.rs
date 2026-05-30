use super::common::*;
use super::super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn interrupted_turn_clears_visible_running_hook() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    handle_hook_started(
        &mut chat,
        hook_started_run(
            "pre-tool-use:0:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::PreToolUse,
            Some("checking command policy"),
        ),
    );
    reveal_running_hooks(&mut chat);
    let before_interrupt = active_hook_blob(&chat);

    handle_turn_interrupted(&mut chat, "turn-1");

    assert_chatwidget_snapshot!(
        "interrupted_turn_clears_visible_running_hook",
        format!(
            "before interrupt:\n{before_interrupt}after interrupt:\n{}",
            active_hook_blob(&chat)
        )
    );
}

#[tokio::test]
async fn completed_hook_with_no_entries_stays_out_of_history() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    handle_hook_started(
        &mut chat,
        hook_started_run(
            "post-tool-use:0:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::PostToolUse,
            /*status_message*/ None,
        ),
    );
    assert!(drain_insert_history(&mut rx).is_empty());
    reveal_running_hooks(&mut chat);
    let running_snapshot = hook_live_and_history_snapshot(&chat, "running", "");

    handle_hook_completed(
        &mut chat,
        hook_completed_run(
            "post-tool-use:0:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::PostToolUse,
            codex_app_server_protocol::HookRunStatus::Completed,
            Vec::new(),
        ),
    );

    assert!(drain_insert_history(&mut rx).is_empty());
    let completed_lingering_snapshot =
        hook_live_and_history_snapshot(&chat, "completed lingering", "");
    expire_quiet_hook_linger(&mut chat);
    let completed_snapshot = hook_live_and_history_snapshot(&chat, "completed after linger", "");
    assert_chatwidget_snapshot!(
        "hook_live_running_then_quiet_completed_snapshot",
        format!("{running_snapshot}\n\n{completed_lingering_snapshot}\n\n{completed_snapshot}")
    );
}

#[tokio::test]
async fn quiet_hook_linger_starts_when_delayed_redraw_reveals_hook() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    handle_hook_started(
        &mut chat,
        hook_started_run(
            "post-tool-use:0:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::PostToolUse,
            Some("checking output policy"),
        ),
    );
    assert!(drain_insert_history(&mut rx).is_empty());

    reveal_running_hooks_after_delayed_redraw(&mut chat);
    handle_hook_completed(
        &mut chat,
        hook_completed_run(
            "post-tool-use:0:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::PostToolUse,
            codex_app_server_protocol::HookRunStatus::Completed,
            Vec::new(),
        ),
    );

    assert!(drain_insert_history(&mut rx).is_empty());
    assert!(
        active_hook_blob(&chat).contains("Running PostToolUse hook"),
        "quiet hook should linger after the row becomes visible"
    );
    expire_quiet_hook_linger(&mut chat);
    assert_eq!(active_hook_blob(&chat), "<empty>\n");
}

#[tokio::test]
async fn turn_completed_clears_orphaned_running_hook() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    handle_turn_started(&mut chat, "turn-1");
    handle_hook_started(
        &mut chat,
        hook_started_run(
            "pre-tool-use:0:/tmp/hooks.json:call-1",
            codex_app_server_protocol::HookEventName::PreToolUse,
            Some("checking input"),
        ),
    );
    reveal_running_hooks(&mut chat);
    assert!(active_hook_blob(&chat).contains("Running PreToolUse hook"));

    handle_turn_completed(&mut chat, "turn-1", /*duration_ms*/ None);

    assert_eq!(active_hook_blob(&chat), "<empty>\n");
}

#[tokio::test]
async fn hidden_active_hook_does_not_add_transcript_separator() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    begin_exec(&mut chat, "call-1", "echo done");
    let exec_only_line_count = chat
        .active_cell_transcript_lines(/*width*/ 80)
        .expect("active exec transcript lines")
        .len();

    handle_hook_started(
        &mut chat,
        hook_started_run(
            "post-tool-use:0:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::PostToolUse,
            Some("checking output policy"),
        ),
    );
    let hidden_hook_transcript = chat
        .active_cell_transcript_lines(/*width*/ 80)
        .expect("active exec transcript lines");
    assert_eq!(hidden_hook_transcript.len(), exec_only_line_count);

    reveal_running_hooks(&mut chat);
    let visible_hook_lines = chat
        .active_hook_cell
        .as_ref()
        .expect("active hook cell")
        .transcript_lines(/*width*/ 80);
    let visible_hook_transcript = chat
        .active_cell_transcript_lines(/*width*/ 80)
        .expect("active exec and hook transcript lines");
    assert_eq!(
        visible_hook_transcript.len(),
        exec_only_line_count + 1 + visible_hook_lines.len()
    );
    assert_eq!(
        lines_to_single_string(
            &visible_hook_transcript[exec_only_line_count..exec_only_line_count + 1],
        ),
        "\n"
    );
}

#[tokio::test]
async fn hook_completed_before_reveal_renders_completed_without_running_flash() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    handle_hook_started(
        &mut chat,
        hook_started_run(
            "session-start:0:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::SessionStart,
            Some("warming the shell"),
        ),
    );
    let started_hidden_snapshot = active_hook_blob(&chat);

    handle_hook_completed(
        &mut chat,
        hook_completed_run(
            "session-start:0:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::SessionStart,
            codex_app_server_protocol::HookRunStatus::Completed,
            vec![codex_app_server_protocol::HookOutputEntry {
                kind: codex_app_server_protocol::HookOutputEntryKind::Context,
                text: "session context".to_string(),
            }],
        ),
    );

    let history = drain_insert_history(&mut rx)
        .iter()
        .map(|lines| lines_to_single_string(lines))
        .collect::<String>();
    assert_chatwidget_snapshot!(
        "hook_completed_before_reveal_renders_completed_without_running_flash_snapshot",
        format!("started hidden:\n{started_hidden_snapshot}\nhistory:\n{history}")
    );
}

#[tokio::test]
async fn running_hook_does_not_displace_active_exec_cell() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    let begin = begin_exec(&mut chat, "call-1", "echo done");
    let exec_running = active_blob(&chat);

    handle_hook_started(
        &mut chat,
        hook_started_run(
            "post-tool-use:0:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::PostToolUse,
            Some("checking output policy"),
        ),
    );
    reveal_running_hooks(&mut chat);
    let exec_and_hook_running = format!(
        "active exec:\n{}active hooks:\n{}",
        active_blob(&chat),
        active_hook_blob(&chat)
    );

    end_exec(&mut chat, begin, "done", "", /*exit_code*/ 0);
    let history_after_exec = drain_insert_history(&mut rx)
        .iter()
        .map(|lines| lines_to_single_string(lines))
        .collect::<String>();
    let hook_running_after_exec = active_hook_blob(&chat);

    handle_hook_completed(
        &mut chat,
        hook_completed_run(
            "post-tool-use:0:/tmp/hooks.json",
            codex_app_server_protocol::HookEventName::PostToolUse,
            codex_app_server_protocol::HookRunStatus::Completed,
            Vec::new(),
        ),
    );
    assert!(drain_insert_history(&mut rx).is_empty());
    let quiet_hook_completed_lingering = active_hook_blob(&chat);
    expire_quiet_hook_linger(&mut chat);
    let quiet_hook_completed = active_hook_blob(&chat);

    assert_chatwidget_snapshot!(
        "hook_runs_while_exec_active_snapshot",
        format!(
            "exec running:\n{exec_running}\nexec and hook running:\n{exec_and_hook_running}\nhistory after exec:\n{history_after_exec}\nhook running after exec:\n{hook_running_after_exec}\nquiet hook completed lingering:\n{quiet_hook_completed_lingering}\nquiet hook completed:\n{quiet_hook_completed}"
        )
    );
}
