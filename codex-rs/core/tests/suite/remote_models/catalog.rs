#![cfg(not(target_os = "windows"))]
#![allow(clippy::expect_used)]
#[path = "support.rs"]
mod support;

use support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_models_do_not_append_removed_builtin_presets() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::start().await;
    let remote_model =
        test_remote_model("remote-alpha", ModelVisibility::List, /*priority*/ 0);
    let models_mock = mount_models_once(
        &server,
        ModelsResponse {
            models: vec![remote_model.clone()],
        },
    )
    .await;

    let codex_home = TempDir::new()?;

    let auth = CodexAuth::create_dummy_chatgpt_auth_for_testing();
    let provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers(/* openai_base_url */ /*openai_base_url*/ None)["openai"].clone()
    };
    let manager = codex_core::test_support::models_manager_with_provider(
        codex_home.path().to_path_buf(),
        codex_core::test_support::auth_manager_from_auth(auth),
        provider,
    );

    let available = manager.list_models(RefreshStrategy::OnlineIfUncached).await;
    let remote = available
        .iter()
        .find(|model| model.model == "remote-alpha")
        .expect("remote model should be listed");
    let mut expected_remote: ModelPreset = remote_model.into();
    expected_remote.is_default = remote.is_default;
    assert_eq!(*remote, expected_remote);
    let default_model = available
        .iter()
        .find(|model| model.show_in_picker)
        .expect("default model should be set");
    assert!(default_model.is_default);
    assert_eq!(
        available.iter().filter(|model| model.is_default).count(),
        1,
        "expected a single default model"
    );
    assert_eq!(
        models_mock.requests().len(),
        1,
        "expected a single /models request"
    );
    // Keep the mock server alive until after async assertions complete.
    drop(server);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_models_merge_adds_new_high_priority_first() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::start().await;
    let remote_model = test_remote_model(
        "remote-top",
        ModelVisibility::List,
        /*priority*/ -10_000,
    );
    let models_mock = mount_models_once(
        &server,
        ModelsResponse {
            models: vec![remote_model],
        },
    )
    .await;

    let codex_home = TempDir::new()?;

    let auth = CodexAuth::create_dummy_chatgpt_auth_for_testing();
    let provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers(/* openai_base_url */ /*openai_base_url*/ None)["openai"].clone()
    };
    let manager = codex_core::test_support::models_manager_with_provider(
        codex_home.path().to_path_buf(),
        codex_core::test_support::auth_manager_from_auth(auth),
        provider,
    );

    let available = manager.list_models(RefreshStrategy::OnlineIfUncached).await;
    assert_eq!(
        available.first().map(|model| model.model.as_str()),
        Some("remote-top")
    );
    assert_eq!(
        models_mock.requests().len(),
        1,
        "expected a single /models request"
    );
    // Keep the mock server alive until after async assertions complete.
    drop(server);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_models_merge_replaces_overlapping_model() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::start().await;
    let slug = bundled_model_slug();
    let mut remote_model = test_remote_model(&slug, ModelVisibility::List, /*priority*/ 0);
    remote_model.display_name = "Overridden".to_string();
    remote_model.description = Some("Overridden description".to_string());
    let models_mock = mount_models_once(
        &server,
        ModelsResponse {
            models: vec![remote_model.clone()],
        },
    )
    .await;

    let codex_home = TempDir::new()?;

    let auth = CodexAuth::create_dummy_chatgpt_auth_for_testing();
    let provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers(/* openai_base_url */ /*openai_base_url*/ None)["openai"].clone()
    };
    let manager = codex_core::test_support::models_manager_with_provider(
        codex_home.path().to_path_buf(),
        codex_core::test_support::auth_manager_from_auth(auth),
        provider,
    );

    let available = manager.list_models(RefreshStrategy::OnlineIfUncached).await;
    let overridden = available
        .iter()
        .find(|model| model.model == slug)
        .expect("overlapping model should be listed");
    assert_eq!(overridden.display_name, remote_model.display_name);
    assert_eq!(
        overridden.description,
        remote_model
            .description
            .expect("remote model should include description")
    );
    assert_eq!(
        models_mock.requests().len(),
        1,
        "expected a single /models request"
    );
    // Keep the mock server alive until after async assertions complete.
    drop(server);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_models_merge_preserves_bundled_models_on_empty_response() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::start().await;
    let _models_mock = mount_models_once(&server, ModelsResponse { models: Vec::new() }).await;

    let codex_home = TempDir::new()?;

    let auth = CodexAuth::create_dummy_chatgpt_auth_for_testing();
    let provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers(/* openai_base_url */ /*openai_base_url*/ None)["openai"].clone()
    };
    let manager = codex_core::test_support::models_manager_with_provider(
        codex_home.path().to_path_buf(),
        codex_core::test_support::auth_manager_from_auth(auth),
        provider,
    );

    let available = manager.list_models(RefreshStrategy::OnlineIfUncached).await;
    let bundled_slug = bundled_model_slug();
    assert!(
        available.iter().any(|model| model.model == bundled_slug),
        "bundled models should remain available after empty remote response"
    );
    // Keep the mock server alive until after async assertions complete.
    drop(server);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_models_request_times_out_after_5s() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::start().await;
    let remote_model =
        test_remote_model("remote-timeout", ModelVisibility::List, /*priority*/ 0);
    let models_mock = mount_models_once_with_delay(
        &server,
        ModelsResponse {
            models: vec![remote_model],
        },
        Duration::from_secs(6),
    )
    .await;

    let codex_home = TempDir::new()?;

    let auth = CodexAuth::create_dummy_chatgpt_auth_for_testing();
    let provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers(/* openai_base_url */ /*openai_base_url*/ None)["openai"].clone()
    };
    let manager = codex_core::test_support::models_manager_with_provider(
        codex_home.path().to_path_buf(),
        codex_core::test_support::auth_manager_from_auth(auth),
        provider,
    );

    let start = Instant::now();
    let model = timeout(
        Duration::from_secs(7),
        manager.get_default_model(&None, RefreshStrategy::OnlineIfUncached),
    )
    .await;
    let elapsed = start.elapsed();
    // get_model should return a default model even when refresh times out
    let default_model = model.expect("get_model should finish and return default model");
    let expected_default = bundled_default_model_slug();
    assert!(
        default_model == expected_default,
        "get_model should return default model when refresh times out, got: {default_model}"
    );
    let _ = server
        .received_requests()
        .await
        .expect("mock server should capture requests")
        .iter()
        .map(|req| format!("{} {}", req.method, req.url.path()))
        .collect::<Vec<String>>();
    assert!(
        elapsed >= Duration::from_millis(4_500),
        "expected models call to block near the timeout; took {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_millis(5_800),
        "expected models call to time out before the delayed response; took {elapsed:?}"
    );
    assert_eq!(
        models_mock.requests().len(),
        1,
        "expected a single /models request"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_models_hide_picker_only_models() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = MockServer::start().await;
    let remote_model = test_remote_model(
        "codex-auto-balanced",
        ModelVisibility::Hide,
        /*priority*/ 0,
    );
    let models_mock = mount_models_once(
        &server,
        ModelsResponse {
            models: vec![remote_model],
        },
    )
    .await;

    let codex_home = TempDir::new()?;

    let auth = CodexAuth::create_dummy_chatgpt_auth_for_testing();
    let provider = ModelProviderInfo {
        base_url: Some(format!("{}/v1", server.uri())),
        ..built_in_model_providers(/* openai_base_url */ /*openai_base_url*/ None)["openai"].clone()
    };
    let manager = codex_core::test_support::models_manager_with_provider(
        codex_home.path().to_path_buf(),
        codex_core::test_support::auth_manager_from_auth(auth),
        provider,
    );

    let selected = manager
        .get_default_model(&None, RefreshStrategy::OnlineIfUncached)
        .await;
    assert_eq!(selected, bundled_default_model_slug());

    let available = manager.list_models(RefreshStrategy::OnlineIfUncached).await;
    let hidden = available
        .iter()
        .find(|model| model.model == "codex-auto-balanced")
        .expect("hidden remote model should be listed");
    assert!(!hidden.show_in_picker, "hidden models should remain hidden");
    assert_eq!(
        models_mock.requests().len(),
        1,
        "expected a single /models request"
    );
    // Keep the mock server alive until after async assertions complete.
    drop(server);

    Ok(())
}
