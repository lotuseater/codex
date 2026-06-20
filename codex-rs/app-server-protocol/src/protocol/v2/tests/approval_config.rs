use super::*;
use pretty_assertions::assert_eq;

#[test]
fn approvals_reviewer_serializes_auto_review_and_accepts_legacy_guardian_subagent() {
    assert_eq!(
        serde_json::to_string(&ApprovalsReviewer::User).expect("serialize reviewer"),
        "\"user\""
    );
    assert_eq!(
        serde_json::to_string(&ApprovalsReviewer::AutoReview).expect("serialize reviewer"),
        "\"guardian_subagent\""
    );

    for value in ["user", "auto_review", "guardian_subagent"] {
        let json = format!("\"{value}\"");
        let reviewer: ApprovalsReviewer =
            serde_json::from_str(&json).expect("deserialize reviewer");
        let expected = if value == "user" {
            ApprovalsReviewer::User
        } else {
            ApprovalsReviewer::AutoReview
        };
        assert_eq!(expected, reviewer);
    }
}

#[test]
fn ask_for_approval_granular_round_trips_request_permissions_flag() {
    let v2_policy = AskForApproval::Granular {
        sandbox_approval: true,
        rules: false,
        skill_approval: false,
        request_permissions: true,
        mcp_elicitations: false,
    };

    let core_policy = v2_policy.to_core();
    assert_eq!(
        core_policy,
        CoreAskForApproval::Granular(CoreGranularApprovalConfig {
            sandbox_approval: true,
            rules: false,
            skill_approval: false,
            request_permissions: true,
            mcp_elicitations: false,
        })
    );

    let back_to_v2 = AskForApproval::from(core_policy);
    assert_eq!(back_to_v2, v2_policy);
}

#[test]
fn ask_for_approval_granular_defaults_missing_optional_flags_to_false() {
    let decoded = serde_json::from_value::<AskForApproval>(serde_json::json!({
        "granular": {
            "sandbox_approval": true,
            "rules": false,
            "mcp_elicitations": true,
        }
    }))
    .expect("granular approval policy should deserialize");

    assert_eq!(
        decoded,
        AskForApproval::Granular {
            sandbox_approval: true,
            rules: false,
            skill_approval: false,
            request_permissions: false,
            mcp_elicitations: true,
        }
    );
}

#[test]
fn ask_for_approval_granular_is_marked_experimental() {
    let reason =
        crate::experimental_api::ExperimentalApi::experimental_reason(&AskForApproval::Granular {
            sandbox_approval: true,
            rules: false,
            skill_approval: false,
            request_permissions: false,
            mcp_elicitations: true,
        });

    assert_eq!(reason, Some("askForApproval.granular"));
    assert_eq!(
        crate::experimental_api::ExperimentalApi::experimental_reason(&AskForApproval::OnRequest,),
        None
    );
}

#[test]
fn config_granular_approval_policy_is_marked_experimental() {
    let reason = crate::experimental_api::ExperimentalApi::experimental_reason(&Config {
        model: None,
        review_model: None,
        model_context_window: None,
        model_auto_compact_token_limit: None,
        model_provider: None,
        approval_policy: Some(AskForApproval::Granular {
            sandbox_approval: false,
            rules: true,
            skill_approval: false,
            request_permissions: false,
            mcp_elicitations: true,
        }),
        approvals_reviewer: None,
        sandbox_mode: None,
        sandbox_workspace_write: None,
        forced_chatgpt_workspace_id: None,
        forced_login_method: None,
        web_search: None,
        tools: None,
        instructions: None,
        developer_instructions: None,
        compact_prompt: None,
        model_reasoning_effort: None,
        model_reasoning_summary: None,
        model_verbosity: None,
        service_tier: None,
        analytics: None,
        apps: None,
        additional: HashMap::new(),
    });

    assert_eq!(reason, Some("askForApproval.granular"));
}

#[test]
fn config_approvals_reviewer_is_marked_experimental() {
    let reason = crate::experimental_api::ExperimentalApi::experimental_reason(&Config {
        model: None,
        review_model: None,
        model_context_window: None,
        model_auto_compact_token_limit: None,
        model_provider: None,
        approval_policy: None,
        approvals_reviewer: Some(ApprovalsReviewer::AutoReview),
        sandbox_mode: None,
        sandbox_workspace_write: None,
        forced_chatgpt_workspace_id: None,
        forced_login_method: None,
        web_search: None,
        tools: None,
        profile: None,
        profiles: HashMap::new(),
        instructions: None,
        developer_instructions: None,
        compact_prompt: None,
        model_reasoning_effort: None,
        model_reasoning_summary: None,
        model_verbosity: None,
        service_tier: None,
        analytics: None,
        apps: None,
        additional: HashMap::new(),
    });

    assert_eq!(reason, Some("config/read.approvalsReviewer"));
}

#[test]
fn config_nested_profile_granular_approval_policy_is_marked_experimental() {
    let reason = crate::experimental_api::ExperimentalApi::experimental_reason(&Config {
        model: None,
        review_model: None,
        model_context_window: None,
        model_auto_compact_token_limit: None,
        model_provider: None,
        approval_policy: None,
        approvals_reviewer: None,
        sandbox_mode: None,
        sandbox_workspace_write: None,
        forced_chatgpt_workspace_id: None,
        forced_login_method: None,
        web_search: None,
        tools: None,
        profile: None,
        profiles: HashMap::from([(
            "default".to_string(),
            ProfileV2 {
                model: None,
                model_provider: None,
                approval_policy: Some(AskForApproval::Granular {
                    sandbox_approval: true,
                    rules: false,
                    skill_approval: false,
                    request_permissions: false,
                    mcp_elicitations: true,
                }),
                approvals_reviewer: None,
                service_tier: None,
                model_reasoning_effort: None,
                model_reasoning_summary: None,
                model_verbosity: None,
                web_search: None,
                tools: None,
                chatgpt_base_url: None,
                additional: HashMap::new(),
            },
        )]),
        instructions: None,
        developer_instructions: None,
        compact_prompt: None,
        model_reasoning_effort: None,
        model_reasoning_summary: None,
        model_verbosity: None,
        service_tier: None,
        analytics: None,
        apps: None,
        additional: HashMap::new(),
    });

    assert_eq!(reason, Some("askForApproval.granular"));
}

#[test]
fn config_nested_profile_approvals_reviewer_is_marked_experimental() {
    let reason = crate::experimental_api::ExperimentalApi::experimental_reason(&Config {
        model: None,
        review_model: None,
        model_context_window: None,
        model_auto_compact_token_limit: None,
        model_provider: None,
        approval_policy: None,
        approvals_reviewer: None,
        sandbox_mode: None,
        sandbox_workspace_write: None,
        forced_chatgpt_workspace_id: None,
        forced_login_method: None,
        web_search: None,
        tools: None,
        profile: None,
        profiles: HashMap::from([(
            "default".to_string(),
            ProfileV2 {
                model: None,
                model_provider: None,
                approval_policy: None,
                approvals_reviewer: Some(ApprovalsReviewer::AutoReview),
                service_tier: None,
                model_reasoning_effort: None,
                model_reasoning_summary: None,
                model_verbosity: None,
                web_search: None,
                tools: None,
                chatgpt_base_url: None,
                additional: HashMap::new(),
            },
        )]),
        instructions: None,
        developer_instructions: None,
        compact_prompt: None,
        model_reasoning_effort: None,
        model_reasoning_summary: None,
        model_verbosity: None,
        service_tier: None,
        analytics: None,
        apps: None,
        additional: HashMap::new(),
    });

    assert_eq!(reason, Some("config/read.approvalsReviewer"));
}

#[test]
fn config_requirements_granular_allowed_approval_policy_is_marked_experimental() {
    let reason =
        crate::experimental_api::ExperimentalApi::experimental_reason(&ConfigRequirements {
            allowed_approval_policies: Some(vec![AskForApproval::Granular {
                sandbox_approval: true,
                rules: true,
                skill_approval: false,
                request_permissions: false,
                mcp_elicitations: false,
            }]),
            allowed_approvals_reviewers: None,
            allowed_sandbox_modes: None,
            allowed_permissions: None,
            allowed_web_search_modes: None,
            allow_managed_hooks_only: None,
            allow_appshots: None,
            computer_use: None,
            feature_requirements: None,
            hooks: None,
            enforce_residency: None,
            network: None,
        });

    assert_eq!(reason, Some("askForApproval.granular"));
}

#[test]
fn client_request_thread_start_granular_approval_policy_is_marked_experimental() {
    let reason = crate::experimental_api::ExperimentalApi::experimental_reason(
        &crate::ClientRequest::ThreadStart {
            request_id: crate::RequestId::Integer(1),
            params: ThreadStartParams {
                approval_policy: Some(AskForApproval::Granular {
                    sandbox_approval: true,
                    rules: false,
                    skill_approval: false,
                    request_permissions: true,
                    mcp_elicitations: false,
                }),
                ..Default::default()
            },
        },
    );

    assert_eq!(reason, Some("askForApproval.granular"));
}

#[test]
fn client_request_thread_resume_granular_approval_policy_is_marked_experimental() {
    let reason = crate::experimental_api::ExperimentalApi::experimental_reason(
        &crate::ClientRequest::ThreadResume {
            request_id: crate::RequestId::Integer(2),
            params: ThreadResumeParams {
                thread_id: "thr_123".to_string(),
                approval_policy: Some(AskForApproval::Granular {
                    sandbox_approval: false,
                    rules: true,
                    skill_approval: false,
                    request_permissions: false,
                    mcp_elicitations: true,
                }),
                ..Default::default()
            },
        },
    );

    assert_eq!(reason, Some("askForApproval.granular"));
}

#[test]
fn client_request_thread_fork_granular_approval_policy_is_marked_experimental() {
    let reason = crate::experimental_api::ExperimentalApi::experimental_reason(
        &crate::ClientRequest::ThreadFork {
            request_id: crate::RequestId::Integer(3),
            params: ThreadForkParams {
                thread_id: "thr_456".to_string(),
                approval_policy: Some(AskForApproval::Granular {
                    sandbox_approval: true,
                    rules: false,
                    skill_approval: false,
                    request_permissions: false,
                    mcp_elicitations: true,
                }),
                ..Default::default()
            },
        },
    );

    assert_eq!(reason, Some("askForApproval.granular"));
}

#[test]
fn client_request_turn_start_granular_approval_policy_is_marked_experimental() {
    let reason = crate::experimental_api::ExperimentalApi::experimental_reason(
        &crate::ClientRequest::TurnStart {
            request_id: crate::RequestId::Integer(4),
            params: TurnStartParams {
                thread_id: "thr_123".to_string(),
                input: Vec::new(),
                approval_policy: Some(AskForApproval::Granular {
                    sandbox_approval: false,
                    rules: true,
                    skill_approval: false,
                    request_permissions: false,
                    mcp_elicitations: true,
                }),
                ..Default::default()
            },
        },
    );

    assert_eq!(reason, Some("askForApproval.granular"));
}

#[test]
fn automatic_approval_review_deserializes_aborted_status() {
    let review: GuardianApprovalReview = serde_json::from_value(json!({
        "status": "aborted",
        "riskLevel": null,
        "userAuthorization": null,
        "rationale": null
    }))
    .expect("aborted automatic review should deserialize");
    assert_eq!(
        review,
        GuardianApprovalReview {
            status: GuardianApprovalReviewStatus::Aborted,
            risk_level: None,
            user_authorization: None,
            rationale: None,
        }
    );
}

#[test]
fn guardian_approval_review_action_round_trips_command_shape() {
    let value = json!({
        "type": "command",
        "source": "shell",
        "command": "rm -rf /tmp/example.sqlite",
        "cwd": absolute_path_string("tmp"),
    });
    let action: GuardianApprovalReviewAction =
        serde_json::from_value(value.clone()).expect("guardian review action");

    assert_eq!(
        action,
        GuardianApprovalReviewAction::Command {
            source: GuardianCommandSource::Shell,
            command: "rm -rf /tmp/example.sqlite".to_string(),
            cwd: absolute_path("tmp"),
        }
    );
    assert_eq!(
        serde_json::to_value(&action).expect("serialize guardian review action"),
        value
    );
}
