use super::*;

#[tokio::test]
async fn turn_start_updates_sandbox_and_cwd_between_turns_v2() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let tmp = TempDir::new()?;
    let codex_home = tmp.path().join("codex_home");
    std::fs::create_dir(&codex_home)?;
    let workspace_root = tmp.path().join("workspace");
    std::fs::create_dir(&workspace_root)?;
    let first_cwd = workspace_root.join("turn1");
    let second_cwd = workspace_root.join("turn2");
    std::fs::create_dir(&first_cwd)?;
    std::fs::create_dir(&second_cwd)?;

    let responses = vec![
        create_shell_command_sse_response(
            vec!["echo".to_string(), "first".to_string(), "turn".to_string()],
            /*workdir*/ None,
            Some(5000),
            "call-first",
        )?,
        create_final_assistant_message_sse_response("done first")?,
        create_shell_command_sse_response(
            vec!["echo".to_string(), "second".to_string(), "turn".to_string()],
            /*workdir*/ None,
            Some(5000),
            "call-second",
        )?,
        create_final_assistant_message_sse_response("done second")?,
    ];
    let server = create_mock_responses_server_sequence(responses).await;
    create_config_toml(
        &codex_home,
        &server.uri(),
        "untrusted",
        &BTreeMap::default(),
    )?;

    let mut mcp = McpProcess::new(&codex_home).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    // thread/start
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

    // first turn with workspace-write sandbox and first_cwd
    let first_turn = mcp
        .send_turn_start_request(TurnStartParams {
            environments: None,
            thread_id: thread.id.clone(),
            input: vec![V2UserInput::Text {
                text: "first turn".to_string(),
                text_elements: Vec::new(),
            }],
            responsesapi_client_metadata: None,
            cwd: Some(first_cwd.clone()),
            runtime_workspace_roots: None,
            approval_policy: Some(codex_app_server_protocol::AskForApproval::Never),
            approvals_reviewer: None,
            sandbox_policy: Some(codex_app_server_protocol::SandboxPolicy::WorkspaceWrite {
                writable_roots: vec![first_cwd.try_into()?],
                network_access: false,
                exclude_tmpdir_env_var: true,
                exclude_slash_tmp: true,
            }),
            permissions: None,
            model: Some("mock-model".to_string()),
            effort: Some(ReasoningEffort::Medium),
            summary: Some(ReasoningSummary::Auto),
            service_tier: None,
            context_budget_mode: None,
            personality: None,
            output_schema: None,
            collaboration_mode: None,
        })
        .await?;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(first_turn)),
    )
    .await??;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("turn/completed"),
    )
    .await??;
    mcp.clear_message_buffer();

    // second turn with workspace-write and second_cwd, ensure exec begins in second_cwd
    let second_turn = mcp
        .send_turn_start_request(TurnStartParams {
            environments: None,
            thread_id: thread.id.clone(),
            input: vec![V2UserInput::Text {
                text: "second turn".to_string(),
                text_elements: Vec::new(),
            }],
            responsesapi_client_metadata: None,
            cwd: Some(second_cwd.clone()),
            runtime_workspace_roots: None,
            approval_policy: Some(codex_app_server_protocol::AskForApproval::Never),
            approvals_reviewer: None,
            sandbox_policy: Some(codex_app_server_protocol::SandboxPolicy::DangerFullAccess),
            permissions: None,
            model: Some("mock-model".to_string()),
            effort: Some(ReasoningEffort::Medium),
            summary: Some(ReasoningSummary::Auto),
            service_tier: None,
            context_budget_mode: None,
            personality: None,
            output_schema: None,
            collaboration_mode: None,
        })
        .await?;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(second_turn)),
    )
    .await??;

    let command_exec_item = timeout(DEFAULT_READ_TIMEOUT, async {
        loop {
            let item_started_notification = mcp
                .read_stream_until_notification_message("item/started")
                .await?;
            let params = item_started_notification
                .params
                .clone()
                .expect("item/started params");
            let item_started: ItemStartedNotification =
                serde_json::from_value(params).expect("deserialize item/started notification");
            if matches!(item_started.item, ThreadItem::CommandExecution { .. }) {
                return Ok::<ThreadItem, anyhow::Error>(item_started.item);
            }
        }
    })
    .await??;
    let ThreadItem::CommandExecution {
        cwd,
        command,
        status,
        ..
    } = command_exec_item
    else {
        unreachable!("loop ensures we break on command execution items");
    };
    assert_eq!(cwd.as_path(), second_cwd.as_path());
    let expected_command = format_with_current_shell_display("echo second turn");
    assert_eq!(command, expected_command);
    assert_eq!(status, CommandExecutionStatus::InProgress);

    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("turn/completed"),
    )
    .await??;

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn turn_start_permission_profile_rebinds_runtime_workspace_roots_between_turns() -> Result<()>
{
    skip_if_no_network!(Ok(()));

    let tmp = TempDir::new()?;
    let codex_home = tmp.path().join("codex_home");
    std::fs::create_dir(&codex_home)?;
    let old_root = tmp.path().join("old-root");
    let new_root = tmp.path().join("new-root");
    std::fs::create_dir(&old_root)?;
    std::fs::create_dir(&new_root)?;
    let old_root_text = old_root.to_string_lossy().into_owned();
    let new_root_text = new_root.to_string_lossy().into_owned();

    let server = responses::start_mock_server().await;
    let response_mock = responses::mount_sse_sequence(
        &server,
        vec![
            responses::sse(vec![
                responses::ev_response_created("resp-1"),
                responses::ev_assistant_message("msg-1", "done first"),
                responses::ev_completed("resp-1"),
            ]),
            responses::sse(vec![
                responses::ev_response_created("resp-2"),
                responses::ev_assistant_message("msg-2", "done second"),
                responses::ev_completed("resp-2"),
            ]),
        ],
    )
    .await;
    let server_uri = server.uri();
    std::fs::write(
        codex_home.join("config.toml"),
        format!(
            r#"
model = "mock-model"
approval_policy = "never"
default_permissions = "dev"
model_provider = "mock_provider"

[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "{server_uri}/v1"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0

[permissions.dev.filesystem.":workspace_roots"]
"." = "write"
"#
        ),
    )?;

    let mut mcp = McpProcess::new(&codex_home).await?;
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

    let first_turn_id = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: thread.id.clone(),
            input: vec![V2UserInput::Text {
                text: "select dev profile".to_string(),
                text_elements: Vec::new(),
            }],
            runtime_workspace_roots: Some(vec![old_root]),
            permissions: Some("dev".to_string()),
            ..Default::default()
        })
        .await?;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(first_turn_id)),
    )
    .await??;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("turn/completed"),
    )
    .await??;

    let second_turn_id = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: thread.id.clone(),
            input: vec![V2UserInput::Text {
                text: "write in new root".to_string(),
                text_elements: Vec::new(),
            }],
            runtime_workspace_roots: Some(vec![new_root]),
            ..Default::default()
        })
        .await?;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(second_turn_id)),
    )
    .await??;

    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("turn/completed"),
    )
    .await??;

    let requests = response_mock.requests();
    assert_eq!(requests.len(), 2, "expected two Responses API requests");
    let latest_permissions_instructions =
        |request: &app_test_support::responses::ResponsesRequest| {
            request
                .message_input_texts("developer")
                .into_iter()
                .rev()
                .find(|text| text.contains("<permissions instructions>"))
                .expect("permissions instructions")
        };
    let first_permissions = latest_permissions_instructions(&requests[0]);
    assert!(first_permissions.contains(&old_root_text));
    assert!(
        !first_permissions.contains(&new_root_text),
        "first turn should materialize the initial runtime workspace root"
    );

    let second_permissions = latest_permissions_instructions(&requests[1]);
    assert!(second_permissions.contains(&new_root_text));
    assert!(
        !second_permissions.contains(&old_root_text),
        "second turn should rebind :workspace_roots to the updated runtime workspace root"
    );

    Ok(())
}

#[tokio::test]
async fn turn_start_resolves_sticky_thread_local_environment_and_turn_overrides() -> Result<()> {
    let tmp = TempDir::new()?;
    let codex_home = tmp.path().join("codex_home");
    std::fs::create_dir(&codex_home)?;
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir(&workspace)?;

    let server = create_mock_responses_server_repeating_assistant("done").await;
    create_config_toml(&codex_home, &server.uri(), "never", &BTreeMap::default())?;
    std::fs::write(
        codex_home.join("environments.toml"),
        r#"
[[environments]]
id = "remote"
url = "ws://127.0.0.1:1"
"#,
    )?;

    let mut mcp = McpProcess::new(&codex_home).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    for case in [
        EnvironmentSelectionCase {
            name: "sticky_unset_turn_unset",
            sticky: None,
            turn: None,
        },
        EnvironmentSelectionCase {
            name: "sticky_empty_turn_unset",
            sticky: Some(&[]),
            turn: None,
        },
        EnvironmentSelectionCase {
            name: "sticky_local_turn_unset",
            sticky: Some(&["local"]),
            turn: None,
        },
        EnvironmentSelectionCase {
            name: "sticky_local_turn_empty",
            sticky: Some(&["local"]),
            turn: Some(&[]),
        },
        EnvironmentSelectionCase {
            name: "sticky_empty_turn_local",
            sticky: Some(&[]),
            turn: Some(&["local"]),
        },
    ] {
        run_environment_selection_case(&mut mcp, &workspace, case).await?;
    }

    Ok(())
}

struct EnvironmentSelectionCase {
    name: &'static str,
    sticky: Option<&'static [&'static str]>,
    turn: Option<&'static [&'static str]>,
}

async fn run_environment_selection_case(
    mcp: &mut McpProcess,
    workspace: &Path,
    case: EnvironmentSelectionCase,
) -> Result<()> {
    let thread_req = mcp
        .send_thread_start_request(ThreadStartParams {
            model: Some("mock-model".to_string()),
            cwd: Some(workspace.to_string_lossy().into_owned()),
            environments: environment_params(case.sticky, workspace)?,
            ..Default::default()
        })
        .await?;
    let thread_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(thread_req)),
    )
    .await??;
    let ThreadStartResponse { thread, .. } = to_response::<ThreadStartResponse>(thread_resp)?;

    let turn_req = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: thread.id.clone(),
            input: vec![V2UserInput::Text {
                text: format!("run {}", case.name),
                text_elements: Vec::new(),
            }],
            environments: environment_params(case.turn, workspace)?,
            cwd: Some(workspace.to_path_buf()),
            model: Some("mock-model".to_string()),
            ..Default::default()
        })
        .await?;
    let turn_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn_req)),
    )
    .await??;
    let TurnStartResponse { turn } = to_response::<TurnStartResponse>(turn_resp)?;

    let started_notification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("turn/started"),
    )
    .await??;
    let started: TurnStartedNotification = serde_json::from_value(
        started_notification
            .params
            .ok_or_else(|| anyhow::anyhow!("turn/started notification should include params"))?,
    )?;
    assert_eq!(started.turn.id, turn.id, "{}", case.name);

    let completed_notification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("turn/completed"),
    )
    .await??;
    let completed: TurnCompletedNotification =
        serde_json::from_value(completed_notification.params.ok_or_else(|| {
            anyhow::anyhow!("turn/completed notification should include params")
        })?)?;
    assert_eq!(completed.turn.id, turn.id, "{}", case.name);
    assert_eq!(
        completed.turn.status,
        TurnStatus::Completed,
        "{}",
        case.name
    );

    mcp.clear_message_buffer();

    Ok(())
}

fn environment_params(
    ids: Option<&[&str]>,
    cwd: &Path,
) -> Result<Option<Vec<TurnEnvironmentParams>>> {
    ids.map(|ids| {
        ids.iter()
            .map(|id| {
                Ok(TurnEnvironmentParams {
                    environment_id: (*id).to_string(),
                    cwd: cwd.to_path_buf().try_into()?,
                })
            })
            .collect()
    })
    .transpose()
}
