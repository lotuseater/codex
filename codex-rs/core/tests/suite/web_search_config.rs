#![cfg(not(target_os = "windows"))]
#![allow(clippy::unwrap_used)]

#[path = "web_search_common.rs"]
mod common;

use common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn web_search_tool_config_from_config_toml_is_forwarded_to_request() {
    skip_if_no_network!();

    let server = start_mock_server().await;
    let sse = responses::sse(vec![
        responses::ev_response_created("resp-1"),
        responses::ev_completed("resp-1"),
    ]);
    let resp_mock = responses::mount_sse_once(&server, sse).await;

    let home = Arc::new(tempfile::TempDir::new().expect("create codex home"));
    std::fs::write(
        home.path().join("config.toml"),
        r#"web_search = "live"

[tools.web_search]
context_size = "high"
allowed_domains = ["example.com"]
location = { country = "US", city = "New York", timezone = "America/New_York" }
"#,
    )
    .expect("write config.toml");

    let mut builder = test_codex().with_model("gpt-5.3-codex").with_home(home);
    let test = builder
        .build(&server)
        .await
        .expect("create test Codex conversation");

    test.submit_turn_with_permission_profile(
        "hello configured web search",
        PermissionProfile::Disabled,
    )
    .await
    .expect("submit turn");

    let body = resp_mock.single_request().body_json();
    let tool = find_web_search_tool(&body);
    assert_eq!(
        tool,
        &json!({
            "type": "web_search",
            "external_web_access": true,
            "search_context_size": "high",
            "filters": {
                "allowed_domains": ["example.com"],
            },
            "user_location": {
                "type": "approximate",
                "country": "US",
                "city": "New York",
                "timezone": "America/New_York",
            },
        })
    );
}
