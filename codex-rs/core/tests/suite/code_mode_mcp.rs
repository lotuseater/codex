#![allow(dead_code, unused_imports, clippy::expect_used, clippy::unwrap_used)]

include!("code_mode_shared.rs");

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_print_structured_mcp_tool_result_fields() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let code = r#"
const { content, structuredContent, isError } = await tools.mcp__rmcp__echo({
  message: "ping",
});
text(
  `echo=${structuredContent?.echo ?? "missing"}\n` +
    `env=${structuredContent?.env ?? "missing"}\n` +
    `isError=${String(isError)}\n` +
    `contentLength=${content.length}`
);
"#;

    let (_test, second_mock) =
        run_code_mode_turn_with_rmcp(&server, "use exec to run the rmcp echo tool", code).await?;

    let req = second_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "exec rmcp echo call failed unexpectedly: {output}"
    );
    assert_eq!(
        output,
        "echo=ECHOING: ping
env=propagated-env
isError=false
contentLength=0"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_only_can_call_mcp_tool() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let code = r#"
const result = await tools.mcp__rmcp__echo({ message: "ping" });
text(`echo=${result.structuredContent?.echo ?? "missing"}`);
"#;

    let (_test, second_mock) = run_code_mode_turn_with_rmcp_mode(
        &server,
        "use exec to run the rmcp echo tool in code mode only",
        code,
        /*code_mode_only*/ true,
    )
    .await?;

    let req = second_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "code_mode_only rmcp tool call failed unexpectedly: {output}"
    );
    assert_eq!(output, "echo=ECHOING: ping");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_exposes_mcp_tools_on_global_tools_object() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let code = r#"
const { content, structuredContent, isError } = await tools.mcp__rmcp__echo({
  message: "ping",
});
text(
  `hasEcho=${String(Object.keys(tools).includes("mcp__rmcp__echo"))}\n` +
    `echoType=${typeof tools.mcp__rmcp__echo}\n` +
    `echo=${structuredContent?.echo ?? "missing"}\n` +
    `isError=${String(isError)}\n` +
    `contentLength=${content.length}`
);
"#;

    let (_test, second_mock) =
        run_code_mode_turn_with_rmcp(&server, "use exec to inspect the global tools object", code)
            .await?;

    let req = second_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "exec global rmcp access failed unexpectedly: {output}"
    );
    assert_eq!(
        output,
        "hasEcho=true
echoType=function
echo=ECHOING: ping
isError=false
contentLength=0"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_exposes_namespaced_mcp_tools_on_global_tools_object() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let code = r#"
text(JSON.stringify({
  hasExecCommand: typeof tools.exec_command === "function",
  hasNamespacedEcho: typeof tools.mcp__rmcp__echo === "function",
}));
"#;

    let (_test, second_mock) =
        run_code_mode_turn_with_rmcp(&server, "use exec to inspect the global tools object", code)
            .await?;

    let req = second_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "exec global tools inspection failed unexpectedly: {output}"
    );

    let parsed: Value = serde_json::from_str(&output)?;
    assert_eq!(
        parsed,
        serde_json::json!({
            "hasExecCommand": !cfg!(windows),
            "hasNamespacedEcho": true,
        })
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_exposes_normalized_illegal_mcp_tool_names() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let code = r#"
const result = await tools.mcp__rmcp__echo_tool({ message: "ping" });
text(`echo=${result.structuredContent.echo}`);
"#;

    let (_test, second_mock) = run_code_mode_turn_with_rmcp(
        &server,
        "use exec to call a normalized rmcp tool name",
        code,
    )
    .await?;

    let req = second_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "exec normalized rmcp tool call failed unexpectedly: {output}"
    );
    assert_eq!(output, "echo=ECHOING: ping");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_exports_all_tools_metadata_for_namespaced_mcp_tools() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let code = r#"
const tool = ALL_TOOLS.find(
  ({ name }) => name === "mcp__rmcp__echo"
);
text(JSON.stringify(tool));
"#;

    let (_test, second_mock) =
        run_code_mode_turn_with_rmcp(&server, "use exec to inspect ALL_TOOLS", code).await?;

    let req = second_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "exec ALL_TOOLS MCP lookup failed unexpectedly: {output}"
    );

    let parsed: Value = serde_json::from_str(
        &custom_tool_output_last_non_empty_text(&req, "call-1")
            .expect("exec ALL_TOOLS MCP lookup should emit JSON"),
    )?;
    assert_eq!(
        parsed,
        serde_json::json!({
            "name": "mcp__rmcp__echo",
            "description": concat!(
                "Echo back the provided message and include environment data.\n\n",
                "exec tool declaration:\n",
                "```ts\n",
                "declare const tools: { mcp__rmcp__echo(args: { env_var?: string; message: string; }): ",
                "Promise<CallToolResult<{ echo: string; env: string | null; }>>; };\n",
                "```",
            ),
        })
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_print_content_only_mcp_tool_result_fields() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let code = r#"
const { content, structuredContent, isError } = await tools.mcp__rmcp__image_scenario({
  scenario: "text_only",
  caption: "caption from mcp",
});
text(
  `firstType=${content[0]?.type ?? "missing"}\n` +
    `firstText=${content[0]?.text ?? "missing"}\n` +
    `structuredContent=${String(structuredContent ?? null)}\n` +
    `isError=${String(isError)}`
);
"#;

    let (_test, second_mock) = run_code_mode_turn_with_rmcp(
        &server,
        "use exec to run the rmcp image scenario tool",
        code,
    )
    .await?;

    let req = second_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "exec rmcp image scenario call failed unexpectedly: {output}"
    );
    assert_eq!(
        output,
        "firstType=text
firstText=caption from mcp
structuredContent=null
isError=false"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_print_error_mcp_tool_result_fields() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let code = r#"
const { content, structuredContent, isError } = await tools.mcp__rmcp__echo({});
const firstText = content[0]?.text ?? "";
const mentionsMissingMessage =
  firstText.includes("missing field") && firstText.includes("message");
text(
  `isError=${String(isError)}\n` +
    `contentLength=${content.length}\n` +
    `mentionsMissingMessage=${String(mentionsMissingMessage)}\n` +
    `structuredContent=${String(structuredContent ?? null)}`
);
"#;

    let (_test, second_mock) =
        run_code_mode_turn_with_rmcp(&server, "use exec to call rmcp echo badly", code).await?;

    let req = second_mock.single_request();
    let (output, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "exec rmcp error call failed unexpectedly: {output}"
    );
    assert_eq!(
        output,
        "isError=true
contentLength=1
mentionsMissingMessage=true
structuredContent=null"
    );

    Ok(())
}
