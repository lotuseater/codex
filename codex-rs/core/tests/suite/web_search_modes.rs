#![cfg(not(target_os = "windows"))]
#![allow(clippy::unwrap_used)]

#[path = "web_search_common.rs"]
mod common;

use common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn web_search_mode_cached_sets_external_web_access_false() {
    skip_if_no_network!();

    let server = start_mock_server().await;
    let sse = responses::sse(vec![
        responses::ev_response_created("resp-1"),
        responses::ev_completed("resp-1"),
    ]);
    let resp_mock = responses::mount_sse_once(&server, sse).await;

    let mut builder = test_codex().with_model("gpt-5.4").with_config(|config| {
        config
            .web_search_mode
            .set(WebSearchMode::Cached)
            .expect("test web_search_mode should satisfy constraints");
    });
    let test = builder
        .build(&server)
        .await
        .expect("create test Codex conversation");

    test.submit_turn_with_permission_profile(
        "hello cached web search",
        PermissionProfile::read_only(),
    )
    .await
    .expect("submit turn");

    let body = resp_mock.single_request().body_json();
    let tool = find_web_search_tool(&body);
    assert_eq!(
        tool.get("external_web_access").and_then(Value::as_bool),
        Some(false),
        "web_search cached mode should force external_web_access=false"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn web_search_mode_takes_precedence_over_legacy_flags() {
    skip_if_no_network!();

    let server = start_mock_server().await;
    let sse = responses::sse(vec![
        responses::ev_response_created("resp-1"),
        responses::ev_completed("resp-1"),
    ]);
    let resp_mock = responses::mount_sse_once(&server, sse).await;

    let mut builder = test_codex().with_model("gpt-5.4").with_config(|config| {
        config
            .features
            .enable(Feature::WebSearchRequest)
            .expect("test config should allow feature update");
        config
            .web_search_mode
            .set(WebSearchMode::Cached)
            .expect("test web_search_mode should satisfy constraints");
    });
    let test = builder
        .build(&server)
        .await
        .expect("create test Codex conversation");

    test.submit_turn_with_permission_profile(
        "hello cached+live flags",
        PermissionProfile::read_only(),
    )
    .await
    .expect("submit turn");

    let body = resp_mock.single_request().body_json();
    let tool = find_web_search_tool(&body);
    assert_eq!(
        tool.get("external_web_access").and_then(Value::as_bool),
        Some(false),
        "web_search mode should win over legacy web_search_request"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn web_search_mode_defaults_to_cached_when_features_disabled() {
    skip_if_no_network!();

    let server = start_mock_server().await;
    let sse = responses::sse(vec![
        responses::ev_response_created("resp-1"),
        responses::ev_completed("resp-1"),
    ]);
    let resp_mock = responses::mount_sse_once(&server, sse).await;

    let mut builder = test_codex().with_model("gpt-5.4").with_config(|config| {
        config
            .web_search_mode
            .set(WebSearchMode::Cached)
            .expect("test web_search_mode should satisfy constraints");
        config
            .features
            .disable(Feature::WebSearchCached)
            .expect("test config should allow feature update");
        config
            .features
            .disable(Feature::WebSearchRequest)
            .expect("test config should allow feature update");
    });
    let test = builder
        .build(&server)
        .await
        .expect("create test Codex conversation");

    test.submit_turn_with_permission_profile(
        "hello default cached web search",
        PermissionProfile::read_only(),
    )
    .await
    .expect("submit turn");

    let body = resp_mock.single_request().body_json();
    let tool = find_web_search_tool(&body);
    assert_eq!(
        tool.get("external_web_access").and_then(Value::as_bool),
        Some(false),
        "default web_search should be cached when unset"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn web_search_mode_updates_between_turns_with_permission_profile() {
    skip_if_no_network!();

    let server = start_mock_server().await;
    let resp_mock = responses::mount_sse_sequence(
        &server,
        vec![
            responses::sse(vec![
                responses::ev_response_created("resp-1"),
                responses::ev_completed("resp-1"),
            ]),
            responses::sse(vec![
                responses::ev_response_created("resp-2"),
                responses::ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut builder = test_codex().with_model("gpt-5.4").with_config(|config| {
        config
            .web_search_mode
            .set(WebSearchMode::Cached)
            .expect("test web_search_mode should satisfy constraints");
        config
            .features
            .disable(Feature::WebSearchCached)
            .expect("test config should allow feature update");
        config
            .features
            .disable(Feature::WebSearchRequest)
            .expect("test config should allow feature update");
    });
    let test = builder
        .build(&server)
        .await
        .expect("create test Codex conversation");

    test.submit_turn_with_permission_profile("hello cached", PermissionProfile::read_only())
        .await
        .expect("submit first turn");
    test.submit_turn_with_permission_profile("hello live", PermissionProfile::Disabled)
        .await
        .expect("submit second turn");

    let requests = resp_mock.requests();
    assert_eq!(requests.len(), 2, "expected two response requests");

    let first_body = requests[0].body_json();
    let first_tool = find_web_search_tool(&first_body);
    assert_eq!(
        first_tool
            .get("external_web_access")
            .and_then(Value::as_bool),
        Some(false),
        "read-only policy should default web_search to cached"
    );

    let second_body = requests[1].body_json();
    let second_tool = find_web_search_tool(&second_body);
    assert_eq!(
        second_tool
            .get("external_web_access")
            .and_then(Value::as_bool),
        Some(true),
        "danger-full-access policy should default web_search to live"
    );
}
