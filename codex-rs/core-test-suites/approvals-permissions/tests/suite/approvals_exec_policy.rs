#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::approvals_support::*;

#[tokio::test(flavor = "current_thread")]
#[cfg(unix)]
async fn approving_execpolicy_amendment_persists_policy_and_skips_future_prompts() -> Result<()> {
    let server = start_mock_server().await;
    let approval_policy = AskForApproval::UnlessTrusted;
    let sandbox_policy = SandboxPolicy::new_read_only_policy();
    let sandbox_policy_for_config = sandbox_policy.clone();
    let mut builder = test_codex().with_config(move |config| {
        config.permissions.approval_policy = Constrained::allow_any(approval_policy);
        config
            .set_legacy_sandbox_policy(sandbox_policy_for_config)
            .expect("set sandbox policy");
    });
    let test = builder.build(&server).await?;
    let allow_prefix_path = test.cwd.path().join("allow-prefix.txt");
    let _ = fs::remove_file(&allow_prefix_path);

    let call_id_first = "allow-prefix-first";
    let (first_event, expected_command) = ActionKind::RunCommand {
        command: "touch allow-prefix.txt",
    }
    .prepare(
        &test,
        &server,
        call_id_first,
        SandboxPermissions::UseDefault,
    )
    .await?;
    let expected_command =
        expected_command.expect("execpolicy amendment scenario should produce a shell command");
    let expected_execpolicy_amendment =
        ExecPolicyAmendment::new(vec!["touch".to_string(), "allow-prefix.txt".to_string()]);

    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-allow-prefix-1"),
            first_event,
            ev_completed("resp-allow-prefix-1"),
        ]),
    )
    .await;
    let first_results = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-allow-prefix-1", "done"),
            ev_completed("resp-allow-prefix-2"),
        ]),
    )
    .await;

    submit_turn(
        &test,
        "allow-prefix-first",
        approval_policy,
        sandbox_policy.clone(),
    )
    .await?;

    let approval = expect_exec_approval(&test, expected_command.as_str()).await;
    assert_eq!(
        approval.proposed_execpolicy_amendment,
        Some(expected_execpolicy_amendment.clone())
    );

    test.codex
        .submit(Op::ExecApproval {
            id: approval.effective_approval_id(),
            turn_id: None,
            decision: ReviewDecision::ApprovedExecpolicyAmendment {
                proposed_execpolicy_amendment: expected_execpolicy_amendment.clone(),
            },
        })
        .await?;
    wait_for_completion(&test).await;

    let developer_messages = first_results
        .single_request()
        .message_input_texts("developer");
    assert!(
        developer_messages
            .iter()
            .any(|message| message.contains(r#"["touch", "allow-prefix.txt"]"#)),
        "expected developer message documenting saved rule, got: {developer_messages:?}"
    );

    let policy_path = test.home.path().join("rules").join("default.rules");
    let policy_contents = fs::read_to_string(&policy_path)?;
    assert!(
        policy_contents
            .contains(r#"prefix_rule(pattern=["touch", "allow-prefix.txt"], decision="allow")"#),
        "unexpected policy contents: {policy_contents}"
    );

    let first_output = parse_result(
        &first_results
            .single_request()
            .function_call_output(call_id_first),
    );
    assert_eq!(first_output.exit_code.unwrap_or(0), 0);
    assert!(
        first_output.stdout.is_empty(),
        "unexpected stdout: {}",
        first_output.stdout
    );
    assert_eq!(
        fs::read_to_string(&allow_prefix_path)?,
        "",
        "unexpected file contents after first run"
    );

    let call_id_second = "allow-prefix-second";
    let (second_event, second_command) = ActionKind::RunCommand {
        command: "touch allow-prefix.txt",
    }
    .prepare(
        &test,
        &server,
        call_id_second,
        SandboxPermissions::UseDefault,
    )
    .await?;
    assert_eq!(second_command.as_deref(), Some(expected_command.as_str()));

    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-allow-prefix-3"),
            second_event,
            ev_completed("resp-allow-prefix-3"),
        ]),
    )
    .await;
    let second_results = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-allow-prefix-2", "done"),
            ev_completed("resp-allow-prefix-4"),
        ]),
    )
    .await;

    submit_turn(
        &test,
        "allow-prefix-second",
        approval_policy,
        sandbox_policy.clone(),
    )
    .await?;

    wait_for_completion_without_approval(&test).await;

    let second_output = parse_result(
        &second_results
            .single_request()
            .function_call_output(call_id_second),
    );
    assert_eq!(second_output.exit_code.unwrap_or(0), 0);
    assert!(
        second_output.stdout.is_empty(),
        "unexpected stdout: {}",
        second_output.stdout
    );
    assert_eq!(
        fs::read_to_string(&allow_prefix_path)?,
        "",
        "unexpected file contents after second run"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spawned_subagent_execpolicy_amendment_propagates_to_parent_session() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let approval_policy = AskForApproval::UnlessTrusted;
    let sandbox_policy = SandboxPolicy::new_read_only_policy();
    let sandbox_policy_for_config = sandbox_policy.clone();
    let mut builder = test_codex().with_config(move |config| {
        config.permissions.approval_policy = Constrained::allow_any(approval_policy);
        config
            .set_legacy_sandbox_policy(sandbox_policy_for_config)
            .expect("set sandbox policy");
        config
            .features
            .enable(Feature::Collab)
            .expect("test config should allow feature update");
    });
    let test = builder.build(&server).await?;

    const PARENT_PROMPT: &str = "spawn a child that repeats a command";
    const CHILD_PROMPT: &str = "run the same command twice";
    const SPAWN_CALL_ID: &str = "spawn-child-1";
    const CHILD_CALL_ID_1: &str = "child-touch-1";
    const PARENT_CALL_ID_2: &str = "parent-touch-2";

    let child_file = test.cwd.path().join("subagent-allow-prefix.txt");
    let _ = fs::remove_file(&child_file);

    let spawn_args = serde_json::to_string(&json!({
        "message": CHILD_PROMPT,
    }))?;
    mount_sse_once_match(
        &server,
        |req: &Request| body_contains(req, PARENT_PROMPT),
        sse(vec![
            ev_response_created("resp-parent-1"),
            ev_function_call(SPAWN_CALL_ID, "spawn_agent", &spawn_args),
            ev_completed("resp-parent-1"),
        ]),
    )
    .await;

    let child_cmd_args = serde_json::to_string(&json!({
        "command": "touch subagent-allow-prefix.txt",
        "timeout_ms": 1_000,
        "prefix_rule": ["touch", "subagent-allow-prefix.txt"],
    }))?;
    mount_sse_once_match(
        &server,
        |req: &Request| body_contains(req, CHILD_PROMPT) && !body_contains(req, SPAWN_CALL_ID),
        sse(vec![
            ev_response_created("resp-child-1"),
            ev_function_call(CHILD_CALL_ID_1, "shell_command", &child_cmd_args),
            ev_completed("resp-child-1"),
        ]),
    )
    .await;

    mount_sse_once_match(
        &server,
        |req: &Request| body_contains(req, CHILD_CALL_ID_1),
        sse(vec![
            ev_response_created("resp-child-2"),
            ev_assistant_message("msg-child-2", "child done"),
            ev_completed("resp-child-2"),
        ]),
    )
    .await;

    mount_sse_once_match(
        &server,
        |req: &Request| body_contains(req, SPAWN_CALL_ID),
        sse(vec![
            ev_response_created("resp-parent-2"),
            ev_assistant_message("msg-parent-2", "parent done"),
            ev_completed("resp-parent-2"),
        ]),
    )
    .await;

    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-parent-3"),
            ev_function_call(PARENT_CALL_ID_2, "shell_command", &child_cmd_args),
            ev_completed("resp-parent-3"),
        ]),
    )
    .await;

    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-parent-4"),
            ev_assistant_message("msg-parent-4", "parent rerun done"),
            ev_completed("resp-parent-4"),
        ]),
    )
    .await;

    submit_turn(
        &test,
        PARENT_PROMPT,
        approval_policy,
        sandbox_policy.clone(),
    )
    .await?;

    let child = wait_for_spawned_thread(&test).await?;
    let approval_event = wait_for_event_with_timeout(
        &child,
        |event| {
            matches!(
                event,
                EventMsg::ExecApprovalRequest(_) | EventMsg::TurnComplete(_)
            )
        },
        Duration::from_secs(2),
    )
    .await;

    let EventMsg::ExecApprovalRequest(approval) = approval_event else {
        panic!("expected child approval before completion");
    };
    let expected_execpolicy_amendment = ExecPolicyAmendment::new(vec![
        "touch".to_string(),
        "subagent-allow-prefix.txt".to_string(),
    ]);
    assert_eq!(
        approval.proposed_execpolicy_amendment,
        Some(expected_execpolicy_amendment.clone())
    );

    child
        .submit(Op::ExecApproval {
            id: approval.effective_approval_id(),
            turn_id: None,
            decision: ReviewDecision::ApprovedExecpolicyAmendment {
                proposed_execpolicy_amendment: expected_execpolicy_amendment,
            },
        })
        .await?;

    let child_event = wait_for_event_with_timeout(
        &child,
        |event| {
            matches!(
                event,
                EventMsg::ExecApprovalRequest(_) | EventMsg::TurnComplete(_)
            )
        },
        Duration::from_secs(2),
    )
    .await;
    match child_event {
        EventMsg::TurnComplete(_) => {}
        EventMsg::ExecApprovalRequest(ev) => {
            panic!("unexpected second child approval request: {:?}", ev.command)
        }
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(
        child_file.exists(),
        "expected subagent command to create file"
    );
    fs::remove_file(&child_file)?;
    assert!(
        !child_file.exists(),
        "expected child file to be removed before parent rerun"
    );

    submit_turn(
        &test,
        "parent reruns child command",
        approval_policy,
        sandbox_policy,
    )
    .await?;
    wait_for_completion_without_approval(&test).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[cfg(unix)]
async fn matched_prefix_rule_runs_unsandboxed_under_zsh_fork() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let Some(runtime) = zsh_fork_runtime("zsh-fork prefix rule unsandboxed test")? else {
        return Ok(());
    };

    let approval_policy = AskForApproval::Never;
    let permission_profile = restrictive_workspace_write_profile();
    let outside_dir = tempfile::tempdir_in(std::env::current_dir()?)?;
    let outside_path = outside_dir
        .path()
        .join("zsh-fork-prefix-rule-unsandboxed.txt");
    let command = format!("touch {outside_path:?}");
    let rules = r#"prefix_rule(pattern=["touch"], decision="allow")"#.to_string();

    let server = start_mock_server().await;
    let outside_path_for_hook = outside_path.clone();
    let test = build_zsh_fork_test(
        &server,
        runtime,
        approval_policy,
        permission_profile.clone(),
        move |home| {
            let _ = fs::remove_file(&outside_path_for_hook);
            let rules_dir = home.join("rules");
            fs::create_dir_all(&rules_dir).unwrap();
            fs::write(rules_dir.join("default.rules"), &rules).unwrap();
        },
    )
    .await?;

    let call_id = "zsh-fork-prefix-rule-unsandboxed";
    let event = shell_event(
        call_id,
        &command,
        /*timeout_ms*/ 1_000,
        SandboxPermissions::UseDefault,
    )?;
    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-zsh-fork-prefix-1"),
            event,
            ev_completed("resp-zsh-fork-prefix-1"),
        ]),
    )
    .await;
    let results = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-zsh-fork-prefix-1", "done"),
            ev_completed("resp-zsh-fork-prefix-2"),
        ]),
    )
    .await;

    let session_model = test.session_configured.model.clone();
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(permission_profile, test.cwd.path());
    test.codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: "run allowed touch under zsh fork".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: test.cwd.path().to_path_buf(),
            approval_policy,
            approvals_reviewer: Some(ApprovalsReviewer::User),
            sandbox_policy,
            permission_profile,
            model: session_model,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            personality: None,
        })
        .await?;

    wait_for_completion_without_approval(&test).await;

    let result = parse_result(&results.single_request().function_call_output(call_id));
    assert_eq!(result.exit_code.unwrap_or(0), 0);
    assert!(
        outside_path.exists(),
        "expected matched prefix_rule to rerun touch unsandboxed; output: {}",
        result.stdout
    );

    Ok(())
}

#[tokio::test(flavor = "current_thread")]
#[cfg(unix)]
async fn invalid_requested_prefix_rule_falls_back_for_compound_command() -> Result<()> {
    let server = start_mock_server().await;
    let approval_policy = AskForApproval::OnRequest;
    let sandbox_policy = SandboxPolicy::new_read_only_policy();
    let sandbox_policy_for_config = sandbox_policy.clone();
    let mut builder = test_codex().with_config(move |config| {
        config.permissions.approval_policy = Constrained::allow_any(approval_policy);
        config
            .set_legacy_sandbox_policy(sandbox_policy_for_config)
            .expect("set sandbox policy");
    });
    let test = builder.build(&server).await?;

    let call_id = "invalid-prefix-rule";
    let command =
        "touch /tmp/codex-fallback-rule-test.txt && echo hello > /tmp/codex-fallback-rule-test.txt";
    let event = shell_event_with_prefix_rule(
        call_id,
        command,
        /*timeout_ms*/ 1_000,
        SandboxPermissions::RequireEscalated,
        Some(vec!["touch".to_string()]),
    )?;

    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-invalid-prefix-1"),
            event,
            ev_completed("resp-invalid-prefix-1"),
        ]),
    )
    .await;

    submit_turn(
        &test,
        "invalid-prefix-rule",
        approval_policy,
        sandbox_policy.clone(),
    )
    .await?;

    let approval = expect_exec_approval(&test, command).await;
    let amendment = approval
        .proposed_execpolicy_amendment
        .expect("should have a proposed execpolicy amendment");
    assert!(amendment.command.contains(&command.to_string()));

    Ok(())
}

#[tokio::test(flavor = "current_thread")]
#[cfg(unix)]
async fn approving_fallback_rule_for_compound_command_works() -> Result<()> {
    let server = start_mock_server().await;
    let approval_policy = AskForApproval::OnRequest;
    let sandbox_policy = SandboxPolicy::new_read_only_policy();
    let sandbox_policy_for_config = sandbox_policy.clone();
    let mut builder = test_codex().with_config(move |config| {
        config.permissions.approval_policy = Constrained::allow_any(approval_policy);
        config
            .set_legacy_sandbox_policy(sandbox_policy_for_config)
            .expect("set sandbox policy");
    });
    let test = builder.build(&server).await?;

    let call_id = "invalid-prefix-rule";
    let command =
        "touch /tmp/codex-fallback-rule-test.txt && echo hello > /tmp/codex-fallback-rule-test.txt";
    let event = shell_event_with_prefix_rule(
        call_id,
        command,
        /*timeout_ms*/ 1_000,
        SandboxPermissions::RequireEscalated,
        Some(vec!["touch".to_string()]),
    )?;

    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-invalid-prefix-1"),
            event,
            ev_completed("resp-invalid-prefix-1"),
        ]),
    )
    .await;

    submit_turn(
        &test,
        "invalid-prefix-rule",
        approval_policy,
        sandbox_policy.clone(),
    )
    .await?;

    let approval = expect_exec_approval(&test, command).await;
    let approval_id = approval.effective_approval_id();
    let amendment = approval
        .proposed_execpolicy_amendment
        .expect("should have a proposed execpolicy amendment");
    assert!(amendment.command.contains(&command.to_string()));

    test.codex
        .submit(Op::ExecApproval {
            id: approval_id,
            turn_id: None,
            decision: ReviewDecision::ApprovedExecpolicyAmendment {
                proposed_execpolicy_amendment: amendment.clone(),
            },
        })
        .await?;
    wait_for_completion(&test).await;

    let call_id = "invalid-prefix-rule-again";
    let command =
        "touch /tmp/codex-fallback-rule-test.txt && echo hello > /tmp/codex-fallback-rule-test.txt";
    let event = shell_event_with_prefix_rule(
        call_id,
        command,
        /*timeout_ms*/ 1_000,
        SandboxPermissions::RequireEscalated,
        Some(vec!["touch".to_string()]),
    )?;

    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-invalid-prefix-1"),
            event,
            ev_completed("resp-invalid-prefix-1"),
        ]),
    )
    .await;
    let second_results = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-invalid-prefix-1", "done"),
            ev_completed("resp-invalid-prefix-2"),
        ]),
    )
    .await;

    submit_turn(
        &test,
        "invalid-prefix-rule",
        approval_policy,
        sandbox_policy.clone(),
    )
    .await?;

    wait_for_completion_without_approval(&test).await;

    let second_output = parse_result(
        &second_results
            .single_request()
            .function_call_output(call_id),
    );
    assert_eq!(second_output.exit_code.unwrap_or(0), 0);
    assert!(
        second_output.stdout.is_empty(),
        "unexpected stdout: {}",
        second_output.stdout
    );

    Ok(())
}

// todo(dylan) add ScenarioSpec support for rules
#[tokio::test(flavor = "current_thread")]
#[cfg(unix)]
async fn compound_command_with_one_safe_command_still_requires_approval() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let approval_policy = AskForApproval::UnlessTrusted;
    let sandbox_policy = SandboxPolicy::new_workspace_write_policy();
    let sandbox_policy_for_config = sandbox_policy.clone();
    let mut builder = test_codex().with_config(move |config| {
        config.permissions.approval_policy = Constrained::allow_any(approval_policy);
        config
            .set_legacy_sandbox_policy(sandbox_policy_for_config)
            .expect("set sandbox policy");
    });
    let test = builder.build(&server).await?;

    let rules_dir = test.home.path().join("rules");
    fs::create_dir_all(&rules_dir)?;
    fs::write(
        rules_dir.join("default.rules"),
        r#"prefix_rule(pattern=["touch", "allow-prefix.txt"], decision="allow")"#,
    )?;

    let call_id = "heredoc-with-chained-prefix";
    let command = "touch ./test.txt && rm ./test.txt";
    let (event, expected_command) = ActionKind::RunCommand { command }
        .prepare(&test, &server, call_id, SandboxPermissions::UseDefault)
        .await?;
    let expected_command =
        expected_command.expect("compound command should produce a shell command");

    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-heredoc-prefix-1"),
            event,
            ev_completed("resp-heredoc-prefix-1"),
        ]),
    )
    .await;
    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-heredoc-prefix-1", "done"),
            ev_completed("resp-heredoc-prefix-2"),
        ]),
    )
    .await;

    submit_turn(
        &test,
        "compound command",
        approval_policy,
        sandbox_policy.clone(),
    )
    .await?;

    let approval = expect_exec_approval(&test, expected_command.as_str()).await;
    test.codex
        .submit(Op::ExecApproval {
            id: approval.effective_approval_id(),
            turn_id: None,
            decision: ReviewDecision::Denied,
        })
        .await?;
    wait_for_completion(&test).await;

    Ok(())
}
