#![allow(dead_code, unused_imports, clippy::expect_used, clippy::unwrap_used)]

include!("code_mode_shared.rs");

#[cfg_attr(windows, ignore = "no exec_command on Windows")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_return_exec_command_output() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let (_test, second_mock) = run_code_mode_turn(
        &server,
        "use exec to run exec_command",
        r#"
text(JSON.stringify(await tools.exec_command({ cmd: "printf code_mode_exec_marker" })));
"#,
        /*include_apply_patch*/ false,
    )
    .await?;

    let req = second_mock.single_request();
    let items = custom_tool_output_items(&req, "call-1");
    assert_eq!(items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script completed\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&items, /*index*/ 0),
    );
    let parsed: Value = serde_json::from_str(text_item(&items, /*index*/ 1))?;
    assert!(
        parsed
            .get("chunk_id")
            .and_then(Value::as_str)
            .is_some_and(|chunk_id| !chunk_id.is_empty())
    );
    assert_eq!(
        parsed.get("output").and_then(Value::as_str),
        Some("code_mode_exec_marker"),
    );
    assert_eq!(parsed.get("exit_code").and_then(Value::as_i64), Some(0));
    assert!(parsed.get("wall_time_seconds").is_some());
    assert!(parsed.get("session_id").is_none());

    Ok(())
}

#[cfg_attr(windows, ignore = "no exec_command on Windows")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_only_can_call_nested_tools() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_custom_tool_call(
                "call-1",
                "exec",
                r#"
const output = await tools.exec_command({ cmd: "printf code_mode_only_nested_tool_marker" });
text(output.output);
"#,
            ),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let follow_up_mock = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    let mut builder = test_codex().with_config(|config| {
        let _ = config.features.enable(Feature::CodeModeOnly);
    });
    let test = builder.build(&server).await?;
    test.submit_turn("use exec to run nested tool in code mode only")
        .await?;

    let request = follow_up_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&request, "call-1");
    assert_ne!(
        success,
        Some(false),
        "code_mode_only nested tool call failed unexpectedly: {output}"
    );
    assert_eq!(output, "code_mode_only_nested_tool_marker");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_update_plan_nested_tool_result_is_empty_object() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let (_test, second_mock) = run_code_mode_turn(
        &server,
        "use exec to run update_plan",
        r#"
const result = await tools.update_plan({
  plan: [{ step: "Run update_plan from code mode", status: "in_progress" }],
});
text(JSON.stringify(result));
"#,
        /*include_apply_patch*/ false,
    )
    .await?;

    let req = second_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "exec update_plan call failed unexpectedly: {output}"
    );

    let parsed: Value = serde_json::from_str(&output)?;
    assert_eq!(parsed, serde_json::json!({}));

    Ok(())
}

#[cfg_attr(windows, ignore = "flaky on windows")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_nested_tool_calls_can_run_in_parallel() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let mut builder = test_codex()
        .with_model("test-gpt-5.1-codex")
        .with_config(move |config| {
            let _ = config.features.enable(Feature::CodeMode);
        });
    let test = builder.build(&server).await?;

    let warmup_code = r#"
const args = {
  sleep_after_ms: 10,
  barrier: {
    id: "code-mode-parallel-tools-warmup",
    participants: 2,
    timeout_ms: 1_000,
  },
};

await Promise.all([
  tools.test_sync_tool(args),
  tools.test_sync_tool(args),
]);
"#;
    let code = r#"
const args = {
  sleep_after_ms: 300,
  barrier: {
    id: "code-mode-parallel-tools",
    participants: 2,
    timeout_ms: 1_000,
  },
};

const results = await Promise.all([
  tools.test_sync_tool(args),
  tools.test_sync_tool(args),
]);

text(JSON.stringify(results));
"#;

    let response_mock = responses::mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-warm-1"),
                ev_custom_tool_call("call-warm-1", "exec", warmup_code),
                ev_completed("resp-warm-1"),
            ]),
            sse(vec![
                ev_assistant_message("msg-warm-1", "warmup done"),
                ev_completed("resp-warm-2"),
            ]),
            sse(vec![
                ev_response_created("resp-1"),
                ev_custom_tool_call("call-1", "exec", code),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    test.submit_turn("warm up nested tools in parallel").await?;

    let start = Instant::now();
    test.submit_turn("run nested tools in parallel").await?;
    let duration = start.elapsed();

    assert!(
        duration < Duration::from_millis(1_600),
        "expected nested tools to finish in parallel, got {duration:?}",
    );

    let req = response_mock
        .last_request()
        .expect("parallel code mode run should send a completion request");
    let items = custom_tool_output_items(&req, "call-1");
    assert_eq!(items.len(), 2);
    assert_eq!(text_item(&items, /*index*/ 1), "[\"ok\",\"ok\"]");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_returns_accumulated_output_when_script_fails() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let (_test, second_mock) = run_code_mode_turn(
        &server,
        "use code_mode to surface script failures",
        r#"
text("before crash");
text("still before crash");
throw new Error("boom");
"#,
        /*include_apply_patch*/ false,
    )
    .await?;

    let req = second_mock.single_request();
    let items = custom_tool_output_items(&req, "call-1");
    assert_eq!(items.len(), 4);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script failed\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&items, /*index*/ 0),
    );
    assert_eq!(text_item(&items, /*index*/ 1), "before crash");
    assert_eq!(text_item(&items, /*index*/ 2), "still before crash");
    assert_regex_match(
        r#"(?sx)
\A
Script\ error:\n
Error:\ boom\n
(?:\s+at\ .+\n?)+
\z
"#,
        text_item(&items, /*index*/ 3),
    );

    Ok(())
}

#[cfg_attr(windows, ignore = "no exec_command on Windows")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_exec_surfaces_handler_errors_as_exceptions() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let (_test, second_mock) = run_code_mode_turn(
        &server,
        "surface nested tool handler failures as script exceptions",
        r#"
try {
  await tools.exec_command({});
  text("no-exception");
} catch (error) {
  text(`caught:${error?.message ?? String(error)}`);
}
"#,
        /*include_apply_patch*/ false,
    )
    .await?;

    let request = second_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&request, "call-1");
    assert_ne!(
        success,
        Some(false),
        "script should catch the nested tool error: {output}"
    );
    assert!(
        output.contains("caught:"),
        "expected caught exception text in output: {output}"
    );
    assert!(
        !output.contains("no-exception"),
        "nested tool error should not allow success path: {output}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_output_serialized_text_via_global_helper() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let (_test, second_mock) = run_code_mode_turn(
        &server,
        "use exec to return structured text",
        r#"
text({ json: true });
"#,
        /*include_apply_patch*/ false,
    )
    .await?;

    let req = second_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&req, "call-1");
    eprintln!(
        "hidden dynamic tool raw output: {}",
        req.custom_tool_call_output("call-1")
    );
    assert_ne!(
        success,
        Some(false),
        "exec call failed unexpectedly: {output}"
    );
    assert_eq!(output, r#"{"json":true}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_notify_injects_additional_exec_tool_output_into_active_context() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let (_test, second_mock) = run_code_mode_turn(
        &server,
        "use exec notify helper",
        r#"
notify("code_mode_notify_marker");
await tools.test_sync_tool({});
text("done");
"#,
        /*include_apply_patch*/ false,
    )
    .await?;

    let req = second_mock.single_request();
    let has_notify_output = req
        .inputs_of_type("custom_tool_call_output")
        .iter()
        .any(|item| {
            item.get("call_id").and_then(serde_json::Value::as_str) == Some("call-1")
                && item
                    .get("output")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|text| text.contains("code_mode_notify_marker"))
                && item.get("name").and_then(serde_json::Value::as_str) == Some("exec")
        });
    assert!(
        has_notify_output,
        "expected notify marker in custom_tool_call_output item: {:?}",
        req.input()
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_exit_stops_script_immediately() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let (_test, second_mock) = run_code_mode_turn(
        &server,
        "use exec to stop script early with exit helper",
        r#"
text("before");
exit();
text("after");
"#,
        /*include_apply_patch*/ false,
    )
    .await?;

    let req = second_mock.single_request();
    let items = custom_tool_output_items(&req, "call-1");
    let (output, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "exec exit helper call failed unexpectedly: {output}"
    );
    assert_eq!(items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script completed\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&items, /*index*/ 0),
    );
    assert_eq!(text_item(&items, /*index*/ 1), "before");
    assert_eq!(output, "before");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_surfaces_text_stringify_errors() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let (_test, second_mock) = run_code_mode_turn(
        &server,
        "use exec to return circular text",
        r#"
const circular = {};
circular.self = circular;
text(circular);
"#,
        /*include_apply_patch*/ false,
    )
    .await?;

    let req = second_mock.single_request();
    let items = custom_tool_output_items(&req, "call-1");
    let (_, success) = req
        .custom_tool_call_output_content_and_success("call-1")
        .expect("custom tool output should be present");
    assert_ne!(
        success,
        Some(true),
        "circular stringify unexpectedly succeeded"
    );
    assert_eq!(items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script failed\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&items, /*index*/ 0),
    );
    assert!(text_item(&items, /*index*/ 1).contains("Script error:"));
    assert!(text_item(&items, /*index*/ 1).contains("Converting circular structure to JSON"));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_apply_patch_via_nested_tool() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let file_name = "code_mode_apply_patch.txt";
    let patch = format!(
        "*** Begin Patch\n*** Add File: {file_name}\n+hello from code_mode\n*** End Patch\n"
    );
    let code = format!("text(await tools.apply_patch({patch:?}));\n");

    let (test, second_mock) = run_code_mode_turn(
        &server,
        "use exec to run apply_patch",
        &code,
        /*include_apply_patch*/ true,
    )
    .await?;

    let req = second_mock.single_request();
    let items = custom_tool_output_items(&req, "call-1");
    let (_, success) = req
        .custom_tool_call_output_content_and_success("call-1")
        .expect("custom tool output should be present");
    assert_ne!(
        success,
        Some(false),
        "exec apply_patch call failed unexpectedly: {items:?}"
    );
    assert_eq!(items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script completed\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&items, /*index*/ 0),
    );
    assert_eq!(text_item(&items, /*index*/ 1), "{}");

    let file_path = test.cwd_path().join(file_name);
    assert_eq!(fs::read_to_string(&file_path)?, "hello from code_mode\n");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_store_and_load_values_across_turns() -> Result<()> {
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
            ev_custom_tool_call(
                "call-1",
                "exec",
                r#"
store("nb", { title: "Notebook", items: [1, true, null] });
text("stored");
"#,
            ),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let first_follow_up = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "stored"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    test.submit_turn("store value for later").await?;

    let first_request = first_follow_up.single_request();
    let (first_output, first_success) =
        custom_tool_output_body_and_success(&first_request, "call-1");
    assert_ne!(
        first_success,
        Some(false),
        "exec store call failed unexpectedly: {first_output}"
    );
    assert_eq!(first_output, "stored");

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-3"),
            ev_custom_tool_call(
                "call-2",
                "exec",
                r#"
text(JSON.stringify(load("nb")));
"#,
            ),
            ev_completed("resp-3"),
        ]),
    )
    .await;
    let second_follow_up = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-2", "loaded"),
            ev_completed("resp-4"),
        ]),
    )
    .await;

    test.submit_turn("load the stored value").await?;

    let second_request = second_follow_up.single_request();
    let (second_output, second_success) =
        custom_tool_output_body_and_success(&second_request, "call-2");
    assert_ne!(
        second_success,
        Some(false),
        "exec load call failed unexpectedly: {second_output}"
    );
    let loaded: Value = serde_json::from_str(
        &custom_tool_output_last_non_empty_text(&second_request, "call-2")
            .expect("exec load call should emit JSON"),
    )?;
    assert_eq!(
        loaded,
        serde_json::json!({ "title": "Notebook", "items": [1, true, null] })
    );

    Ok(())
}
