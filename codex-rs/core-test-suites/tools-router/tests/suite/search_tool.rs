#![cfg(not(target_os = "windows"))]
#![allow(dead_code, unused_imports, clippy::unwrap_used, clippy::expect_used)]

use crate::search_tool_support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_tool_enabled_by_default_adds_tool_search() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder = configured_builder(apps_server.chatgpt_base_url.clone());
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "list tools",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let body = mock.single_request().body_json();
    let tools = body
        .get("tools")
        .and_then(Value::as_array)
        .expect("tools array should exist");
    let tool_search = tools
        .iter()
        .find(|tool| tool.get("type").and_then(Value::as_str) == Some(TOOL_SEARCH_TOOL_NAME))
        .cloned()
        .expect("tool_search should be present");

    assert_eq!(
        tool_search,
        json!({
            "type": "tool_search",
            "execution": "client",
            "description": tool_search["description"].as_str().expect("description should exist"),
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query for deferred tools."},
                    "limit": {"type": "number", "description": "Maximum number of tools to return (defaults to 8)."},
                },
                "required": ["query"],
                "additionalProperties": false,
            }
        })
    );

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn always_defer_feature_hides_small_app_tool_sets() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount(&server).await?;
    let mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder =
        configured_builder(apps_server.chatgpt_base_url.clone()).with_config(|config| {
            config
                .features
                .enable(Feature::ToolSearchAlwaysDeferMcpTools)
                .expect("test config should allow feature update");
        });
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "list tools",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let body = mock.single_request().body_json();
    let tools = tool_names(&body);
    assert!(
        tools.iter().any(|name| name == TOOL_SEARCH_TOOL_NAME),
        "small app tool sets should be deferred behind tool_search: {tools:?}"
    );
    assert!(
        tools.iter().all(|name| !name.starts_with("mcp__")),
        "MCP tools should not be directly exposed: {tools:?}"
    );

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn app_search_sources_are_hidden_for_api_key_auth() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount(&server).await?;
    let mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder = test_codex()
        .with_auth(CodexAuth::from_api_key("Test API Key"))
        .with_config(move |config| {
            configure_search_capable_apps(config, apps_server.chatgpt_base_url.as_str())
        });
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "list tools",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let body = mock.single_request().body_json();
    let tools = tool_names(&body);
    assert!(
        !tools.iter().any(|name| name == SEARCH_CALENDAR_NAMESPACE),
        "tools list should not include app tools for API key auth: {tools:?}"
    );
    let description = tool_search_description(&body).unwrap_or_default();
    assert!(
        !description.contains("Calendar"),
        "tool_search description should not include app sources for API key auth: {description}"
    );

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_tool_adds_discovery_instructions_to_tool_description() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder = configured_builder(apps_server.chatgpt_base_url.clone());
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "list tools",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let body = mock.single_request().body_json();
    let description = tool_search_description(&body).expect("tool_search description should exist");
    assert!(
        SEARCH_TOOL_DESCRIPTION_SNIPPETS
            .iter()
            .all(|snippet| description.contains(snippet)),
        "tool_search description should include the updated workflow: {description:?}"
    );
    assert!(
        !description.contains("remainder of the current session/thread"),
        "tool_search description should not mention legacy client-side persistence: {description:?}"
    );

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_tool_hides_apps_tools_without_search() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount_searchable(&server).await?;
    let mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder = configured_builder(apps_server.chatgpt_base_url.clone());
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "hello tools",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let body = mock.single_request().body_json();
    let tools = tool_names(&body);
    assert!(tools.iter().any(|name| name == TOOL_SEARCH_TOOL_NAME));
    assert!(!tools.iter().any(|name| name == CALENDAR_CREATE_TOOL));
    assert!(!tools.iter().any(|name| name == CALENDAR_LIST_TOOL));
    assert!(!tools.iter().any(|name| name == SEARCH_CALENDAR_NAMESPACE));

    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_app_mentions_respect_always_defer() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let apps_server = AppsTestServer::mount(&server).await?;
    let mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut builder =
        configured_builder(apps_server.chatgpt_base_url.clone()).with_config(|config| {
            config
                .features
                .enable(Feature::ToolSearchAlwaysDeferMcpTools)
                .expect("test config should allow feature update");
        });
    let test = builder.build(&server).await?;

    test.submit_turn_with_approval_and_permission_profile(
        "Use [$calendar](app://calendar) and then call tools.",
        AskForApproval::Never,
        PermissionProfile::Disabled,
    )
    .await?;

    let body = mock.single_request().body_json();
    let tools = tool_names(&body);
    assert!(
        tools.iter().any(|name| name == TOOL_SEARCH_TOOL_NAME),
        "explicit app mentions should leave app tools deferred when always-defer is active: {tools:?}"
    );
    assert!(
        namespace_child_tool(
            &body,
            SEARCH_CALENDAR_NAMESPACE,
            SEARCH_CALENDAR_CREATE_TOOL
        )
        .is_none(),
        "explicit app mentions should not directly expose create tool, got tools: {tools:?}"
    );
    assert!(
        namespace_child_tool(&body, SEARCH_CALENDAR_NAMESPACE, SEARCH_CALENDAR_LIST_TOOL).is_none(),
        "explicit app mentions should not directly expose list tool, got tools: {tools:?}"
    );

    Ok(())
}
