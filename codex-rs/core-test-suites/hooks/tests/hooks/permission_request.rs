use super::common::*;

#[tokio::test]
async fn permission_request_hook_allows_shell_command_without_user_approval() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "permissionrequest-shell-command";
    let marker = std::env::temp_dir().join("permissionrequest-shell-command-marker");
    let command = format!("rm -f {}", marker.display());
    let args = serde_json::json!({ "command": command });
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                codex_core_test_runtime::responses::ev_function_call(
                    call_id,
                    "shell_command",
                    &serde_json::to_string(&args)?,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "permission request hook allowed it"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) = install_allow_permission_request_hook(home) {
                panic!("failed to write permission request hook test fixture: {error}");
            }
        })
        .with_config(trust_discovered_hooks);
    let test = builder.build(&server).await?;

    fs::write(&marker, "seed").context("create permission request marker")?;

    test.submit_turn_with_approval_and_permission_profile(
        "run the shell command after hook approval",
        AskForApproval::OnRequest,
        PermissionProfile::Disabled,
    )
    .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    requests[1].function_call_output(call_id);
    assert!(
        !marker.exists(),
        "approved command should remove marker file"
    );

    let hook_inputs = assert_single_permission_request_hook_input(
        test.codex_home_path(),
        &command,
        /*description*/ None,
    )?;
    assert!(
        hook_inputs[0].get("tool_use_id").is_none(),
        "PermissionRequest input should not include a tool_use_id",
    );
    assert!(
        hook_inputs[0]["turn_id"]
            .as_str()
            .is_some_and(|turn_id| !turn_id.is_empty())
    );

    Ok(())
}

#[tokio::test]
async fn permission_request_hook_allows_apply_patch_with_write_alias() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "permissionrequest-apply-patch";
    let file_name = "permission_request_apply_patch.txt";
    let patch_path = format!("../{file_name}");
    let patch = format!(
        r#"*** Begin Patch
*** Add File: {patch_path}
+approved
*** End Patch"#
    );
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_apply_patch_custom_tool_call(call_id, &patch),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "permission request hook allowed apply_patch"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) = write_permission_request_hook(
                home,
                Some("^Write$"),
                "allow",
                PERMISSION_REQUEST_ALLOW_REASON,
            ) {
                panic!("failed to write permission request hook test fixture: {error}");
            }
        })
        .with_config(|config| {
            trust_discovered_hooks(config);
        });
    let test = builder.build(&server).await?;
    let target_path = test.workspace_path(&patch_path);

    test.submit_turn_with_approval_and_permission_profile(
        "apply the patch after hook approval",
        AskForApproval::OnRequest,
        restrictive_workspace_write_profile(),
    )
    .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    requests[1].custom_tool_call_output(call_id);
    assert!(
        target_path.exists(),
        "approved apply_patch should create the out-of-workspace file"
    );

    assert_single_permission_request_hook_input_for_tool(
        test.codex_home_path(),
        "apply_patch",
        &patch,
        /*description*/ None,
    )?;

    Ok(())
}

#[tokio::test]
async fn permission_request_hook_sees_raw_exec_command_input() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "permissionrequest-exec-command";
    let marker = std::env::temp_dir().join("permissionrequest-exec-command-marker");
    let command = format!("rm -f {}", marker.display());
    let justification = "remove the temporary marker";
    let args = serde_json::json!({
        "cmd": command,
        "login": true,
        "sandbox_permissions": "require_escalated",
        "justification": justification,
    });
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                codex_core_test_runtime::responses::ev_function_call(
                    call_id,
                    "exec_command",
                    &serde_json::to_string(&args)?,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "permission request hook allowed exec_command"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) = install_allow_permission_request_hook(home) {
                panic!("failed to write permission request hook test fixture: {error}");
            }
        })
        .with_config(|config| {
            config.use_experimental_unified_exec_tool = true;
            trust_discovered_hooks(config);
            config
                .features
                .enable(Feature::UnifiedExec)
                .expect("test config should allow feature update");
        });
    let test = builder.build(&server).await?;

    fs::write(&marker, "seed").context("create exec command permission request marker")?;

    test.submit_turn_with_approval_and_permission_profile(
        "run the exec command after hook approval",
        AskForApproval::OnRequest,
        PermissionProfile::read_only(),
    )
    .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    requests[1].function_call_output(call_id);
    assert!(
        !marker.exists(),
        "approved exec command should remove marker file"
    );

    assert_single_permission_request_hook_input(
        test.codex_home_path(),
        &command,
        Some(justification),
    )?;

    Ok(())
}

#[tokio::test]
async fn permission_request_hook_allows_network_approval_without_prompt() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let home = Arc::new(TempDir::new()?);
    fs::write(
        home.path().join("config.toml"),
        r#"default_permissions = "workspace"

[permissions.workspace.filesystem]
":minimal" = "read"

[permissions.workspace.network]
enabled = true
mode = "limited"
allow_local_binding = true
"#,
    )?;
    let call_id = "permissionrequest-network-approval";
    let command = r#"python3 -c "import urllib.request; opener = urllib.request.build_opener(urllib.request.ProxyHandler()); print('OK:' + opener.open('http://codex-network-test.invalid', timeout=2).read().decode(errors='replace'))""#;
    let args = serde_json::json!({ "command": command });
    let _responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(call_id, "shell_command", &serde_json::to_string(&args)?),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "permission request hook allowed network access"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let approval_policy = AskForApproval::OnFailure;
    let permission_profile = network_workspace_write_profile();
    let permission_profile_for_config = permission_profile.clone();
    let test = test_codex()
        .with_home(Arc::clone(&home))
        .with_pre_build_hook(|home| {
            if let Err(error) = install_allow_permission_request_hook(home) {
                panic!("failed to write permission request hook test fixture: {error}");
            }
        })
        .with_cloud_requirements(managed_network_requirements_loader())
        .with_config(move |config| {
            trust_discovered_hooks(config);
            config.permissions.approval_policy = Constrained::allow_any(approval_policy);
            config
                .permissions
                .set_permission_profile(permission_profile_for_config)
                .expect("set permission profile");
        })
        .build(&server)
        .await?;
    assert!(
        test.config.managed_network_requirements_enabled(),
        "expected managed network requirements to be enabled"
    );
    assert!(
        test.config.permissions.network.is_some(),
        "expected managed network proxy config to be present"
    );
    test.session_configured
        .network_proxy
        .as_ref()
        .expect("expected runtime managed network proxy addresses");

    test.submit_turn_with_approval_and_permission_profile(
        "run the shell command after network hook approval",
        approval_policy,
        permission_profile,
    )
    .await?;

    timeout(Duration::from_secs(10), async {
        loop {
            if test
                .codex_home_path()
                .join("permission_request_hook_log.jsonl")
                .exists()
            {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("expected network approval hook to run");

    assert!(
        timeout(
            Duration::from_secs(2),
            wait_for_event(&test.codex, |event| matches!(
                event,
                EventMsg::ExecApprovalRequest(_)
            ))
        )
        .await
        .is_err(),
        "expected the network approval hook to bypass the approval prompt"
    );

    assert_single_permission_request_hook_input(
        test.codex_home_path(),
        command,
        Some("network-access http://codex-network-test.invalid:80"),
    )?;

    test.codex.submit(Op::Shutdown {}).await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::ShutdownComplete)
    })
    .await;

    Ok(())
}

#[cfg(not(target_os = "linux"))]
#[tokio::test]
async fn permission_request_hook_sees_retry_context_after_sandbox_denial() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let call_id = "permissionrequest-retry-shell-command";
    let marker = "permissionrequest_retry_marker.txt";
    let command = format!("printf retry > {marker}");
    let args = serde_json::json!({ "command": command });
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                codex_core_test_runtime::responses::ev_function_call(
                    call_id,
                    "shell_command",
                    &serde_json::to_string(&args)?,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "permission request hook allowed retry"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut builder = test_codex()
        .with_pre_build_hook(|home| {
            if let Err(error) = install_allow_permission_request_hook(home) {
                panic!("failed to write permission request hook test fixture: {error}");
            }
        })
        .with_config(trust_discovered_hooks);
    let test = builder.build(&server).await?;
    let marker_path = test.workspace_path(marker);
    let _ = fs::remove_file(&marker_path);

    test.submit_turn_with_approval_and_permission_profile(
        "retry the shell command after sandbox denial",
        AskForApproval::OnFailure,
        PermissionProfile::read_only(),
    )
    .await?;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    requests[1].function_call_output(call_id);
    assert_eq!(
        fs::read_to_string(&marker_path).context("read retry marker")?,
        "retry"
    );

    assert_single_permission_request_hook_input(
        test.codex_home_path(),
        &command,
        /*description*/ None,
    )?;

    Ok(())
}
