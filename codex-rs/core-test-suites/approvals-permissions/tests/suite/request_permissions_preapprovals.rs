#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::request_permissions_common::*;

#[tokio::test(flavor = "current_thread")]
async fn request_permissions_tool_is_auto_denied_when_granular_request_permissions_is_disabled()
-> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = start_mock_server().await;
    let approval_policy = AskForApproval::Granular(GranularApprovalConfig {
        sandbox_approval: true,
        rules: true,
        skill_approval: true,
        request_permissions: false,
        mcp_elicitations: true,
    });
    let sandbox_policy = SandboxPolicy::new_read_only_policy();
    let sandbox_policy_for_config = sandbox_policy.clone();

    let mut builder = test_codex().with_config(move |config| {
        config.permissions.approval_policy = Constrained::allow_any(approval_policy);
        config
            .set_legacy_sandbox_policy(sandbox_policy_for_config)
            .expect("set sandbox policy");
        config
            .features
            .enable(Feature::RequestPermissionsTool)
            .expect("test config should allow feature update");
    });
    let test = builder.build(&server).await?;

    let requested_dir = test.workspace_path("request-permissions-reject");
    fs::create_dir_all(&requested_dir)?;
    let requested_permissions = requested_directory_write_permissions(&requested_dir);
    let call_id = "request_permissions_reject_auto_denied";
    let event = request_permissions_tool_event(
        call_id,
        "Request access through the standalone tool",
        &requested_permissions,
    )?;

    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-request-permissions-reject-1"),
            event,
            ev_completed("resp-request-permissions-reject-1"),
        ]),
    )
    .await;
    let results = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-request-permissions-reject-1", "done"),
            ev_completed("resp-request-permissions-reject-2"),
        ]),
    )
    .await;

    submit_turn(
        &test,
        "request permissions under granular.request_permissions = false",
        approval_policy,
        sandbox_policy,
    )
    .await?;

    let event = wait_for_event(&test.codex, |event| {
        matches!(
            event,
            EventMsg::RequestPermissions(_) | EventMsg::TurnComplete(_)
        )
    })
    .await;
    assert!(
        matches!(event, EventMsg::TurnComplete(_)),
        "request_permissions should not emit a prompt when granular.request_permissions is false: {event:?}"
    );

    let call_output = results.single_request().function_call_output(call_id);
    let result: RequestPermissionsResponse =
        serde_json::from_str(call_output["output"].as_str().unwrap_or_default())?;
    assert_eq!(
        result,
        RequestPermissionsResponse {
            permissions: RequestPermissionProfile::default(),
            scope: PermissionGrantScope::Turn,
            strict_auto_review: false,
        }
    );

    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn request_permissions_preapprove_explicit_exec_permissions_outside_on_request() -> Result<()>
{
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

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
    let outside_write = outside_dir.path().join("sticky-explicit-write.txt");
    let command = format!(
        "printf {:?} > {:?} && cat {:?}",
        "sticky-explicit-grant-ok", outside_write, outside_write
    );
    let requested_permissions = requested_directory_write_permissions(outside_dir.path());
    let normalized_requested_permissions =
        normalized_directory_write_permissions(outside_dir.path())?;
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-sticky-explicit-1"),
                request_permissions_tool_event(
                    "permissions-call",
                    "Allow writing outside the workspace",
                    &requested_permissions,
                )?,
                ev_completed("resp-sticky-explicit-1"),
            ]),
            sse(vec![
                ev_response_created("resp-sticky-explicit-2"),
                exec_command_event_with_request_permissions(
                    "exec-call",
                    &command,
                    &requested_permissions,
                )?,
                ev_completed("resp-sticky-explicit-2"),
            ]),
            sse(vec![
                ev_response_created("resp-sticky-explicit-3"),
                ev_assistant_message("msg-sticky-explicit-1", "done"),
                ev_completed("resp-sticky-explicit-3"),
            ]),
        ],
    )
    .await;

    submit_turn(
        &test,
        "write outside the workspace",
        approval_policy,
        sandbox_policy,
    )
    .await?;

    let granted_permissions = expect_request_permissions_event(&test, "permissions-call").await;
    assert_eq!(
        granted_permissions,
        normalized_requested_permissions.clone()
    );
    test.codex
        .submit(Op::RequestPermissionsResponse {
            id: "permissions-call".to_string(),
            response: RequestPermissionsResponse {
                permissions: normalized_requested_permissions,
                scope: PermissionGrantScope::Turn,
                strict_auto_review: false,
            },
        })
        .await?;

    if let Some(approval) = wait_for_exec_approval_or_completion(&test).await {
        test.codex
            .submit(Op::ExecApproval {
                id: approval.effective_approval_id(),
                turn_id: None,
                decision: ReviewDecision::Approved,
            })
            .await?;
        wait_for_completion(&test).await;
    }

    let exec_output = responses
        .function_call_output_text("exec-call")
        .map(|output| json!({ "output": output }))
        .unwrap_or_else(|| panic!("expected exec-call output"));
    let result = parse_result(&exec_output);
    assert!(
        result.exit_code.is_none_or(|exit_code| exit_code == 0),
        "expected success output, got exit_code={:?}, stdout={:?}",
        result.exit_code,
        result.stdout
    );
    assert_eq!(result.stdout.trim(), "sticky-explicit-grant-ok");
    assert_eq!(
        fs::read_to_string(&outside_write)?,
        "sticky-explicit-grant-ok"
    );

    Ok(())
}
