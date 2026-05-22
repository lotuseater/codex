#![allow(dead_code, unused_imports, clippy::expect_used, clippy::unwrap_used)]

include!("code_mode_shared.rs");

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_only_restricts_prompt_tools() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let resp_mock = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder = test_codex().with_config(|config| {
        let _ = config.features.enable(Feature::CodeModeOnly);
    });
    let test = builder.build(&server).await?;
    test.submit_turn("list tools in code mode only").await?;

    let first_body = resp_mock.single_request().body_json();
    assert_eq!(
        tool_names(&first_body),
        vec!["exec".to_string(), "wait".to_string()]
    );

    Ok(())
}

#[cfg_attr(windows, ignore = "no exec_command on Windows")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_truncate_final_result_with_configured_budget() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let (_test, second_mock) = run_code_mode_turn(
        &server,
        "use exec to truncate the final result",
        r#"// @exec: {"max_output_tokens": 6}
text(JSON.stringify(await tools.exec_command({
  cmd: "printf 'token one token two token three token four token five token six token seven'",
  max_output_tokens: 100
})));
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
    let expected_pattern = r#"(?sx)
\A
Total\ output\ lines:\ 1\n
\n
.*…\d+\ tokens\ truncated….*
\z
"#;
    assert_regex_match(expected_pattern, text_item(&items, /*index*/ 1));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_lists_global_scope_items() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let code = r#"
text(JSON.stringify(Object.getOwnPropertyNames(globalThis).sort()));
"#;

    let (_test, second_mock) =
        run_code_mode_turn_with_rmcp(&server, "use exec to inspect global scope", code).await?;

    let req = second_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "exec global scope inspection failed unexpectedly: {output}"
    );
    let globals = serde_json::from_str::<Vec<String>>(&output)?;
    let globals = globals.into_iter().collect::<HashSet<_>>();
    let expected = [
        "AggregateError",
        "ALL_TOOLS",
        "Array",
        "ArrayBuffer",
        "AsyncDisposableStack",
        "BigInt",
        "BigInt64Array",
        "BigUint64Array",
        "Boolean",
        "clearTimeout",
        "DataView",
        "Date",
        "DisposableStack",
        "Error",
        "EvalError",
        "FinalizationRegistry",
        "Float16Array",
        "Float32Array",
        "Float64Array",
        "Function",
        "Infinity",
        "Int16Array",
        "Int32Array",
        "Int8Array",
        "Intl",
        "Iterator",
        "JSON",
        "Map",
        "Math",
        "NaN",
        "Number",
        "Object",
        "Promise",
        "Proxy",
        "RangeError",
        "ReferenceError",
        "Reflect",
        "RegExp",
        "Set",
        "String",
        "SuppressedError",
        "Symbol",
        "SyntaxError",
        "Temporal",
        "TypeError",
        "URIError",
        "Uint16Array",
        "Uint32Array",
        "Uint8Array",
        "Uint8ClampedArray",
        "WeakMap",
        "WeakRef",
        "WeakSet",
        "__codexContentItems",
        "add_content",
        "decodeURI",
        "decodeURIComponent",
        "encodeURI",
        "encodeURIComponent",
        "escape",
        "exit",
        "eval",
        "globalThis",
        "image",
        "isFinite",
        "isNaN",
        "load",
        "notify",
        "parseFloat",
        "parseInt",
        "setTimeout",
        "store",
        "text",
        "tools",
        "undefined",
        "unescape",
        "yield_control",
    ];
    for g in &globals {
        assert!(
            expected.contains(&g.as_str()),
            "unexpected global {g} in {globals:?}"
        );
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_exports_all_tools_metadata_for_builtin_tools() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let code = r#"
const tool = ALL_TOOLS.find(({ name }) => name === "view_image");
text(JSON.stringify(tool));
"#;

    let (_test, second_mock) = run_code_mode_turn(
        &server,
        "use exec to inspect ALL_TOOLS",
        code,
        /*include_apply_patch*/ false,
    )
    .await?;

    let req = second_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "exec ALL_TOOLS lookup failed unexpectedly: {output}"
    );

    let parsed: Value = serde_json::from_str(
        &custom_tool_output_last_non_empty_text(&req, "call-1")
            .expect("exec ALL_TOOLS lookup should emit JSON"),
    )?;
    assert_eq!(
        parsed,
        serde_json::json!({
            "name": "view_image",
            "description": "View a local image from the filesystem (only use if given a full filepath by the user, and the image isn't already attached to the thread context within <image ...> tags).\n\nexec tool declaration:\n```ts\ndeclare const tools: { view_image(args: {\n  // Local filesystem path to an image file\n  path: string;\n}): Promise<{\n  // Image detail hint returned by view_image. Returns `original` when original resolution is preserved, otherwise `null`.\n  detail: string | null;\n  // Data URL for the loaded image.\n  image_url: string;\n}>; };\n```",
        })
    );

    Ok(())
}
