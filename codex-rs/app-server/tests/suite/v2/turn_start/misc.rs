use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
#[cfg_attr(windows, ignore = "process id reporting differs on Windows")]
async fn command_execution_notifications_include_process_id() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let responses = vec![
        create_exec_command_sse_response("uexec-1")?,
        create_final_assistant_message_sse_response("done")?,
    ];
    let server = create_mock_responses_server_sequence(responses).await;
    let codex_home = TempDir::new()?;
    create_config_toml_with_sandbox(
        codex_home.path(),
        &server.uri(),
        "never",
        &BTreeMap::from([(Feature::UnifiedExec, true)]),
        "danger-full-access",
    )?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let start_id = mcp
        .send_thread_start_request(ThreadStartParams {
            model: Some("mock-model".to_string()),
            ..Default::default()
        })
        .await?;
    let start_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_id)),
    )
    .await??;
    let ThreadStartResponse { thread, .. } = to_response::<ThreadStartResponse>(start_resp)?;

    let turn_id = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: thread.id.clone(),
            input: vec![V2UserInput::Text {
                text: "run a command".to_string(),
                text_elements: Vec::new(),
            }],
            sandbox_policy: Some(codex_app_server_protocol::SandboxPolicy::DangerFullAccess),
            ..Default::default()
        })
        .await?;
    let turn_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn_id)),
    )
    .await??;
    let TurnStartResponse { turn: _turn } = to_response::<TurnStartResponse>(turn_resp)?;

    let started_command = timeout(DEFAULT_READ_TIMEOUT, async {
        loop {
            let notif = mcp
                .read_stream_until_notification_message("item/started")
                .await?;
            let started: ItemStartedNotification = serde_json::from_value(
                notif
                    .params
                    .clone()
                    .expect("item/started should include params"),
            )?;
            if let ThreadItem::CommandExecution { .. } = started.item {
                return Ok::<ThreadItem, anyhow::Error>(started.item);
            }
        }
    })
    .await??;
    let ThreadItem::CommandExecution {
        id,
        process_id: started_process_id,
        status,
        ..
    } = started_command
    else {
        unreachable!("loop ensures we break on command execution items");
    };
    assert_eq!(id, "uexec-1");
    assert_eq!(status, CommandExecutionStatus::InProgress);
    let started_process_id = started_process_id.expect("process id should be present");

    let completed_command = timeout(DEFAULT_READ_TIMEOUT, async {
        loop {
            let notif = mcp
                .read_stream_until_notification_message("item/completed")
                .await?;
            let completed: ItemCompletedNotification = serde_json::from_value(
                notif
                    .params
                    .clone()
                    .expect("item/completed should include params"),
            )?;
            if let ThreadItem::CommandExecution { .. } = completed.item {
                return Ok::<ThreadItem, anyhow::Error>(completed.item);
            }
        }
    })
    .await??;
    let ThreadItem::CommandExecution {
        id: completed_id,
        process_id: completed_process_id,
        status: completed_status,
        exit_code,
        ..
    } = completed_command
    else {
        unreachable!("loop ensures we break on command execution items");
    };
    assert_eq!(completed_id, "uexec-1");
    assert!(
        matches!(
            completed_status,
            CommandExecutionStatus::Completed | CommandExecutionStatus::Failed
        ),
        "unexpected command execution status: {completed_status:?}"
    );
    if completed_status == CommandExecutionStatus::Completed {
        assert_eq!(exit_code, Some(0));
    } else {
        assert!(exit_code.is_some(), "expected exit_code for failed command");
    }
    assert_eq!(
        completed_process_id.as_deref(),
        Some(started_process_id.as_str())
    );

    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("turn/completed"),
    )
    .await??;

    Ok(())
}

#[tokio::test]
async fn turn_start_with_elevated_override_does_not_persist_project_trust() -> Result<()> {
    let responses = vec![create_final_assistant_message_sse_response("Done")?];
    let server = create_mock_responses_server_sequence_unchecked(responses).await;

    let codex_home = TempDir::new()?;
    create_config_toml(
        codex_home.path(),
        &server.uri(),
        "never",
        &BTreeMap::from([(Feature::Personality, true)]),
    )?;

    let workspace = TempDir::new()?;

    let mut mcp = McpProcess::new(codex_home.path()).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let thread_request = mcp
        .send_thread_start_request(ThreadStartParams {
            cwd: Some(workspace.path().display().to_string()),
            ..Default::default()
        })
        .await?;
    let thread_response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(thread_request)),
    )
    .await??;
    let ThreadStartResponse { thread, .. } = to_response::<ThreadStartResponse>(thread_response)?;

    let turn_request = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: thread.id,
            cwd: Some(workspace.path().to_path_buf()),
            sandbox_policy: Some(codex_app_server_protocol::SandboxPolicy::DangerFullAccess),
            input: vec![V2UserInput::Text {
                text: "Hello".to_string(),
                text_elements: Vec::new(),
            }],
            ..Default::default()
        })
        .await?;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn_request)),
    )
    .await??;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("turn/completed"),
    )
    .await??;

    let config_toml = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    assert!(!config_toml.contains("trust_level = \"trusted\""));
    assert!(!config_toml.contains(&workspace.path().display().to_string()));

    Ok(())
}
