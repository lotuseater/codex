#[path = "remote_env/support.rs"]
mod support;

use support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_command_routes_to_selected_remote_environment() -> Result<()> {
    skip_if_no_network!(Ok(()));
    let Some(_remote_env) = get_remote_test_env() else {
        return Ok(());
    };

    let server = start_mock_server().await;
    let test = unified_exec_test(&server).await?;
    let local_cwd = TempDir::new()?;
    fs::write(local_cwd.path().join("marker.txt"), "local-routing")?;
    let local_selection = TurnEnvironmentSelection {
        environment_id: LOCAL_ENVIRONMENT_ID.to_string(),
        cwd: local_cwd.path().abs(),
    };
    let remote_cwd = PathBuf::from(format!(
        "/tmp/codex-remote-routing-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
    ))
    .abs();
    let remote_marker_name = "marker.txt";
    test.fs()
        .create_directory(
            &remote_cwd,
            CreateDirectoryOptions { recursive: true },
            /*sandbox*/ None,
        )
        .await?;
    test.fs()
        .write_file(
            &remote_cwd.join(remote_marker_name),
            b"remote-routing".to_vec(),
            /*sandbox*/ None,
        )
        .await?;
    let remote_selection = TurnEnvironmentSelection {
        environment_id: REMOTE_ENVIRONMENT_ID.to_string(),
        cwd: remote_cwd.clone(),
    };
    let multi_env_output = exec_command_routing_output(
        &test,
        &server,
        "call-multi-env",
        json!({
            "shell": "/bin/sh",
            "cmd": format!("cat {remote_marker_name}"),
            "login": false,
            "yield_time_ms": 1_000,
            "environment_id": REMOTE_ENVIRONMENT_ID,
        }),
        Some(vec![local_selection, remote_selection]),
    )
    .await?;
    assert!(
        multi_env_output.contains("remote-routing"),
        "unexpected multi-env output: {multi_env_output}",
    );
    assert!(
        !multi_env_output.contains("local-routing"),
        "multi-env command should not route to local: {multi_env_output}",
    );

    test.fs()
        .remove(
            &remote_cwd,
            RemoveOptions {
                recursive: true,
                force: true,
            },
            /*sandbox*/ None,
        )
        .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_patch_freeform_routes_to_selected_remote_environment() -> Result<()> {
    skip_if_no_network!(Ok(()));
    let Some(_remote_env) = get_remote_test_env() else {
        return Ok(());
    };

    let server = start_mock_server().await;
    let mut builder = test_codex().with_config(|config| {
        config.include_apply_patch_tool = true;
    });
    let test = builder.build_remote_aware(&server).await?;
    let local_cwd = TempDir::new()?;
    let file_name = "apply_patch_remote_freeform.txt";
    let remote_cwd = PathBuf::from(format!(
        "/tmp/codex-remote-apply-patch-freeform-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
    ))
    .abs();
    test.fs()
        .create_directory(
            &remote_cwd,
            CreateDirectoryOptions { recursive: true },
            /*sandbox*/ None,
        )
        .await?;

    let patch = format!(
        "*** Begin Patch\n*** Environment ID: {REMOTE_ENVIRONMENT_ID}\n*** Add File: {file_name}\n+patched remote freeform\n*** End Patch"
    );
    let call_id = "apply-patch-remote-freeform";
    mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_apply_patch_custom_tool_call(call_id, &patch),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    test.submit_turn_with_environments(
        "apply patch to remote environment",
        Some(vec![
            TurnEnvironmentSelection {
                environment_id: LOCAL_ENVIRONMENT_ID.to_string(),
                cwd: local_cwd.path().abs(),
            },
            TurnEnvironmentSelection {
                environment_id: REMOTE_ENVIRONMENT_ID.to_string(),
                cwd: remote_cwd.clone(),
            },
        ]),
    )
    .await?;

    let remote_contents = test
        .fs()
        .read_file_text(&remote_cwd.join(file_name), /*sandbox*/ None)
        .await?;
    assert_eq!(remote_contents, "patched remote freeform\n");
    assert!(
        !local_cwd.path().join(file_name).exists(),
        "freeform apply_patch should not create the file in the local environment"
    );

    test.fs()
        .remove(
            &remote_cwd,
            RemoveOptions {
                recursive: true,
                force: true,
            },
            /*sandbox*/ None,
        )
        .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_patch_approvals_are_remembered_per_environment() -> Result<()> {
    skip_if_no_network!(Ok(()));
    let Some(_remote_env) = get_remote_test_env() else {
        return Ok(());
    };

    let server = start_mock_server().await;
    let mut builder = test_codex().with_config(|config| {
        config.include_apply_patch_tool = true;
        config.permissions.approval_policy = Constrained::allow_any(AskForApproval::OnRequest);
        config.approvals_reviewer = ApprovalsReviewer::User;
    });
    let test = builder.build_remote_aware(&server).await?;
    let local_cwd = TempDir::new()?;
    let remote_cwd = PathBuf::from(format!(
        "/tmp/codex-remote-apply-patch-approval-cwd-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
    ))
    .abs();
    test.fs()
        .create_directory(
            &remote_cwd,
            CreateDirectoryOptions { recursive: true },
            /*sandbox*/ None,
        )
        .await?;

    let target_path = PathBuf::from(format!(
        "/tmp/codex-apply-patch-approval-scope-{}.txt",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
    ))
    .abs();
    let _ = fs::remove_file(&target_path);
    test.fs()
        .remove(
            &target_path,
            RemoveOptions {
                recursive: false,
                force: true,
            },
            /*sandbox*/ None,
        )
        .await?;

    let environments = vec![
        TurnEnvironmentSelection {
            environment_id: LOCAL_ENVIRONMENT_ID.to_string(),
            cwd: local_cwd.path().abs(),
        },
        TurnEnvironmentSelection {
            environment_id: REMOTE_ENVIRONMENT_ID.to_string(),
            cwd: remote_cwd.clone(),
        },
    ];
    let local_patch = format!(
        "*** Begin Patch\n*** Environment ID: {LOCAL_ENVIRONMENT_ID}\n*** Add File: {}\n+local\n*** End Patch",
        target_path.display()
    );
    let remote_patch = format!(
        "*** Begin Patch\n*** Environment ID: {REMOTE_ENVIRONMENT_ID}\n*** Add File: {}\n+remote\n*** End Patch",
        target_path.display()
    );
    let remote_update_patch = format!(
        "*** Begin Patch\n*** Environment ID: {REMOTE_ENVIRONMENT_ID}\n*** Update File: {}\n@@\n-remote\n+remote updated\n*** End Patch",
        target_path.display()
    );

    mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-local-1"),
                ev_apply_patch_custom_tool_call("call-local", &local_patch),
                ev_completed("resp-local-1"),
            ]),
            sse(vec![
                ev_response_created("resp-local-2"),
                ev_assistant_message("msg-local", "done"),
                ev_completed("resp-local-2"),
            ]),
            sse(vec![
                ev_response_created("resp-remote-1"),
                ev_apply_patch_custom_tool_call("call-remote", &remote_patch),
                ev_completed("resp-remote-1"),
            ]),
            sse(vec![
                ev_response_created("resp-remote-2"),
                ev_assistant_message("msg-remote", "done"),
                ev_completed("resp-remote-2"),
            ]),
            sse(vec![
                ev_response_created("resp-remote-3"),
                ev_apply_patch_custom_tool_call("call-remote-followup", &remote_update_patch),
                ev_completed("resp-remote-3"),
            ]),
            sse(vec![
                ev_response_created("resp-remote-4"),
                ev_assistant_message("msg-remote-followup", "done"),
                ev_completed("resp-remote-4"),
            ]),
        ],
    )
    .await;

    submit_turn_with_approval_and_environments(
        &test,
        "apply patch in local environment",
        environments.clone(),
    )
    .await?;
    let approval = expect_patch_approval(&test, "call-local").await;
    test.codex
        .submit(Op::PatchApproval {
            id: approval.call_id,
            decision: ReviewDecision::ApprovedForSession,
        })
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    assert_eq!(fs::read_to_string(&target_path)?, "local\n");

    submit_turn_with_approval_and_environments(
        &test,
        "apply patch in remote environment",
        environments.clone(),
    )
    .await?;
    let approval = expect_patch_approval(&test, "call-remote").await;
    test.codex
        .submit(Op::PatchApproval {
            id: approval.call_id,
            decision: ReviewDecision::ApprovedForSession,
        })
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    assert_eq!(
        test.fs()
            .read_file_text(&target_path, /*sandbox*/ None)
            .await?,
        "remote\n"
    );

    submit_turn_with_approval_and_environments(
        &test,
        "apply patch again in remote environment",
        environments,
    )
    .await?;
    wait_for_completion_without_patch_approval(&test).await;
    assert_eq!(
        test.fs()
            .read_file_text(&target_path, /*sandbox*/ None)
            .await?,
        "remote updated\n"
    );

    let _ = fs::remove_file(&target_path);
    test.fs()
        .remove(
            &target_path,
            RemoveOptions {
                recursive: false,
                force: true,
            },
            /*sandbox*/ None,
        )
        .await?;
    test.fs()
        .remove(
            &remote_cwd,
            RemoveOptions {
                recursive: true,
                force: true,
            },
            /*sandbox*/ None,
        )
        .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_patch_intercepted_exec_command_routes_to_selected_remote_environment() -> Result<()>
{
    skip_if_no_network!(Ok(()));
    let Some(_remote_env) = get_remote_test_env() else {
        return Ok(());
    };

    let server = start_mock_server().await;
    let test = unified_exec_test(&server).await?;
    let local_cwd = TempDir::new()?;
    let file_name = "apply_patch_remote_exec.txt";
    let remote_cwd = PathBuf::from(format!(
        "/tmp/codex-remote-apply-patch-exec-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
    ))
    .abs();
    test.fs()
        .create_directory(
            &remote_cwd,
            CreateDirectoryOptions { recursive: true },
            /*sandbox*/ None,
        )
        .await?;

    let patch =
        format!("*** Begin Patch\n*** Add File: {file_name}\n+patched remote exec\n*** End Patch");
    let command = format!("apply_patch <<'EOF'\n{patch}\nEOF\n");
    let call_id = "apply-patch-remote-exec";
    mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(
                    call_id,
                    "exec_command",
                    &serde_json::to_string(&json!({
                        "shell": "/bin/sh",
                        "cmd": command,
                        "login": false,
                        "yield_time_ms": 5_000,
                        "environment_id": REMOTE_ENVIRONMENT_ID,
                    }))?,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    test.submit_turn_with_environments(
        "apply patch through exec command to remote environment",
        Some(vec![
            TurnEnvironmentSelection {
                environment_id: LOCAL_ENVIRONMENT_ID.to_string(),
                cwd: local_cwd.path().abs(),
            },
            TurnEnvironmentSelection {
                environment_id: REMOTE_ENVIRONMENT_ID.to_string(),
                cwd: remote_cwd.clone(),
            },
        ]),
    )
    .await?;

    let remote_contents = test
        .fs()
        .read_file_text(&remote_cwd.join(file_name), /*sandbox*/ None)
        .await?;
    assert_eq!(remote_contents, "patched remote exec\n");
    assert!(
        !local_cwd.path().join(file_name).exists(),
        "intercepted apply_patch should not create the file in the local environment"
    );

    test.fs()
        .remove(
            &remote_cwd,
            RemoveOptions {
                recursive: true,
                force: true,
            },
            /*sandbox*/ None,
        )
        .await?;

    Ok(())
}
