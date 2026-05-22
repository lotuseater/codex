#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::request_permissions_common::*;

#[tokio::test(flavor = "current_thread")]
async fn with_additional_permissions_requires_approval_under_on_request() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = start_mock_server().await;
    let approval_policy = AskForApproval::OnRequest;
    let sandbox_policy = SandboxPolicy::new_read_only_policy();
    let sandbox_policy_for_config = sandbox_policy.clone();

    let mut builder = test_codex().with_config(move |config| {
        config.permissions.approval_policy = Constrained::allow_any(approval_policy);
        config
            .set_legacy_sandbox_policy(sandbox_policy_for_config)
            .expect("set sandbox policy");
        config
            .features
            .enable(Feature::ExecPermissionApprovals)
            .expect("test config should allow feature update");
        config
            .features
            .enable(Feature::RequestPermissionsTool)
            .expect("test config should allow feature update");
    });
    let test = builder.build(&server).await?;

    let requested_dir = test.workspace_path("requested-dir");
    fs::create_dir_all(&requested_dir)?;
    let requested_dir_canonical = requested_dir.canonicalize()?;
    let requested_write = requested_dir.join("requested-but-unused.txt");
    let _ = fs::remove_file(&requested_write);
    let call_id = "request_permissions_skip_approval";
    let command = "touch requested-dir/requested-but-unused.txt";
    let requested_permissions = PermissionProfile {
        file_system: Some(FileSystemPermissions::from_read_write_roots(
            Some(vec![]),
            Some(vec![absolute_path(&requested_dir_canonical)]),
        )),
        ..Default::default()
    };
    let event = shell_event_with_request_permissions(call_id, command, &requested_permissions)?;

    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            event,
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let results = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    submit_turn(&test, call_id, approval_policy, sandbox_policy.clone()).await?;
    let approval = expect_exec_approval(&test, command).await;
    assert_eq!(
        approval.additional_permissions,
        Some(requested_permissions.clone())
    );
    test.codex
        .submit(Op::ExecApproval {
            id: approval.effective_approval_id(),
            turn_id: None,
            decision: ReviewDecision::Approved,
        })
        .await?;
    wait_for_completion(&test).await;

    let result = parse_result(&results.single_request().function_call_output(call_id));
    assert!(
        result.exit_code.is_none() || result.exit_code == Some(0),
        "unexpected exit code/output: {:?} {}",
        result.exit_code,
        result.stdout
    );
    assert!(
        requested_write.exists(),
        "touch command should create requested path"
    );

    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn with_additional_permissions_denied_approval_blocks_execution() -> Result<()> {
    skip_if_no_network!(Ok(()));
    let server = start_mock_server().await;
    let approval_policy = AskForApproval::OnRequest;
    let sandbox_policy = workspace_write_excluding_tmp();
    let sandbox_policy_for_config = sandbox_policy.clone();

    let mut builder = test_codex().with_config(move |config| {
        config.permissions.approval_policy = Constrained::allow_any(approval_policy);
        config
            .set_legacy_sandbox_policy(sandbox_policy_for_config)
            .expect("set sandbox policy");
        config
            .features
            .enable(Feature::ExecPermissionApprovals)
            .expect("test config should allow feature update");
        config
            .features
            .enable(Feature::RequestPermissionsTool)
            .expect("test config should allow feature update");
    });
    let test = builder.build(&server).await?;

    let outside_dir = tempfile::tempdir()?;
    let outside_write = outside_dir.path().join("workspace-write-denied.txt");
    let _ = fs::remove_file(&outside_write);

    let call_id = "request_permissions_denied";
    let command = format!(
        "printf {:?} > {:?} && cat {:?}",
        "should-not-write", outside_write, outside_write
    );
    let requested_permissions = PermissionProfile {
        file_system: Some(FileSystemPermissions::from_read_write_roots(
            Some(vec![]),
            Some(vec![absolute_path(outside_dir.path())]),
        )),
        ..Default::default()
    };
    let normalized_requested_permissions = PermissionProfile {
        file_system: Some(FileSystemPermissions::from_read_write_roots(
            Some(vec![]),
            Some(vec![AbsolutePathBuf::try_from(
                outside_dir.path().canonicalize()?,
            )?]),
        )),
        ..Default::default()
    };
    let event = shell_event_with_request_permissions(call_id, &command, &requested_permissions)?;

    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-denied-1"),
            event,
            ev_completed("resp-denied-1"),
        ]),
    )
    .await;
    let results = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-denied-1", "done"),
            ev_completed("resp-denied-2"),
        ]),
    )
    .await;

    submit_turn(&test, call_id, approval_policy, sandbox_policy.clone()).await?;

    let approval = expect_exec_approval(&test, &command).await;
    assert_eq!(
        approval.additional_permissions,
        Some(normalized_requested_permissions)
    );
    test.codex
        .submit(Op::ExecApproval {
            id: approval.effective_approval_id(),
            turn_id: None,
            decision: ReviewDecision::Denied,
        })
        .await?;
    wait_for_completion(&test).await;

    let result = parse_result(&results.single_request().function_call_output(call_id));
    assert_ne!(
        result.exit_code,
        Some(0),
        "denied command should not succeed"
    );
    assert!(
        result.stdout.contains("rejected by user"),
        "unexpected denial output: {}",
        result.stdout
    );
    assert!(
        !outside_write.exists(),
        "denied command should not create file"
    );

    let _ = fs::remove_file(outside_write);
    Ok(())
}
