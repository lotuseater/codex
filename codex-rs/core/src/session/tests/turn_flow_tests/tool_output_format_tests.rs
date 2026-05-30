use super::*;

#[test]
fn prefers_structured_content_when_present() {
    let ctr = McpCallToolResult {
        // Content present but should be ignored because structured_content is set.
        content: vec![text_block("ignored")],
        is_error: None,
        structured_content: Some(json!({
            "ok": true,
            "value": 42
        })),
        meta: None,
    };

    let got = ctr.into_function_call_output_payload();
    let expected = FunctionCallOutputPayload {
        body: FunctionCallOutputBody::Text(
            serde_json::to_string(&json!({
                "ok": true,
                "value": 42
            }))
            .unwrap(),
        ),
        success: Some(true),
    };

    assert_eq!(expected, got);
}

#[tokio::test]
async fn includes_timed_out_message() {
    let exec = ExecToolCallOutput {
        exit_code: 0,
        stdout: StreamOutput::new(String::new()),
        stderr: StreamOutput::new(String::new()),
        aggregated_output: StreamOutput::new("Command output".to_string()),
        duration: StdDuration::from_secs(1),
        timed_out: true,
    };
    let (_, turn_context) = make_session_and_context().await;

    let out = format_exec_output_str(&exec, turn_context.truncation_policy);

    assert_eq!(
        out,
        "command timed out after 1000 milliseconds\nCommand output"
    );
}

#[test]
fn falls_back_to_content_when_structured_is_null() {
    let ctr = McpCallToolResult {
        content: vec![text_block("hello"), text_block("world")],
        is_error: None,
        structured_content: Some(serde_json::Value::Null),
        meta: None,
    };

    let got = ctr.into_function_call_output_payload();
    let expected = FunctionCallOutputPayload {
        body: FunctionCallOutputBody::Text(
            serde_json::to_string(&vec![text_block("hello"), text_block("world")]).unwrap(),
        ),
        success: Some(true),
    };

    assert_eq!(expected, got);
}

#[test]
fn success_flag_reflects_is_error_true() {
    let ctr = McpCallToolResult {
        content: vec![text_block("unused")],
        is_error: Some(true),
        structured_content: Some(json!({ "message": "bad" })),
        meta: None,
    };

    let got = ctr.into_function_call_output_payload();
    let expected = FunctionCallOutputPayload {
        body: FunctionCallOutputBody::Text(
            serde_json::to_string(&json!({ "message": "bad" })).unwrap(),
        ),
        success: Some(false),
    };

    assert_eq!(expected, got);
}

#[test]
fn success_flag_true_with_no_error_and_content_used() {
    let ctr = McpCallToolResult {
        content: vec![text_block("alpha")],
        is_error: Some(false),
        structured_content: None,
        meta: None,
    };

    let got = ctr.into_function_call_output_payload();
    let expected = FunctionCallOutputPayload {
        body: FunctionCallOutputBody::Text(
            serde_json::to_string(&vec![text_block("alpha")]).unwrap(),
        ),
        success: Some(true),
    };

    assert_eq!(expected, got);
}

#[test]
fn get_service_tier_defaults_enterprise_accounts_to_fast() {
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ false,
            Some(AccountPlanType::Enterprise),
            /*fast_mode_enabled*/ true,
        ),
        Some(ServiceTier::Fast.request_value().to_string())
    );
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ false,
            Some(AccountPlanType::EnterpriseCbpUsageBased),
            /*fast_mode_enabled*/ true,
        ),
        Some(ServiceTier::Fast.request_value().to_string())
    );
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ false,
            Some(AccountPlanType::Business),
            /*fast_mode_enabled*/ true,
        ),
        Some(ServiceTier::Fast.request_value().to_string())
    );
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ false,
            Some(AccountPlanType::Team),
            /*fast_mode_enabled*/ true,
        ),
        Some(ServiceTier::Fast.request_value().to_string())
    );
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ false,
            Some(AccountPlanType::SelfServeBusinessUsageBased),
            /*fast_mode_enabled*/ true,
        ),
        Some(ServiceTier::Fast.request_value().to_string())
    );
}

#[test]
fn get_service_tier_respects_fast_default_opt_out() {
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ true,
            Some(AccountPlanType::Enterprise),
            /*fast_mode_enabled*/ true,
        ),
        None
    );
}

#[test]
fn get_service_tier_does_not_default_non_enterprise_or_disabled_fast_mode() {
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ false,
            Some(AccountPlanType::Pro),
            /*fast_mode_enabled*/ true,
        ),
        None
    );
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_default_opt_out*/ false,
            Some(AccountPlanType::Enterprise),
            /*fast_mode_enabled*/ false,
        ),
        None
    );
}
