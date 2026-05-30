use super::*;

#[test]
fn turn_start_params_round_trips_context_budget_mode() {
    let params = TurnStartParams {
        thread_id: "thr_123".to_string(),
        input: Vec::new(),
        context_budget_mode: Some(ContextBudgetMode::Slow),
        ..Default::default()
    };

    let value = serde_json::to_value(&params).expect("serialize turn/start params");
    assert_eq!(value["contextBudgetMode"], json!("slow"));

    let decoded: TurnStartParams = serde_json::from_value(json!({
        "threadId": "thr_123",
        "input": [],
        "contextBudgetMode": "standard"
    }))
    .expect("deserialize turn/start params");

    assert_eq!(
        decoded.context_budget_mode,
        Some(ContextBudgetMode::Standard)
    );
}

#[test]
fn thread_start_params_preserve_explicit_null_service_tier() {
    let params: ThreadStartParams =
        serde_json::from_value(json!({ "serviceTier": null })).expect("params should deserialize");
    assert_eq!(params.service_tier, Some(None));

    let serialized = serde_json::to_value(&params).expect("params should serialize");
    assert_eq!(
        serialized.get("serviceTier"),
        Some(&serde_json::Value::Null)
    );

    let serialized_without_override =
        serde_json::to_value(ThreadStartParams::default()).expect("params should serialize");
    assert_eq!(serialized_without_override.get("serviceTier"), None);
}

#[test]
fn thread_lifecycle_responses_default_missing_optional_fields() {
    let response = json!({
        "thread": {
            "id": "thread-id",
            "sessionId": "thread-id",
            "forkedFromId": null,
            "preview": "",
            "ephemeral": false,
            "modelProvider": "openai",
            "createdAt": 1,
            "updatedAt": 1,
            "status": { "type": "idle" },
            "path": null,
            "cwd": absolute_path_string("tmp"),
            "cliVersion": "0.0.0",
            "source": "exec",
            "agentNickname": null,
            "agentRole": null,
            "gitInfo": null,
            "name": null,
            "turns": []
        },
        "model": "gpt-5",
        "modelProvider": "openai",
        "serviceTier": null,
        "cwd": absolute_path_string("tmp"),
        "approvalPolicy": "on-failure",
        "approvalsReviewer": "user",
        "sandbox": { "type": "dangerFullAccess" },
        "reasoningEffort": null
    });

    let start: ThreadStartResponse =
        serde_json::from_value(response.clone()).expect("thread/start response");
    let resume: ThreadResumeResponse =
        serde_json::from_value(response.clone()).expect("thread/resume response");
    let fork: ThreadForkResponse = serde_json::from_value(response).expect("thread/fork response");

    assert_eq!(start.instruction_sources, Vec::<AbsolutePathBuf>::new());
    assert_eq!(resume.instruction_sources, Vec::<AbsolutePathBuf>::new());
    assert_eq!(fork.instruction_sources, Vec::<AbsolutePathBuf>::new());
    assert_eq!(start.active_permission_profile, None);
    assert_eq!(resume.active_permission_profile, None);
    assert_eq!(fork.active_permission_profile, None);
}

#[test]
fn turn_start_params_preserve_explicit_null_service_tier() {
    let params: TurnStartParams = serde_json::from_value(json!({
        "threadId": "thread_123",
        "input": [],
        "serviceTier": null
    }))
    .expect("params should deserialize");
    assert_eq!(params.service_tier, Some(None));

    let serialized = serde_json::to_value(&params).expect("params should serialize");
    assert_eq!(
        serialized.get("serviceTier"),
        Some(&serde_json::Value::Null)
    );

    let without_override = TurnStartParams {
        thread_id: "thread_123".to_string(),
        input: vec![],
        responsesapi_client_metadata: None,
        environments: None,
        cwd: None,
        runtime_workspace_roots: None,
        approval_policy: None,
        approvals_reviewer: None,
        sandbox_policy: None,
        permissions: None,
        model: None,
        service_tier: None,
        context_budget_mode: None,
        effort: None,
        summary: None,
        output_schema: None,
        collaboration_mode: None,
        personality: None,
    };
    let serialized_without_override =
        serde_json::to_value(&without_override).expect("params should serialize");
    assert_eq!(serialized_without_override.get("serviceTier"), None);
}

#[test]
fn thread_settings_update_params_preserve_explicit_null_service_tier() {
    let params: ThreadSettingsUpdateParams = serde_json::from_value(json!({
        "threadId": "thread_123",
        "serviceTier": null
    }))
    .expect("params should deserialize");
    assert_eq!(params.service_tier, Some(None));

    let serialized = serde_json::to_value(&params).expect("params should serialize");
    assert_eq!(
        serialized.get("serviceTier"),
        Some(&serde_json::Value::Null)
    );

    let without_override = ThreadSettingsUpdateParams {
        thread_id: "thread_123".to_string(),
        service_tier: None,
        ..Default::default()
    };
    let serialized_without_override =
        serde_json::to_value(&without_override).expect("params should serialize");
    assert_eq!(serialized_without_override.get("serviceTier"), None);
}

#[test]
fn thread_settings_update_params_preserve_field_level_experimental_gates() {
    let permissions = ThreadSettingsUpdateParams {
        thread_id: "thread_123".to_string(),
        permissions: Some(":workspace".to_string()),
        ..Default::default()
    };
    assert_eq!(
        crate::experimental_api::ExperimentalApi::experimental_reason(&permissions),
        Some("thread/settings/update.permissions")
    );

    let granular_approval = ThreadSettingsUpdateParams {
        thread_id: "thread_123".to_string(),
        approval_policy: Some(AskForApproval::Granular {
            sandbox_approval: true,
            rules: true,
            skill_approval: false,
            request_permissions: false,
            mcp_elicitations: true,
        }),
        ..Default::default()
    };
    assert_eq!(
        crate::experimental_api::ExperimentalApi::experimental_reason(&granular_approval),
        Some("askForApproval.granular")
    );

    let collaboration_mode = ThreadSettingsUpdateParams {
        thread_id: "thread_123".to_string(),
        collaboration_mode: Some(codex_protocol::config_types::CollaborationMode {
            mode: codex_protocol::config_types::ModeKind::Plan,
            settings: codex_protocol::config_types::Settings {
                model: "mock-model".to_string(),
                reasoning_effort: None,
                developer_instructions: None,
            },
        }),
        ..Default::default()
    };
    assert_eq!(
        crate::experimental_api::ExperimentalApi::experimental_reason(&collaboration_mode),
        Some("thread/settings/update.collaborationMode")
    );
}

#[test]
fn turn_start_params_round_trip_environments() {
    let cwd = test_absolute_path();
    let params: TurnStartParams = serde_json::from_value(json!({
        "threadId": "thread_123",
        "input": [],
        "environments": [
            {
                "environmentId": "local",
                "cwd": cwd
            }
        ],
    }))
    .expect("params should deserialize");

    assert_eq!(
        params.environments,
        Some(vec![TurnEnvironmentParams {
            environment_id: "local".to_string(),
            cwd: cwd.clone(),
        }])
    );
    assert_eq!(
        crate::experimental_api::ExperimentalApi::experimental_reason(&params),
        Some("turn/start.environments")
    );

    let serialized = serde_json::to_value(&params).expect("params should serialize");
    assert_eq!(
        serialized.get("environments"),
        Some(&json!([
            {
                "environmentId": "local",
                "cwd": cwd
            }
        ]))
    );
}

#[test]
fn turn_start_params_preserve_empty_environments() {
    let params: TurnStartParams = serde_json::from_value(json!({
        "threadId": "thread_123",
        "input": [],
        "environments": [],
    }))
    .expect("params should deserialize");

    assert_eq!(params.environments, Some(Vec::new()));
    assert_eq!(
        crate::experimental_api::ExperimentalApi::experimental_reason(&params),
        Some("turn/start.environments")
    );

    let serialized = serde_json::to_value(&params).expect("params should serialize");
    assert_eq!(serialized.get("environments"), Some(&json!([])));
}

#[test]
fn turn_start_params_treat_null_or_omitted_environments_as_default() {
    let null_environments: TurnStartParams = serde_json::from_value(json!({
        "threadId": "thread_123",
        "input": [],
        "environments": null,
    }))
    .expect("params should deserialize");
    let omitted_environments: TurnStartParams = serde_json::from_value(json!({
        "threadId": "thread_123",
        "input": [],
    }))
    .expect("params should deserialize");

    assert_eq!(null_environments.environments, None);
    assert_eq!(omitted_environments.environments, None);
    assert_eq!(
        crate::experimental_api::ExperimentalApi::experimental_reason(&null_environments),
        None
    );
    assert_eq!(
        crate::experimental_api::ExperimentalApi::experimental_reason(&omitted_environments),
        None
    );
}

#[test]
fn turn_start_params_reject_relative_environment_cwd() {
    let err = serde_json::from_value::<TurnStartParams>(json!({
        "threadId": "thread_123",
        "input": [],
        "environments": [
            {
                "environmentId": "local",
                "cwd": "relative"
            }
        ],
    }))
    .expect_err("relative environment cwd should fail");

    assert!(
        err.to_string()
            .contains("AbsolutePathBuf deserialized without a base path"),
        "unexpected error: {err}"
    );
}
