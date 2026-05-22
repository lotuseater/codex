#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::approvals_scenarios::scenarios;
use super::approvals_support::*;
use test_case::test_case;

#[test_case(ScenarioGroup::DangerFullAccess ; "danger_full_access")]
#[test_case(ScenarioGroup::ReadOnly ; "read_only")]
#[test_case(ScenarioGroup::WorkspaceWrite ; "workspace_write")]
#[test_case(ScenarioGroup::ApplyPatch ; "apply_patch")]
#[test_case(ScenarioGroup::UnifiedExec ; "unified_exec")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn approval_matrix_covers_group(group: ScenarioGroup) -> Result<()> {
    run_scenario_group(group).await
}

async fn run_scenario_group(group: ScenarioGroup) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let scenarios = scenarios()
        .into_iter()
        .filter(|scenario| scenario_group(scenario) == group)
        .collect::<Vec<_>>();
    assert!(!scenarios.is_empty(), "expected scenarios for {group:?}");

    for scenario in scenarios {
        run_scenario(&scenario)
            .await
            .with_context(|| format!("approval scenario failed: {}", scenario.name))?;
    }

    Ok(())
}

fn scenario_group(scenario: &ScenarioSpec) -> ScenarioGroup {
    match &scenario.action {
        ActionKind::ApplyPatchFreeform { .. } | ActionKind::ApplyPatchShell { .. } => {
            ScenarioGroup::ApplyPatch
        }
        ActionKind::RunUnifiedExecCommand { .. } => ScenarioGroup::UnifiedExec,
        ActionKind::WriteFile { .. }
        | ActionKind::FetchUrlNoProxy { .. }
        | ActionKind::FetchUrl { .. }
        | ActionKind::RunCommand { .. }
        | ActionKind::RunCommandWithPolicy { .. }
        | ActionKind::RunCommandWithPrefixRule { .. } => match &scenario.sandbox_policy {
            SandboxPolicy::DangerFullAccess => ScenarioGroup::DangerFullAccess,
            SandboxPolicy::ReadOnly { .. } => ScenarioGroup::ReadOnly,
            SandboxPolicy::WorkspaceWrite { .. } => ScenarioGroup::WorkspaceWrite,
            SandboxPolicy::ExternalSandbox { .. } => ScenarioGroup::WorkspaceWrite,
        },
    }
}

async fn run_scenario(scenario: &ScenarioSpec) -> Result<()> {
    eprintln!("running approval scenario: {}", scenario.name);
    let server = start_mock_server().await;
    let approval_policy = scenario.approval_policy;
    let sandbox_policy = scenario.sandbox_policy.clone();
    let features = scenario.features.clone();
    let model_override = scenario.model_override;
    let model = model_override.unwrap_or("gpt-5.4");
    let policy_src = scenario.action.policy_src();

    let mut builder = test_codex().with_model(model).with_config(move |config| {
        config.permissions.approval_policy = Constrained::allow_any(approval_policy);
        config
            .set_legacy_sandbox_policy(sandbox_policy.clone())
            .expect("set sandbox policy");
        for feature in features {
            config
                .features
                .enable(feature)
                .expect("test config should allow feature update");
        }
    });
    if let Some(policy_src) = policy_src {
        builder = builder.with_pre_build_hook(move |home| {
            let rules_dir = home.join("rules");
            fs::create_dir_all(&rules_dir).expect("create rules dir");
            fs::write(rules_dir.join("default.rules"), policy_src).expect("write policy");
        });
    }
    let test = builder.build(&server).await?;

    let call_id = scenario.name;
    let (event, expected_command) = scenario
        .action
        .prepare(&test, &server, call_id, scenario.sandbox_permissions)
        .await?;
    if let Some(command) = expected_command.as_deref() {
        eprintln!("approval scenario {} command: {command}", scenario.name);
    }

    let _ = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            event,
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let results_mock = mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    submit_turn(
        &test,
        scenario.name,
        scenario.approval_policy,
        scenario.sandbox_policy.clone(),
    )
    .await?;

    match &scenario.outcome {
        Outcome::Auto => {
            wait_for_completion_without_approval(&test).await;
        }
        Outcome::ExecApproval {
            decision,
            expected_reason,
        } => {
            let command = expected_command
                .as_deref()
                .expect("exec approval requires shell command");
            let approval = expect_exec_approval(&test, command).await;
            if let Some(expected_reason) = expected_reason {
                assert_eq!(
                    approval.reason.as_deref(),
                    Some(*expected_reason),
                    "unexpected approval reason for {}",
                    scenario.name
                );
            }
            test.codex
                .submit(Op::ExecApproval {
                    id: approval.effective_approval_id(),
                    turn_id: None,
                    decision: decision.clone(),
                })
                .await?;
            wait_for_completion(&test).await;
        }
        Outcome::ExecApprovalWithAmendment {
            decision,
            expected_reason,
            expected_execpolicy_amendment,
        } => {
            let command = expected_command
                .as_deref()
                .expect("exec approval requires shell command");
            let approval = expect_exec_approval(&test, command).await;
            if let Some(expected_reason) = expected_reason {
                assert_eq!(
                    approval.reason.as_deref(),
                    Some(*expected_reason),
                    "unexpected approval reason for {}",
                    scenario.name
                );
            }
            let expected_execpolicy_amendment = expected_execpolicy_amendment.map(|command| {
                ExecPolicyAmendment::new(command.iter().map(|part| (*part).to_string()).collect())
            });
            assert_eq!(
                approval.proposed_execpolicy_amendment, expected_execpolicy_amendment,
                "unexpected execpolicy amendment for {}",
                scenario.name
            );
            test.codex
                .submit(Op::ExecApproval {
                    id: approval.effective_approval_id(),
                    turn_id: None,
                    decision: decision.clone(),
                })
                .await?;
            wait_for_completion(&test).await;
        }
        Outcome::PatchApproval {
            decision,
            expected_reason,
        } => {
            let approval = expect_patch_approval(&test, call_id).await;
            if let Some(expected_reason) = expected_reason {
                assert_eq!(
                    approval.reason.as_deref(),
                    Some(*expected_reason),
                    "unexpected patch approval reason for {}",
                    scenario.name
                );
            }
            test.codex
                .submit(Op::PatchApproval {
                    id: approval.call_id,
                    decision: decision.clone(),
                })
                .await?;
            wait_for_completion(&test).await;
        }
    }

    let output_request = results_mock.single_request();
    let output_item = if matches!(scenario.action, ActionKind::ApplyPatchFreeform { .. }) {
        output_request.custom_tool_call_output(call_id)
    } else {
        output_request.function_call_output(call_id)
    };
    let result = parse_result(&output_item);
    eprintln!(
        "approval scenario {} result: exit_code={:?} stdout={:?}",
        scenario.name, result.exit_code, result.stdout
    );
    scenario.expectation.verify(&test, &result)?;

    Ok(())
}
