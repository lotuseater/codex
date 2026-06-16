#![allow(clippy::expect_used)]

use crate::rmcp_client_support::*;
use anyhow::Context as _;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_cwd)]
async fn stdio_server_uses_configured_cwd_before_runtime_fallback() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let server_name = "rmcp_configured_cwd";
    let expected_cwd = Arc::new(Mutex::new(None::<PathBuf>));
    let expected_cwd_for_config = Arc::clone(&expected_cwd);
    let rmcp_test_server_bin = remote_aware_stdio_server_bin()?;

    let fixture = test_codex()
        .with_workspace_setup(|cwd, fs| async move {
            fs.create_directory(
                &cwd.join("mcp-configured-cwd"),
                CreateDirectoryOptions { recursive: true },
                /*sandbox*/ None,
            )
            .await?;
            Ok::<(), anyhow::Error>(())
        })
        .with_config(move |config| {
            let configured_cwd = config.cwd.join("mcp-configured-cwd").into_path_buf();
            *expected_cwd_for_config
                .lock()
                .expect("expected cwd lock should not be poisoned") = Some(configured_cwd.clone());
            insert_mcp_server(
                config,
                server_name,
                stdio_transport_with_cwd(
                    rmcp_test_server_bin,
                    Some(HashMap::from([(
                        "MCP_TEST_VALUE".to_string(),
                        "configured-cwd".to_string(),
                    )])),
                    Vec::new(),
                    Some(configured_cwd),
                ),
                TestMcpServerOptions {
                    experimental_environment: remote_aware_experimental_environment(),
                    ..Default::default()
                },
            );
        })
        .build_remote_aware(&server)
        .await?;

    let expected_cwd = expected_cwd
        .lock()
        .expect("expected cwd lock should not be poisoned")
        .clone()
        .expect("test config should record configured MCP cwd");
    let structured = call_cwd_tool(&server, &fixture, server_name, "call-configured-cwd").await?;

    assert_cwd_tool_output(&structured, &expected_cwd);
    server.verify().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_cwd)]
async fn remote_stdio_server_uses_runtime_fallback_cwd_when_config_omits_cwd() -> anyhow::Result<()>
{
    skip_if_no_network!(Ok(()));
    if std::env::var_os(remote_env_env_var()).is_none() {
        return Ok(());
    }

    let server = responses::start_mock_server().await;
    let server_name = "rmcp_fallback_cwd";
    let expected_cwd = Arc::new(Mutex::new(None::<PathBuf>));
    let expected_cwd_for_config = Arc::clone(&expected_cwd);
    let rmcp_test_server_bin = remote_aware_stdio_server_bin()?;

    let fixture = test_codex()
        .with_config(move |config| {
            *expected_cwd_for_config
                .lock()
                .expect("expected cwd lock should not be poisoned") =
                Some(config.cwd.to_path_buf());
            insert_mcp_server(
                config,
                server_name,
                stdio_transport(
                    rmcp_test_server_bin,
                    Some(HashMap::from([(
                        "MCP_TEST_VALUE".to_string(),
                        "fallback-cwd".to_string(),
                    )])),
                    Vec::new(),
                ),
                TestMcpServerOptions {
                    experimental_environment: remote_aware_experimental_environment(),
                    ..Default::default()
                },
            );
        })
        .build_remote_aware(&server)
        .await?;

    let expected_cwd = expected_cwd
        .lock()
        .expect("expected cwd lock should not be poisoned")
        .clone()
        .expect("test config should record runtime fallback cwd");
    let structured = call_cwd_tool(&server, &fixture, server_name, "call-fallback-cwd").await?;

    assert_cwd_tool_output(&structured, &expected_cwd);
    server.verify().await;
    Ok(())
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_cwd)]
async fn local_stdio_server_uses_runtime_fallback_cwd_when_config_omits_cwd() -> anyhow::Result<()>
{
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let server_name = "rmcp_local_fallback_cwd";
    let expected_cwd = Arc::new(Mutex::new(None::<PathBuf>));
    let expected_cwd_for_config = Arc::clone(&expected_cwd);
    let rmcp_test_server_bin = cargo_bin("test_stdio_server")?;
    let relative_server_path = PathBuf::from("mcp-bin").join(
        rmcp_test_server_bin
            .file_name()
            .expect("test stdio server binary should have a file name"),
    );
    let relative_command = relative_server_path.to_string_lossy().into_owned();

    let fixture = test_codex()
        .with_config(move |config| {
            *expected_cwd_for_config
                .lock()
                .expect("expected cwd lock should not be poisoned") =
                Some(config.cwd.to_path_buf());

            let target_bin = config.cwd.join(&relative_server_path).into_path_buf();
            let target_dir = target_bin
                .parent()
                .expect("relative test server path should include a parent");
            fs::create_dir_all(target_dir).expect("create relative MCP bin directory");
            fs::copy(&rmcp_test_server_bin, &target_bin).expect("copy test stdio server");

            insert_mcp_server(
                config,
                server_name,
                stdio_transport(
                    relative_command,
                    Some(HashMap::from([(
                        "MCP_TEST_VALUE".to_string(),
                        "local-fallback-cwd".to_string(),
                    )])),
                    Vec::new(),
                ),
                TestMcpServerOptions::default(),
            );
        })
        .build(&server)
        .await?;

    let expected_cwd = expected_cwd
        .lock()
        .expect("expected cwd lock should not be poisoned")
        .clone()
        .expect("test config should record runtime fallback cwd");
    let structured =
        call_cwd_tool(&server, &fixture, server_name, "call-local-fallback-cwd").await?;

    assert_cwd_tool_output(&structured, &expected_cwd);
    server.verify().await;
    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_test_value)]
async fn stdio_server_propagates_whitelisted_env_vars() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;

    let call_id = "call-1234";
    let server_name = "rmcp_whitelist";
    let namespace = format!("mcp__{server_name}__");

    mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_function_call_with_namespace(
                call_id,
                &namespace,
                "echo",
                "{\"message\":\"ping\"}",
            ),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message("msg-1", "rmcp echo tool completed successfully."),
            responses::ev_completed("resp-2"),
        ]),
    )
    .await;

    let expected_env_value = "propagated-env-from-whitelist";
    let _guard = EnvVarGuard::set("MCP_TEST_VALUE", OsStr::new(expected_env_value));
    let rmcp_test_server_bin = remote_aware_stdio_server_bin()?;

    let fixture = test_codex()
        .with_config(move |config| {
            insert_mcp_server(
                config,
                server_name,
                stdio_transport(
                    rmcp_test_server_bin,
                    /*env*/ None,
                    vec!["MCP_TEST_VALUE".into()],
                ),
                TestMcpServerOptions {
                    experimental_environment: remote_aware_experimental_environment(),
                    ..Default::default()
                },
            );
        })
        .build_remote_aware(&server)
        .await?;
    fixture
        .codex
        .submit(read_only_user_turn(&fixture, "call the rmcp echo tool"))
        .await?;

    let begin_event = wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallBegin(_))
    })
    .await;

    let EventMsg::McpToolCallBegin(begin) = begin_event else {
        unreachable!("event guard guarantees McpToolCallBegin");
    };
    assert_eq!(begin.invocation.server, server_name);
    assert_eq!(begin.invocation.tool, "echo");

    let end_event = wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallEnd(_))
    })
    .await;
    let EventMsg::McpToolCallEnd(end) = end_event else {
        unreachable!("event guard guarantees McpToolCallEnd");
    };

    let result = end
        .result
        .as_ref()
        .expect("rmcp echo tool should return success");
    assert_eq!(result.is_error, Some(false));
    assert!(
        result.content.is_empty(),
        "content should default to an empty array"
    );

    let structured = result
        .structured_content
        .as_ref()
        .expect("structured content");
    let Value::Object(map) = structured else {
        panic!("structured content should be an object: {structured:?}");
    };
    let echo_value = map
        .get("echo")
        .and_then(Value::as_str)
        .expect("echo payload present");
    assert_eq!(echo_value, "ECHOING: ping");
    let env_value = map
        .get("env")
        .and_then(Value::as_str)
        .expect("env snapshot inserted");
    assert_eq!(env_value, expected_env_value);

    wait_for_event(&fixture.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    server.verify().await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_env_source)]
async fn stdio_server_propagates_explicit_local_env_var_source() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let call_id = "call-local-source";
    let server_name = "rmcp_local_source";
    let namespace = format!("mcp__{server_name}__");
    let env_name = "MCP_TEST_LOCAL_SOURCE";
    let expected_env_value = "propagated-explicit-local-source";

    mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_function_call_with_namespace(
                call_id,
                &namespace,
                "echo",
                &format!(r#"{{"message":"ping","env_var":"{env_name}"}}"#),
            ),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message("msg-1", "rmcp echo tool completed successfully."),
            responses::ev_completed("resp-2"),
        ]),
    )
    .await;

    let _guard = EnvVarGuard::set(env_name, OsStr::new(expected_env_value));
    let rmcp_test_server_bin = remote_aware_stdio_server_bin()?;

    let fixture = test_codex()
        .with_config(move |config| {
            insert_mcp_server(
                config,
                server_name,
                stdio_transport(
                    rmcp_test_server_bin,
                    /*env*/ None,
                    vec![McpServerEnvVar::Config {
                        name: env_name.to_string(),
                        source: Some("local".to_string()),
                    }],
                ),
                TestMcpServerOptions {
                    experimental_environment: remote_aware_experimental_environment(),
                    ..Default::default()
                },
            );
        })
        .build_remote_aware(&server)
        .await?;

    fixture
        .codex
        .submit(read_only_user_turn(&fixture, "call the rmcp echo tool"))
        .await?;

    wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallBegin(_))
    })
    .await;
    let end_event = wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallEnd(_))
    })
    .await;
    let EventMsg::McpToolCallEnd(end) = end_event else {
        unreachable!("event guard guarantees McpToolCallEnd");
    };
    let structured = end
        .result
        .as_ref()
        .expect("rmcp echo tool should return success")
        .structured_content
        .as_ref()
        .expect("structured content");
    assert_eq!(structured["env"], expected_env_value);

    wait_for_event(&fixture.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;
    server.verify().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_env_source)]
async fn remote_stdio_env_var_source_does_not_copy_local_env() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    if std::env::var_os(remote_env_env_var()).is_none() {
        return Ok(());
    }

    let server = responses::start_mock_server().await;
    let call_id = "call-remote-source";
    let server_name = "rmcp_remote_source";
    let namespace = format!("mcp__{server_name}__");
    let env_name = "MCP_TEST_REMOTE_SOURCE_ONLY";

    mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_function_call_with_namespace(
                call_id,
                &namespace,
                "echo",
                &format!(r#"{{"message":"ping","env_var":"{env_name}"}}"#),
            ),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message("msg-1", "rmcp echo tool completed successfully."),
            responses::ev_completed("resp-2"),
        ]),
    )
    .await;

    let _guard = EnvVarGuard::set(env_name, OsStr::new("local-value-should-not-cross"));
    let rmcp_test_server_bin = remote_aware_stdio_server_bin()?;

    let fixture = test_codex()
        .with_config(move |config| {
            insert_mcp_server(
                config,
                server_name,
                stdio_transport(
                    rmcp_test_server_bin,
                    /*env*/ None,
                    vec![McpServerEnvVar::Config {
                        name: env_name.to_string(),
                        source: Some("remote".to_string()),
                    }],
                ),
                TestMcpServerOptions {
                    experimental_environment: remote_aware_experimental_environment(),
                    ..Default::default()
                },
            );
        })
        .build_remote_aware(&server)
        .await?;

    fixture
        .codex
        .submit(read_only_user_turn(&fixture, "call the rmcp echo tool"))
        .await?;

    wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallBegin(_))
    })
    .await;
    let end_event = wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallEnd(_))
    })
    .await;
    let EventMsg::McpToolCallEnd(end) = end_event else {
        unreachable!("event guard guarantees McpToolCallEnd");
    };
    let structured = end
        .result
        .as_ref()
        .expect("rmcp echo tool should return success")
        .structured_content
        .as_ref()
        .expect("structured content");
    assert_eq!(structured["env"], Value::Null);

    wait_for_event(&fixture.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;
    server.verify().await;
    Ok(())
}
