#![allow(dead_code, unused_imports, clippy::expect_used, clippy::unwrap_used)]

include!("code_mode_shared.rs");
use pretty_assertions::assert_eq;
use pretty_assertions::assert_ne;

#[cfg_attr(windows, ignore = "no exec_command on Windows")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_yield_and_resume_with_wait() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let mut builder = test_codex().with_config(move |config| {
        let _ = config.features.enable(Feature::CodeMode);
    });
    let test = builder.build(&server).await?;
    let phase_2_gate = test.workspace_path("code-mode-phase-2.ready");
    let phase_3_gate = test.workspace_path("code-mode-phase-3.ready");
    let phase_2_wait = wait_for_file_source(&phase_2_gate)?;
    let phase_3_wait = wait_for_file_source(&phase_3_gate)?;

    let code = format!(
        r#"
text("phase 1");
yield_control();
{phase_2_wait}
text("phase 2");
{phase_3_wait}
text("phase 3");
"#
    );

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_custom_tool_call("call-1", "exec", &code),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let first_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "waiting"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    test.submit_turn("start the long exec").await?;

    let first_request = first_completion.single_request();
    let first_items = custom_tool_output_items(&first_request, "call-1");
    assert_eq!(first_items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script running with cell ID \d+\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&first_items, /*index*/ 0),
    );
    assert_eq!(text_item(&first_items, /*index*/ 1), "phase 1");
    let cell_id = extract_running_cell_id(text_item(&first_items, /*index*/ 0));

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-3"),
            responses::ev_function_call(
                "call-2",
                "wait",
                &serde_json::to_string(&serde_json::json!({
                    "cell_id": cell_id.clone(),
                    "yield_time_ms": 1_000,
                }))?,
            ),
            ev_completed("resp-3"),
        ]),
    )
    .await;
    let second_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-2", "still waiting"),
            ev_completed("resp-4"),
        ]),
    )
    .await;

    fs::write(&phase_2_gate, "ready")?;
    test.submit_turn("wait again").await?;

    let second_request = second_completion.single_request();
    let second_items = function_tool_output_items(&second_request, "call-2");
    assert_eq!(second_items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script running with cell ID \d+\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&second_items, /*index*/ 0),
    );
    assert_eq!(
        extract_running_cell_id(text_item(&second_items, /*index*/ 0)),
        cell_id
    );
    assert_eq!(text_item(&second_items, /*index*/ 1), "phase 2");

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-5"),
            responses::ev_function_call(
                "call-3",
                "wait",
                &serde_json::to_string(&serde_json::json!({
                    "cell_id": cell_id.clone(),
                    "yield_time_ms": 1_000,
                }))?,
            ),
            ev_completed("resp-5"),
        ]),
    )
    .await;
    let third_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-3", "done"),
            ev_completed("resp-6"),
        ]),
    )
    .await;

    fs::write(&phase_3_gate, "ready")?;
    test.submit_turn("wait for completion").await?;

    let third_request = third_completion.single_request();
    let third_items = function_tool_output_items(&third_request, "call-3");
    assert_eq!(third_items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script completed\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&third_items, /*index*/ 0),
    );
    assert_eq!(text_item(&third_items, /*index*/ 1), "phase 3");

    Ok(())
}

#[cfg_attr(windows, ignore = "no exec_command on Windows")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_yield_timeout_works_for_busy_loop() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let mut builder = test_codex().with_config(move |config| {
        let _ = config.features.enable(Feature::CodeMode);
    });
    let test = builder.build(&server).await?;

    let code = r#"// @exec: {"yield_time_ms": 100}
text("phase 1");
while (true) {}
"#;

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_custom_tool_call("call-1", "exec", code),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let first_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "waiting"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    tokio::time::timeout(
        Duration::from_secs(5),
        test.submit_turn("start the busy loop"),
    )
    .await??;

    let first_request = first_completion.single_request();
    let first_items = custom_tool_output_items(&first_request, "call-1");
    assert_eq!(first_items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script running with cell ID \d+\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&first_items, /*index*/ 0),
    );
    assert_eq!(text_item(&first_items, /*index*/ 1), "phase 1");
    let cell_id = extract_running_cell_id(text_item(&first_items, /*index*/ 0));

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-3"),
            responses::ev_function_call(
                "call-2",
                "wait",
                &serde_json::to_string(&serde_json::json!({
                    "cell_id": cell_id.clone(),
                    "terminate": true,
                }))?,
            ),
            ev_completed("resp-3"),
        ]),
    )
    .await;
    let second_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-2", "terminated"),
            ev_completed("resp-4"),
        ]),
    )
    .await;

    test.submit_turn("terminate it").await?;

    let second_request = second_completion.single_request();
    let second_items = function_tool_output_items(&second_request, "call-2");
    assert_eq!(second_items.len(), 1);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script terminated\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&second_items, /*index*/ 0),
    );

    Ok(())
}

#[cfg_attr(windows, ignore = "no exec_command on Windows")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_run_multiple_yielded_sessions() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let mut builder = test_codex().with_config(move |config| {
        let _ = config.features.enable(Feature::CodeMode);
    });
    let test = builder.build(&server).await?;
    let session_a_gate = test.workspace_path("code-mode-session-a.ready");
    let session_b_gate = test.workspace_path("code-mode-session-b.ready");
    let session_a_wait = wait_for_file_source(&session_a_gate)?;
    let session_b_wait = wait_for_file_source(&session_b_gate)?;

    let session_a_code = format!(
        r#"
text("session a start");
yield_control();
{session_a_wait}
text("session a done");
"#
    );
    let session_b_code = format!(
        r#"
text("session b start");
yield_control();
{session_b_wait}
text("session b done");
"#
    );

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_custom_tool_call("call-1", "exec", &session_a_code),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let first_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "session a waiting"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    test.submit_turn("start session a").await?;

    let first_request = first_completion.single_request();
    let first_items = custom_tool_output_items(&first_request, "call-1");
    assert_eq!(first_items.len(), 2);
    let session_a_id = extract_running_cell_id(text_item(&first_items, /*index*/ 0));
    assert_eq!(text_item(&first_items, /*index*/ 1), "session a start");

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-3"),
            ev_custom_tool_call("call-2", "exec", &session_b_code),
            ev_completed("resp-3"),
        ]),
    )
    .await;
    let second_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-2", "session b waiting"),
            ev_completed("resp-4"),
        ]),
    )
    .await;

    test.submit_turn("start session b").await?;

    let second_request = second_completion.single_request();
    let second_items = custom_tool_output_items(&second_request, "call-2");
    assert_eq!(second_items.len(), 2);
    let session_b_id = extract_running_cell_id(text_item(&second_items, /*index*/ 0));
    assert_eq!(text_item(&second_items, /*index*/ 1), "session b start");
    assert_ne!(session_a_id, session_b_id);

    fs::write(&session_a_gate, "ready")?;
    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-5"),
            responses::ev_function_call(
                "call-3",
                "wait",
                &serde_json::to_string(&serde_json::json!({
                    "cell_id": session_a_id.clone(),
                    "yield_time_ms": 1_000,
                }))?,
            ),
            ev_completed("resp-5"),
        ]),
    )
    .await;
    let third_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-3", "session a done"),
            ev_completed("resp-6"),
        ]),
    )
    .await;

    test.submit_turn("wait session a").await?;

    let third_request = third_completion.single_request();
    let third_items = function_tool_output_items(&third_request, "call-3");
    assert_eq!(third_items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script completed\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&third_items, /*index*/ 0),
    );
    assert_eq!(text_item(&third_items, /*index*/ 1), "session a done");

    fs::write(&session_b_gate, "ready")?;
    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-7"),
            responses::ev_function_call(
                "call-4",
                "wait",
                &serde_json::to_string(&serde_json::json!({
                    "cell_id": session_b_id.clone(),
                    "yield_time_ms": 1_000,
                }))?,
            ),
            ev_completed("resp-7"),
        ]),
    )
    .await;
    let fourth_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-4", "session b done"),
            ev_completed("resp-8"),
        ]),
    )
    .await;

    test.submit_turn("wait session b").await?;

    let fourth_request = fourth_completion.single_request();
    let fourth_items = function_tool_output_items(&fourth_request, "call-4");
    assert_eq!(fourth_items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script completed\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&fourth_items, /*index*/ 0),
    );
    assert_eq!(text_item(&fourth_items, /*index*/ 1), "session b done");

    Ok(())
}

#[cfg_attr(windows, ignore = "no exec_command on Windows")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_wait_can_terminate_and_continue() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let mut builder = test_codex().with_config(move |config| {
        let _ = config.features.enable(Feature::CodeMode);
    });
    let test = builder.build(&server).await?;
    let termination_gate = test.workspace_path("code-mode-terminate.ready");
    let termination_wait = wait_for_file_source(&termination_gate)?;

    let code = format!(
        r#"
text("phase 1");
yield_control();
{termination_wait}
text("phase 2");
"#
    );

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_custom_tool_call("call-1", "exec", &code),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let first_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "waiting"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    test.submit_turn("start the long exec").await?;

    let first_request = first_completion.single_request();
    let first_items = custom_tool_output_items(&first_request, "call-1");
    assert_eq!(first_items.len(), 2);
    let cell_id = extract_running_cell_id(text_item(&first_items, /*index*/ 0));
    assert_eq!(text_item(&first_items, /*index*/ 1), "phase 1");

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-3"),
            responses::ev_function_call(
                "call-2",
                "wait",
                &serde_json::to_string(&serde_json::json!({
                    "cell_id": cell_id.clone(),
                    "terminate": true,
                }))?,
            ),
            ev_completed("resp-3"),
        ]),
    )
    .await;
    let second_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-2", "terminated"),
            ev_completed("resp-4"),
        ]),
    )
    .await;

    test.submit_turn("terminate it").await?;

    let second_request = second_completion.single_request();
    let second_items = function_tool_output_items(&second_request, "call-2");
    assert_eq!(second_items.len(), 1);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script terminated\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&second_items, /*index*/ 0),
    );

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-5"),
            ev_custom_tool_call(
                "call-3",
                "exec",
                r#"
text("after terminate");
"#,
            ),
            ev_completed("resp-5"),
        ]),
    )
    .await;
    let third_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-3", "done"),
            ev_completed("resp-6"),
        ]),
    )
    .await;

    test.submit_turn("run another exec").await?;

    let third_request = third_completion.single_request();
    let third_items = custom_tool_output_items(&third_request, "call-3");
    assert_eq!(third_items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script completed\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&third_items, /*index*/ 0),
    );
    assert_eq!(text_item(&third_items, /*index*/ 1), "after terminate");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_wait_returns_error_for_unknown_session() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let mut builder = test_codex().with_config(move |config| {
        let _ = config.features.enable(Feature::CodeMode);
    });
    let test = builder.build(&server).await?;

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            responses::ev_function_call(
                "call-1",
                "wait",
                &serde_json::to_string(&serde_json::json!({
                    "cell_id": "999999",
                    "yield_time_ms": 1_000,
                }))?,
            ),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    test.submit_turn("wait on an unknown exec cell").await?;

    let request = completion.single_request();
    let (_, success) = request
        .function_call_output_content_and_success("call-1")
        .expect("function tool output should be present");
    assert_ne!(success, Some(true));

    let items = function_tool_output_items(&request, "call-1");
    assert_eq!(items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script failed\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&items, /*index*/ 0),
    );
    assert_eq!(
        text_item(&items, /*index*/ 1),
        "Script error:\nexec cell 999999 not found"
    );

    Ok(())
}

#[cfg_attr(windows, ignore = "no exec_command on Windows")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_wait_terminate_returns_completed_session_if_it_finished_after_yield_control()
-> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let mut builder = test_codex().with_config(move |config| {
        let _ = config.features.enable(Feature::CodeMode);
    });
    let test = builder.build(&server).await?;
    let session_a_gate = test.workspace_path("code-mode-session-a-finished.ready");
    let session_b_gate = test.workspace_path("code-mode-session-b-blocked.ready");
    let session_a_done_marker = test.workspace_path("code-mode-session-a-done.txt");
    let session_a_wait = wait_for_file_source(&session_a_gate)?;
    let session_b_wait = wait_for_file_source(&session_b_gate)?;
    let session_a_done_marker_quoted =
        shlex::try_join([session_a_done_marker.to_string_lossy().as_ref()])?;
    let session_a_done_command = format!("printf done > {session_a_done_marker_quoted}");

    let session_a_code = format!(
        r#"
text("session a start");
yield_control();
{session_a_wait}
text("session a done");
await tools.exec_command({{ cmd: {session_a_done_command:?} }});
"#
    );
    let session_b_code = format!(
        r#"
text("session b start");
yield_control();
{session_b_wait}
text("session b done");
"#
    );

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_custom_tool_call("call-1", "exec", &session_a_code),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let first_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "session a waiting"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    test.submit_turn("start session a").await?;

    let first_request = first_completion.single_request();
    let first_items = custom_tool_output_items(&first_request, "call-1");
    assert_eq!(first_items.len(), 2);
    let session_a_id = extract_running_cell_id(text_item(&first_items, /*index*/ 0));
    assert_eq!(text_item(&first_items, /*index*/ 1), "session a start");

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-3"),
            ev_custom_tool_call("call-2", "exec", &session_b_code),
            ev_completed("resp-3"),
        ]),
    )
    .await;
    let second_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-2", "session b waiting"),
            ev_completed("resp-4"),
        ]),
    )
    .await;

    test.submit_turn("start session b").await?;

    let second_request = second_completion.single_request();
    let second_items = custom_tool_output_items(&second_request, "call-2");
    assert_eq!(second_items.len(), 2);
    let session_b_id = extract_running_cell_id(text_item(&second_items, /*index*/ 0));
    assert_eq!(text_item(&second_items, /*index*/ 1), "session b start");

    fs::write(&session_a_gate, "ready")?;
    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-5"),
            responses::ev_function_call(
                "call-3",
                "wait",
                &serde_json::to_string(&serde_json::json!({
                    "cell_id": session_b_id.clone(),
                    "yield_time_ms": 1_000,
                }))?,
            ),
            ev_completed("resp-5"),
        ]),
    )
    .await;
    let third_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-3", "session b still waiting"),
            ev_completed("resp-6"),
        ]),
    )
    .await;

    test.submit_turn("wait session b").await?;

    let third_request = third_completion.single_request();
    let third_items = function_tool_output_items(&third_request, "call-3");
    assert_eq!(third_items.len(), 1);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script running with cell ID \d+\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&third_items, /*index*/ 0),
    );
    assert_eq!(
        extract_running_cell_id(text_item(&third_items, /*index*/ 0)),
        session_b_id
    );

    for _ in 0..100 {
        if session_a_done_marker.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(session_a_done_marker.exists());

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-7"),
            responses::ev_function_call(
                "call-4",
                "wait",
                &serde_json::to_string(&serde_json::json!({
                    "cell_id": session_a_id.clone(),
                    "terminate": true,
                }))?,
            ),
            ev_completed("resp-7"),
        ]),
    )
    .await;
    let fourth_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-4", "session a already done"),
            ev_completed("resp-8"),
        ]),
    )
    .await;

    test.submit_turn("terminate session a").await?;

    let fourth_request = fourth_completion.single_request();
    let fourth_items = function_tool_output_items(&fourth_request, "call-4");
    match fourth_items.len() {
        1 => {
            assert_regex_match(
                concat!(
                    r"(?s)\A",
                    r"Script terminated\nWall time \d+\.\d seconds\nOutput:\n\z"
                ),
                text_item(&fourth_items, /*index*/ 0),
            );
        }
        2 => {
            assert_regex_match(
                concat!(
                    r"(?s)\A",
                    r"Script (?:completed|terminated)\nWall time \d+\.\d seconds\nOutput:\n\z"
                ),
                text_item(&fourth_items, /*index*/ 0),
            );
            assert_eq!(text_item(&fourth_items, /*index*/ 1), "session a done");
        }
        other => panic!("unexpected number of content items: {other}"),
    }

    Ok(())
}

#[cfg_attr(windows, ignore = "no exec_command on Windows")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_background_keeps_running_on_later_turn_without_wait() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let mut builder = test_codex().with_config(move |config| {
        let _ = config.features.enable(Feature::CodeMode);
    });
    let test = builder.build(&server).await?;
    let resumed_file = test.workspace_path("code-mode-yield-resumed.txt");
    let resumed_file_quoted = shlex::try_join([resumed_file.to_string_lossy().as_ref()])?;
    let write_file_command = format!("printf resumed > {resumed_file_quoted}");
    let wait_for_file_command =
        format!("while [ ! -f {resumed_file_quoted} ]; do sleep 0.01; done; printf ready");
    let code = format!(
        r#"
text("before yield");
yield_control();
await tools.exec_command({{ cmd: {write_file_command:?} }});
text("after yield");
"#
    );

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_custom_tool_call("call-1", "exec", &code),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let first_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "exec yielded"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    test.submit_turn("start yielded exec").await?;

    let first_request = first_completion.single_request();
    let first_items = custom_tool_output_items(&first_request, "call-1");
    assert_eq!(first_items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script running with cell ID \d+\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&first_items, /*index*/ 0),
    );
    assert_eq!(text_item(&first_items, /*index*/ 1), "before yield");

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-3"),
            responses::ev_function_call(
                "call-2",
                "exec_command",
                &serde_json::to_string(&serde_json::json!({
                    "cmd": wait_for_file_command,
                }))?,
            ),
            ev_completed("resp-3"),
        ]),
    )
    .await;
    let second_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-2", "file appeared"),
            ev_completed("resp-4"),
        ]),
    )
    .await;

    test.submit_turn("wait for resumed file").await?;

    let second_request = second_completion.single_request();
    assert!(
        second_request
            .function_call_output_text("call-2")
            .is_some_and(|output| output.ends_with("ready"))
    );
    assert_eq!(fs::read_to_string(&resumed_file)?, "resumed");

    Ok(())
}

#[cfg_attr(windows, ignore = "no exec_command on Windows")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_wait_uses_its_own_max_tokens_budget() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let mut builder = test_codex().with_config(move |config| {
        let _ = config.features.enable(Feature::CodeMode);
    });
    let test = builder.build(&server).await?;
    let completion_gate = test.workspace_path("code-mode-max-tokens.ready");
    let completion_wait = wait_for_file_source(&completion_gate)?;

    let code = format!(
        r#"// @exec: {{"max_output_tokens": 100}}
text("phase 1");
yield_control();
{completion_wait}
text("token one token two token three token four token five token six token seven");
"#
    );

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_custom_tool_call("call-1", "exec", &code),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let first_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "waiting"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    test.submit_turn("start the long exec").await?;

    let first_request = first_completion.single_request();
    let first_items = custom_tool_output_items(&first_request, "call-1");
    assert_eq!(first_items.len(), 2);
    assert_eq!(text_item(&first_items, /*index*/ 1), "phase 1");
    let cell_id = extract_running_cell_id(text_item(&first_items, /*index*/ 0));

    fs::write(&completion_gate, "ready")?;
    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-3"),
            responses::ev_function_call(
                "call-2",
                "wait",
                &serde_json::to_string(&serde_json::json!({
                    "cell_id": cell_id.clone(),
                    "yield_time_ms": 1_000,
                    "max_tokens": 6,
                }))?,
            ),
            ev_completed("resp-3"),
        ]),
    )
    .await;
    let second_completion = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-2", "done"),
            ev_completed("resp-4"),
        ]),
    )
    .await;

    test.submit_turn("wait for completion").await?;

    let second_request = second_completion.single_request();
    let second_items = function_tool_output_items(&second_request, "call-2");
    assert_eq!(second_items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script completed\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&second_items, /*index*/ 0),
    );
    let expected_pattern = r#"(?sx)
\A
Total\ output\ lines:\ 1\n
\n
.*…\d+\ tokens\ truncated….*
\z
"#;
    assert_regex_match(expected_pattern, text_item(&second_items, /*index*/ 1));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_resume_after_set_timeout() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let (_test, second_mock) = run_code_mode_turn(
        &server,
        "use exec to wait for a timeout",
        r#"
await new Promise((resolve) => setTimeout(resolve, 10));
text("timer done");
"#,
        /*include_apply_patch*/ false,
    )
    .await?;

    let req = second_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "exec setTimeout call failed unexpectedly: {output}"
    );
    assert_eq!(output, "timer done");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_compare_elapsed_time_around_set_timeout() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let (_test, second_mock) = run_code_mode_turn(
        &server,
        "measure elapsed time around setTimeout",
        r#"
const start_ms = Date.now();
await new Promise((resolve) => setTimeout(resolve, 100));
const end_ms = Date.now();
text(JSON.stringify({
  start_ms,
  end_ms,
  elapsed_ms: end_ms - start_ms,
  waited_long_enough: end_ms - start_ms >= 100,
}));
"#,
        /*include_apply_patch*/ false,
    )
    .await?;

    let second_request = second_mock.single_request();
    let (second_output, second_success) =
        custom_tool_output_body_and_success(&second_request, "call-1");
    assert_ne!(
        second_success,
        Some(false),
        "exec compare time call failed unexpectedly: {second_output}"
    );
    let compared: Value = serde_json::from_str(
        &custom_tool_output_last_non_empty_text(&second_request, "call-1")
            .expect("exec compare time call should emit JSON"),
    )?;
    let elapsed_ms = compared
        .get("elapsed_ms")
        .and_then(Value::as_i64)
        .expect("elapsed_ms should be an integer");
    assert!(
        elapsed_ms >= 100,
        "expected elapsed_ms >= 100, got {elapsed_ms}"
    );
    assert_eq!(compared.get("waited_long_enough"), Some(&Value::Bool(true)));

    Ok(())
}
