use codex_login::CodexAuth;
use codex_model_provider_info::built_in_model_providers;
use codex_protocol::error::CodexErr;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use codex_core_test_runtime::responses::ev_completed;
use codex_core_test_runtime::responses::ev_completed_with_tokens;
use codex_core_test_runtime::responses::ev_response_created;
use codex_core_test_runtime::responses::mount_sse_once;
use codex_core_test_runtime::responses::mount_sse_once_match;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::responses::sse_failed;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::test_codex::TestCodex;
use codex_core_test_runtime::test_codex::test_codex;
use codex_core_test_runtime::wait_for_event;
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::body_string_contains;
use wiremock::matchers::method;
use wiremock::matchers::path;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn token_count_includes_rate_limits_snapshot() {
    skip_if_no_network!();
    let server = MockServer::start().await;

    let sse_body = sse(vec![ev_completed_with_tokens(
        "resp_rate",
        /*total_tokens*/ 123,
    )]);

    let response = ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .insert_header("x-codex-primary-used-percent", "12.5")
        .insert_header("x-codex-secondary-used-percent", "40.0")
        .insert_header("x-codex-primary-window-minutes", "10")
        .insert_header("x-codex-secondary-window-minutes", "60")
        .insert_header("x-codex-primary-reset-at", "1704069000")
        .insert_header("x-codex-secondary-reset-at", "1704074400")
        .set_body_raw(sse_body, "text/event-stream");

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(response)
        .expect(1)
        .mount(&server)
        .await;

    let mut provider =
        built_in_model_providers(/* openai_base_url */ /*openai_base_url*/ None)["openai"].clone();
    provider.base_url = Some(format!("{}/v1", server.uri()));
    provider.supports_websockets = false;

    let mut builder = test_codex()
        .with_auth(CodexAuth::from_api_key("test"))
        .with_config(move |config| {
            config.model_provider = provider;
        });
    let codex = builder
        .build(&server)
        .await
        .expect("create conversation")
        .codex;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .unwrap();

    let token_event = wait_for_event(
        &codex,
        |msg| matches!(msg, EventMsg::TokenCount(ev) if ev.info.is_some()),
    )
    .await;
    let final_payload = match token_event {
        EventMsg::TokenCount(ev) => ev,
        _ => unreachable!(),
    };
    // Assert full JSON for the final token count event (usage + rate limits)
    let final_json = serde_json::to_value(&final_payload).unwrap();
    pretty_assertions::assert_eq!(
        final_json,
        json!({
            "info": {
                "total_token_usage": {
                    "input_tokens": 123,
                    "cached_input_tokens": 0,
                    "output_tokens": 0,
                    "reasoning_output_tokens": 0,
                    "total_tokens": 123
                },
                "last_token_usage": {
                    "input_tokens": 123,
                    "cached_input_tokens": 0,
                    "output_tokens": 0,
                    "reasoning_output_tokens": 0,
                    "total_tokens": 123
                },
                // Default model is gpt-5.4 in tests → 95% usable context window
                "model_context_window": 258400
            },
            "rate_limits": {
                "limit_id": "codex",
                "limit_name": null,
                "primary": {
                    "used_percent": 12.5,
                    "window_minutes": 10,
                    "resets_at": 1704069000
                },
                "secondary": {
                    "used_percent": 40.0,
                    "window_minutes": 60,
                    "resets_at": 1704074400
                },
                "credits": null,
                "plan_type": null,
                "rate_limit_reached_type": null
            }
        })
    );
    let usage = final_payload
        .info
        .expect("token usage info should be recorded after completion");
    assert_eq!(usage.total_token_usage.total_tokens, 123);
    let final_snapshot = final_payload
        .rate_limits
        .expect("latest rate limit snapshot should be retained");
    assert_eq!(
        final_snapshot
            .primary
            .as_ref()
            .map(|window| window.used_percent),
        Some(12.5)
    );
    assert_eq!(
        final_snapshot
            .primary
            .as_ref()
            .and_then(|window| window.resets_at),
        Some(1704069000)
    );

    wait_for_event(&codex, |msg| matches!(msg, EventMsg::TurnComplete(_))).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn usage_limit_error_emits_rate_limit_event() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let server = MockServer::start().await;

    let response = ResponseTemplate::new(429)
        .insert_header("x-codex-primary-used-percent", "100.0")
        .insert_header("x-codex-secondary-used-percent", "87.5")
        .insert_header("x-codex-primary-over-secondary-limit-percent", "95.0")
        .insert_header("x-codex-primary-window-minutes", "15")
        .insert_header("x-codex-secondary-window-minutes", "60")
        .set_body_json(json!({
            "error": {
                "type": "usage_limit_reached",
                "message": "limit reached",
                "resets_at": 1704067242,
                "plan_type": "pro"
            }
        }));

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(response)
        .expect(1)
        .mount(&server)
        .await;

    let mut builder = test_codex();
    let codex_fixture = builder.build(&server).await?;
    let codex = codex_fixture.codex.clone();

    let expected_limits = json!({
        "limit_id": "codex",
        "limit_name": null,
        "primary": {
            "used_percent": 100.0,
            "window_minutes": 15,
            "resets_at": null
        },
        "secondary": {
            "used_percent": 87.5,
            "window_minutes": 60,
            "resets_at": null
        },
        "credits": null,
        "plan_type": null,
        "rate_limit_reached_type": null
    });

    let submission_id = codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "hello".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await
        .expect("submission should succeed while emitting usage limit error events");

    let token_event = wait_for_event(&codex, |msg| matches!(msg, EventMsg::TokenCount(_))).await;
    let EventMsg::TokenCount(event) = token_event else {
        unreachable!();
    };

    let event_json = serde_json::to_value(&event).expect("serialize token count event");
    pretty_assertions::assert_eq!(
        event_json,
        json!({
            "info": null,
            "rate_limits": expected_limits
        })
    );

    let error_event = wait_for_event(&codex, |msg| matches!(msg, EventMsg::Error(_))).await;
    let EventMsg::Error(error_event) = error_event else {
        unreachable!();
    };
    assert!(
        error_event.message.to_lowercase().contains("usage limit"),
        "unexpected error message for submission {submission_id}: {}",
        error_event.message
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn context_window_error_sets_total_tokens_to_model_window() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let server = MockServer::start().await;

    const EFFECTIVE_CONTEXT_WINDOW: i64 = (272_000 * 95) / 100;

    mount_sse_once_match(
        &server,
        body_string_contains("trigger context window"),
        sse_failed(
            "resp_context_window",
            "context_length_exceeded",
            "Your input exceeds the context window of this model. Please adjust your input and try again.",
        ),
    )
    .await;

    mount_sse_once_match(
        &server,
        body_string_contains("seed turn"),
        sse(vec![
            ev_response_created("resp_seed"),
            ev_completed("resp_seed"),
        ]),
    )
    .await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(|config| {
            config.model = Some("gpt-5.4".to_string());
            config.model_context_window = Some(272_000);
        })
        .build(&server)
        .await?;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "seed turn".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "trigger context window".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let token_event = wait_for_event(&codex, |event| {
        matches!(
            event,
            EventMsg::TokenCount(payload)
                if payload.info.as_ref().is_some_and(|info| {
                    info.model_context_window == Some(info.total_token_usage.total_tokens)
                        && info.total_token_usage.total_tokens > 0
                })
        )
    })
    .await;

    let EventMsg::TokenCount(token_payload) = token_event else {
        unreachable!("wait_for_event returned unexpected event");
    };

    let info = token_payload
        .info
        .expect("token usage info present when context window is exceeded");

    assert_eq!(info.model_context_window, Some(EFFECTIVE_CONTEXT_WINDOW));
    assert_eq!(
        info.total_token_usage.total_tokens,
        EFFECTIVE_CONTEXT_WINDOW
    );

    let error_event = wait_for_event(&codex, |ev| matches!(ev, EventMsg::Error(_))).await;
    let expected_context_window_message = CodexErr::ContextWindowExceeded.to_string();
    assert!(
        matches!(
            error_event,
            EventMsg::Error(ref err) if err.message == expected_context_window_message
        ),
        "expected context window error; got {error_event:?}"
    );

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn incomplete_response_emits_content_filter_error_message() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let server = MockServer::start().await;

    let incomplete_response = sse(vec![
        ev_response_created("resp_incomplete"),
        ev_message_item_added("msg_incomplete", "partial content"),
        ev_output_text_delta("continued chunk"),
        json!({
            "type": "response.incomplete",
            "response": {
                "id": "resp_incomplete",
                "object": "response",
                "status": "incomplete",
                "error": null,
                "incomplete_details": {
                    "reason": "content_filter"
                }
            }
        }),
    ]);

    let responses_mock = mount_sse_once(&server, incomplete_response).await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(|config| {
            config.model_provider.stream_max_retries = Some(0);
        })
        .build(&server)
        .await?;
    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "trigger incomplete".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let error_event = wait_for_event(&codex, |ev| matches!(ev, EventMsg::Error(_))).await;
    assert!(
        matches!(
            error_event,
            EventMsg::Error(ref err)
                if err.message
                    == "stream disconnected before completion: Incomplete response returned, reason: content_filter"
        ),
        "expected incomplete content filter error; got {error_event:?}"
    );

    assert_eq!(responses_mock.requests().len(), 1);

    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;
    Ok(())
}

/// We try to avoid setting env vars in tests because std::env::set_var() is
/// process-wide and unsafe. Though for this test, we want to simulate the
/// presence of an environment variable that the provider will read for auth, so
/// we pick a commonly existing env var that is guaranteed to have a non-empty
/// value on both Windows and Unix. Note that this test must also work when run
/// under Bazel in CI, which uses a restricted environment, so PATH seems like
/// the safest choice.
