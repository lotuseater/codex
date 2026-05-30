use super::*;
use super::super::*;

#[tokio::test]
async fn featured_plugin_ids_for_config_uses_restriction_product_query_param() {
    let tmp = tempfile::tempdir().unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true
"#,
    );

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/backend-api/plugins/featured"))
        .and(query_param("platform", "chat"))
        .and(header("authorization", "Bearer Access Token"))
        .and(header("chatgpt-account-id", "account_id"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"["chat-plugin"]"#))
        .mount(&server)
        .await;

    let mut config = load_config(tmp.path(), tmp.path()).await;
    config.chatgpt_base_url = format!("{}/backend-api/", server.uri());
    let manager = PluginsManager::new_with_restriction_product(
        tmp.path().to_path_buf(),
        Some(Product::Chatgpt),
    );

    let featured_plugin_ids = manager
        .featured_plugin_ids_for_config(
            &config,
            Some(&CodexAuth::create_dummy_chatgpt_auth_for_testing()),
        )
        .await
        .unwrap();

    assert_eq!(featured_plugin_ids, vec!["chat-plugin".to_string()]);
}

#[tokio::test]
async fn featured_plugin_ids_for_config_defaults_query_param_to_codex() {
    let tmp = tempfile::tempdir().unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true
"#,
    );

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/backend-api/plugins/featured"))
        .and(query_param("platform", "codex"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"["codex-plugin"]"#))
        .mount(&server)
        .await;

    let mut config = load_config(tmp.path(), tmp.path()).await;
    config.chatgpt_base_url = format!("{}/backend-api/", server.uri());
    let manager = PluginsManager::new_with_restriction_product(
        tmp.path().to_path_buf(),
        /*restriction_product*/ None,
    );

    let featured_plugin_ids = manager
        .featured_plugin_ids_for_config(&config, /*auth*/ None)
        .await
        .unwrap();

    assert_eq!(featured_plugin_ids, vec!["codex-plugin".to_string()]);
}
