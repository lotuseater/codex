#![allow(dead_code, unused_imports, clippy::expect_used, clippy::unwrap_used)]

include!("code_mode_shared.rs");
use pretty_assertions::assert_eq;
use pretty_assertions::assert_ne;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_output_images_via_global_helper() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let (_test, second_mock) = run_code_mode_turn(
        &server,
        "use exec to return images",
        r#"
image("https://example.com/image.jpg");
image("data:image/png;base64,AAA");
"#,
        /*include_apply_patch*/ false,
    )
    .await?;

    let req = second_mock.single_request();
    let items = custom_tool_output_items(&req, "call-1");
    let (_, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "code_mode image output failed unexpectedly"
    );
    assert_eq!(items.len(), 3);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script completed\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&items, /*index*/ 0),
    );
    assert_eq!(
        items[1],
        serde_json::json!({
            "type": "input_image",
            "image_url": "https://example.com/image.jpg",
            "detail": "high"
        }),
    );
    assert_eq!(
        items[2],
        serde_json::json!({
            "type": "input_image",
            "image_url": "data:image/png;base64,AAA",
            "detail": "high"
        }),
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_use_view_image_result_with_image_helper() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let mut builder = test_codex()
        .with_model("gpt-5.3-codex")
        .with_config(move |config| {
            let _ = config.features.enable(Feature::CodeMode);
        });
    let test = builder.build(&server).await?;

    let image_bytes = BASE64_STANDARD.decode(
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
    )?;
    let image_path = test.cwd_path().join("code_mode_view_image.png");
    fs::write(&image_path, image_bytes)?;

    let image_path_json = serde_json::to_string(&image_path.to_string_lossy().to_string())?;
    let code = format!(
        r#"
const out = await tools.view_image({{ path: {image_path_json}, detail: "original" }});
image(out);
"#
    );

    responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_custom_tool_call("call-1", "exec", &code),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let second_mock = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-2"),
        ]),
    )
    .await;

    test.submit_turn("use exec to call view_image and emit its image output")
        .await?;

    let req = second_mock.single_request();
    let items = custom_tool_output_items(&req, "call-1");
    let (_, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "code_mode view_image call failed unexpectedly"
    );
    assert_eq!(items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script completed\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&items, /*index*/ 0),
    );

    assert_eq!(
        items[1].get("type").and_then(Value::as_str),
        Some("input_image")
    );

    let emitted_image_url = items[1]
        .get("image_url")
        .and_then(Value::as_str)
        .expect("image helper should emit an input_image item with image_url");
    assert!(emitted_image_url.starts_with("data:image/png;base64,"));
    assert_eq!(
        items[1].get("detail").and_then(Value::as_str),
        Some("original")
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn code_mode_can_use_mcp_image_result_with_image_helper() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let code = r#"
const out = await tools.mcp__rmcp__image_scenario({
  scenario: "image_only_original_detail",
});
const imageItem = out.content.find((item) => item.type === "image");
image(imageItem);
"#;

    let (_test, second_mock) = run_code_mode_turn_with_rmcp_model(
        &server,
        "use exec to call the rmcp image scenario tool and emit its image output",
        code,
        "gpt-5.3-codex",
    )
    .await?;

    let req = second_mock.single_request();
    let items = custom_tool_output_items(&req, "call-1");
    let (_, success) = custom_tool_output_body_and_success(&req, "call-1");
    assert_ne!(
        success,
        Some(false),
        "code_mode mcp image scenario call failed unexpectedly"
    );
    assert_eq!(items.len(), 2);
    assert_regex_match(
        concat!(
            r"(?s)\A",
            r"Script completed\nWall time \d+\.\d seconds\nOutput:\n\z"
        ),
        text_item(&items, /*index*/ 0),
    );

    assert_eq!(
        items[1].get("type").and_then(Value::as_str),
        Some("input_image")
    );

    let emitted_image_url = items[1]
        .get("image_url")
        .and_then(Value::as_str)
        .expect("image helper should emit an input_image item with image_url");
    assert!(emitted_image_url.starts_with("data:image/png;base64,"));
    assert_eq!(
        items[1].get("detail").and_then(Value::as_str),
        Some("original")
    );

    Ok(())
}
