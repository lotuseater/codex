#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::approvals_support::*;

#[tokio::test(flavor = "current_thread")]
#[cfg(unix)]
async fn approving_apply_patch_for_session_skips_future_prompts_for_same_file() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let approval_policy = AskForApproval::OnRequest;
    let sandbox_policy = SandboxPolicy::WorkspaceWrite {
        writable_roots: vec![],
        network_access: false,
        exclude_tmpdir_env_var: false,
        exclude_slash_tmp: false,
    };
    let sandbox_policy_for_config = sandbox_policy.clone();

    let mut builder = test_codex()
        .with_model("gpt-5.4")
        .with_config(move |config| {
            config.permissions.approval_policy = Constrained::allow_any(approval_policy);
            config
                .set_legacy_sandbox_policy(sandbox_policy_for_config)
                .expect("set sandbox policy");
            config.approvals_reviewer = ApprovalsReviewer::User;
        });
    let test = builder.build(&server).await?;

    let target = TargetPath::OutsideWorkspace("apply_patch_allow_session.txt");
    let (path, patch_path) = target.resolve_for_patch(&test);
    let _ = fs::remove_file(&path);

    let patch_add = build_add_file_patch(&patch_path, "before");
    let patch_update = format!(
        "*** Begin Patch\n*** Update File: {patch_path}\n@@\n-before\n+after\n*** End Patch\n"
    );

    let call_id_1 = "apply_patch_allow_session_1";
    let call_id_2 = "apply_patch_allow_session_2";

    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_apply_patch_custom_tool_call(call_id_1, &patch_add),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    submit_turn(
        &test,
        "apply_patch allow session",
        approval_policy,
        sandbox_policy.clone(),
    )
    .await?;
    let approval = expect_patch_approval(&test, call_id_1).await;
    test.codex
        .submit(Op::PatchApproval {
            id: approval.call_id,
            decision: ReviewDecision::ApprovedForSession,
        })
        .await?;
    wait_for_completion(&test).await;
    assert!(fs::read_to_string(&path)?.contains("before"));

    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-3"),
            ev_apply_patch_custom_tool_call(call_id_2, &patch_update),
            ev_completed("resp-3"),
        ]),
    )
    .await;
    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-2", "done"),
            ev_completed("resp-4"),
        ]),
    )
    .await;

    submit_turn(
        &test,
        "apply_patch allow session followup",
        approval_policy,
        sandbox_policy.clone(),
    )
    .await?;

    let event = wait_for_event(&test.codex, |event| {
        matches!(
            event,
            EventMsg::ApplyPatchApprovalRequest(_) | EventMsg::TurnComplete(_)
        )
    })
    .await;
    match event {
        EventMsg::TurnComplete(_) => {}
        EventMsg::ApplyPatchApprovalRequest(event) => {
            panic!("unexpected patch approval request: {:?}", event.call_id)
        }
        other => panic!("unexpected event: {other:?}"),
    }

    assert!(fs::read_to_string(&path)?.contains("after"));
    let _ = fs::remove_file(path);

    Ok(())
}
