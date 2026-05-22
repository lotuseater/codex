#![cfg(not(target_os = "windows"))]

#[path = "view_image_common.rs"]
mod common;

use common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn view_image_tool_attaches_local_image() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex();
    let test = builder.build_remote_aware(&server).await?;
    let TestCodex {
        codex,
        session_configured,
        config,
        ..
    } = &test;
    let cwd = config.cwd.clone();

    let rel_path = "assets/example.png";
    let abs_path = cwd.join(rel_path);
    let original_width = 2304;
    let original_height = 864;
    write_workspace_png(
        &test,
        rel_path,
        original_width,
        original_height,
        [255u8, 0, 0, 255],
    )
    .await?;

    let call_id = "view-image-call";
    let arguments = serde_json::json!({ "path": rel_path }).to_string();

    let first_response = sse(vec![
        ev_response_created("resp-1"),
        ev_function_call(call_id, "view_image", &arguments),
        ev_completed("resp-1"),
    ]);
    responses::mount_sse_once(&server, first_response).await;

    let second_response = sse(vec![
        ev_assistant_message("msg-1", "done"),
        ev_completed("resp-2"),
    ]);
    let mock = responses::mount_sse_once(&server, second_response).await;

    let session_model = session_configured.model.clone();

    codex
        .submit(disabled_user_turn(
            &test,
            vec![UserInput::Text {
                text: "please add the screenshot".into(),
                text_elements: Vec::new(),
            }],
            session_model,
        ))
        .await?;

    let mut item_started = None;
    let mut item_completed = None;
    let mut legacy_event = None;
    wait_for_event_with_timeout(
        codex,
        |event| match event {
            EventMsg::ItemStarted(event) => {
                if matches!(&event.item, codex_protocol::items::TurnItem::ImageView(_)) {
                    item_started = Some(event.item.clone());
                }
                false
            }
            EventMsg::ItemCompleted(event) => {
                if matches!(&event.item, codex_protocol::items::TurnItem::ImageView(_)) {
                    item_completed = Some(event.item.clone());
                }
                false
            }
            EventMsg::ViewImageToolCall(event) => {
                legacy_event = Some(event.clone());
                false
            }
            EventMsg::TurnComplete(_) => true,
            _ => false,
        },
        // Empirically, we have seen this run slow when run under
        // Bazel on arm Linux.
        VIEW_IMAGE_TURN_COMPLETE_TIMEOUT,
    )
    .await;

    match item_started.expect("view image item started event emitted") {
        codex_protocol::items::TurnItem::ImageView(item) => {
            assert_eq!(item.id, call_id);
            assert_eq!(item.path, abs_path);
        }
        other => panic!("expected ImageView item, got {other:?}"),
    }
    match item_completed.expect("view image item completed event emitted") {
        codex_protocol::items::TurnItem::ImageView(item) => {
            assert_eq!(item.id, call_id);
            assert_eq!(item.path, abs_path);
        }
        other => panic!("expected ImageView item, got {other:?}"),
    }
    let legacy_event = legacy_event.expect("legacy view image event emitted");
    assert_eq!(legacy_event.call_id, call_id);
    assert_eq!(legacy_event.path, abs_path);

    let req = mock.single_request();
    let body = req.body_json();
    assert!(
        find_image_message(&body).is_none(),
        "view_image tool should not inject a separate image message"
    );

    let function_output = req.function_call_output(call_id);
    let output_items = function_output
        .get("output")
        .and_then(Value::as_array)
        .expect("function_call_output should be a content item array");
    assert_eq!(
        output_items.len(),
        1,
        "view_image should return only the image content item (no tag/label text)"
    );
    assert_eq!(
        output_items[0].get("type").and_then(Value::as_str),
        Some("input_image"),
        "view_image should return only an input_image content item"
    );
    let image_url = output_items[0]
        .get("image_url")
        .and_then(Value::as_str)
        .expect("image_url present");

    let (prefix, encoded) = image_url
        .split_once(',')
        .expect("image url contains data prefix");
    assert_eq!(prefix, "data:image/png;base64");

    let decoded = BASE64_STANDARD
        .decode(encoded)
        .expect("image data decodes from base64 for request");
    let resized = load_from_memory(&decoded).expect("load resized image");
    let (resized_width, resized_height) = resized.dimensions();
    assert_eq!((resized_width, resized_height), (2048, 768));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn view_image_routes_to_selected_local_environment() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex();
    let test = builder.build(&server).await?;
    write_workspace_file(
        &test,
        "local.png",
        png_bytes(/*width*/ 1, /*height*/ 1, [0, 255, 0, 255])?,
    )
    .await?;
    let call_id = "call-view-image-local-env";
    let response_mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(
                    call_id,
                    "view_image",
                    &json!({
                        "path": "local.png",
                        "environment_id": LOCAL_ENVIRONMENT_ID,
                    })
                    .to_string(),
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    test.submit_turn_with_environments(
        "route local view image",
        Some(vec![TurnEnvironmentSelection {
            environment_id: LOCAL_ENVIRONMENT_ID.to_string(),
            cwd: test.config.cwd.clone(),
        }]),
    )
    .await?;

    let output = response_mock
        .last_request()
        .context("missing request containing local view_image output")?
        .function_call_output(call_id);
    let output_items = output
        .get("output")
        .and_then(Value::as_array)
        .context("view_image output should be content items")?;
    assert_eq!(output_items.len(), 1);
    let image_url = output_items[0]
        .get("image_url")
        .and_then(Value::as_str)
        .context("view_image output should include image_url")?;
    assert!(
        image_url.starts_with("data:image/png;base64,"),
        "unexpected image_url: {image_url}",
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn view_image_tool_applies_local_sandbox_read_denies() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex();
    let test = builder.build(&server).await?;
    let rel_path = "denied.png";
    let denied_path = test.config.cwd.join(rel_path);
    write_workspace_file(
        &test,
        rel_path,
        png_bytes(/*width*/ 1, /*height*/ 1, [0, 255, 0, 255])?,
    )
    .await?;
    let call_id = "call-view-image-outside-cwd";
    let response_mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(
                    call_id,
                    "view_image",
                    &json!({ "path": rel_path }).to_string(),
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let mut file_system_sandbox_policy = FileSystemSandboxPolicy::default();
    file_system_sandbox_policy
        .entries
        .push(FileSystemSandboxEntry {
            path: FileSystemPath::Path {
                path: denied_path.clone(),
            },
            access: FileSystemAccessMode::None,
        });
    let permission_profile = PermissionProfile::from_runtime_permissions(
        &file_system_sandbox_policy,
        NetworkSandboxPolicy::Restricted,
    );

    test.submit_turn_with_permission_profile("attach the denied image", permission_profile)
        .await?;

    let request = response_mock
        .last_request()
        .context("missing request containing sandboxed view_image output")?;
    assert!(
        request.inputs_of_type("input_image").is_empty(),
        "sandboxed local view_image should not attach denied images"
    );
    let output_text = request
        .function_call_output_content_and_success(call_id)
        .and_then(|(content, _)| content)
        .context("sandboxed view_image error text present")?;
    let expected_locate_prefix = format!("unable to locate image at `{}`:", denied_path.display());
    let expected_read_prefix = format!("unable to read image at `{}`:", denied_path.display());
    assert!(
        output_text.starts_with(&expected_locate_prefix)
            || output_text.starts_with(&expected_read_prefix),
        "expected error to start with `{expected_locate_prefix}` or `{expected_read_prefix}` but got `{output_text}`"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn view_image_tool_can_preserve_original_resolution_when_requested_on_gpt5_3_codex()
-> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex().with_model("gpt-5.3-codex");
    let test = builder.build_remote_aware(&server).await?;
    let TestCodex {
        codex,
        session_configured,
        ..
    } = &test;

    let rel_path = "assets/original-example.png";
    let original_width = 2304;
    let original_height = 864;
    write_workspace_png(
        &test,
        rel_path,
        original_width,
        original_height,
        [0u8, 80, 255, 255],
    )
    .await?;

    let call_id = "view-image-original";
    let arguments = serde_json::json!({ "path": rel_path, "detail": "original" }).to_string();

    let first_response = sse(vec![
        ev_response_created("resp-1"),
        ev_function_call(call_id, "view_image", &arguments),
        ev_completed("resp-1"),
    ]);
    responses::mount_sse_once(&server, first_response).await;

    let second_response = sse(vec![
        ev_assistant_message("msg-1", "done"),
        ev_completed("resp-2"),
    ]);
    let mock = responses::mount_sse_once(&server, second_response).await;

    let session_model = session_configured.model.clone();

    codex
        .submit(disabled_user_turn(
            &test,
            vec![UserInput::Text {
                text: "please add the original screenshot".into(),
                text_elements: Vec::new(),
            }],
            session_model,
        ))
        .await?;

    wait_for_event_with_timeout(
        codex,
        |event| matches!(event, EventMsg::TurnComplete(_)),
        VIEW_IMAGE_TURN_COMPLETE_TIMEOUT,
    )
    .await;

    let req = mock.single_request();
    let function_output = req.function_call_output(call_id);
    let output_items = function_output
        .get("output")
        .and_then(Value::as_array)
        .expect("function_call_output should be a content item array");
    assert_eq!(output_items.len(), 1);
    assert_eq!(
        output_items[0].get("detail").and_then(Value::as_str),
        Some("original")
    );
    let image_url = output_items[0]
        .get("image_url")
        .and_then(Value::as_str)
        .expect("image_url present");

    let (_, encoded) = image_url
        .split_once(',')
        .expect("image url contains data prefix");
    let decoded = BASE64_STANDARD
        .decode(encoded)
        .expect("image data decodes from base64 for request");
    let preserved = load_from_memory(&decoded).expect("load preserved image");
    let (width, height) = preserved.dimensions();
    assert_eq!(width, original_width);
    assert_eq!(height, original_height);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn view_image_tool_treats_null_detail_as_omitted() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex().with_model("gpt-5.3-codex");
    let test = builder.build_remote_aware(&server).await?;
    let TestCodex {
        codex,
        session_configured,
        ..
    } = &test;

    let rel_path = "assets/null-detail.png";
    let original_width = 2304;
    let original_height = 864;
    write_workspace_png(
        &test,
        rel_path,
        original_width,
        original_height,
        [0u8, 80, 255, 255],
    )
    .await?;

    let call_id = "view-image-null-detail";
    let arguments = serde_json::json!({ "path": rel_path, "detail": null }).to_string();

    let first_response = sse(vec![
        ev_response_created("resp-1"),
        ev_function_call(call_id, "view_image", &arguments),
        ev_completed("resp-1"),
    ]);
    responses::mount_sse_once(&server, first_response).await;

    let second_response = sse(vec![
        ev_assistant_message("msg-1", "done"),
        ev_completed("resp-2"),
    ]);
    let mock = responses::mount_sse_once(&server, second_response).await;

    let session_model = session_configured.model.clone();

    codex
        .submit(disabled_user_turn(
            &test,
            vec![UserInput::Text {
                text: "please attach the image with a null detail".into(),
                text_elements: Vec::new(),
            }],
            session_model,
        ))
        .await?;

    wait_for_event_with_timeout(
        codex,
        |event| matches!(event, EventMsg::TurnComplete(_)),
        VIEW_IMAGE_TURN_COMPLETE_TIMEOUT,
    )
    .await;

    let req = mock.single_request();
    let function_output = req.function_call_output(call_id);
    let output_items = function_output
        .get("output")
        .and_then(Value::as_array)
        .expect("function_call_output should be a content item array");
    assert_eq!(output_items.len(), 1);
    assert_eq!(
        output_items[0].get("detail").and_then(Value::as_str),
        Some("high")
    );
    let image_url = output_items[0]
        .get("image_url")
        .and_then(Value::as_str)
        .expect("image_url present");

    let (_, encoded) = image_url
        .split_once(',')
        .expect("image url contains data prefix");
    let decoded = BASE64_STANDARD
        .decode(encoded)
        .expect("image data decodes from base64 for request");
    let resized = load_from_memory(&decoded).expect("load resized image");
    let (width, height) = resized.dimensions();
    assert_eq!((width, height), (2048, 768));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn view_image_tool_resizes_when_model_lacks_original_detail_support() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex().with_model("gpt-5.2");
    let test = builder.build_remote_aware(&server).await?;
    let TestCodex {
        codex,
        session_configured,
        ..
    } = &test;

    let rel_path = "assets/original-example-lower-model.png";
    let original_width = 2304;
    let original_height = 864;
    write_workspace_png(
        &test,
        rel_path,
        original_width,
        original_height,
        [0u8, 80, 255, 255],
    )
    .await?;

    let call_id = "view-image-original-lower-model";
    let arguments = serde_json::json!({ "path": rel_path }).to_string();

    let first_response = sse(vec![
        ev_response_created("resp-1"),
        ev_function_call(call_id, "view_image", &arguments),
        ev_completed("resp-1"),
    ]);
    responses::mount_sse_once(&server, first_response).await;

    let second_response = sse(vec![
        ev_assistant_message("msg-1", "done"),
        ev_completed("resp-2"),
    ]);
    let mock = responses::mount_sse_once(&server, second_response).await;

    let session_model = session_configured.model.clone();

    codex
        .submit(disabled_user_turn(
            &test,
            vec![UserInput::Text {
                text: "please add the screenshot".into(),
                text_elements: Vec::new(),
            }],
            session_model,
        ))
        .await?;

    wait_for_event_with_timeout(
        codex,
        |event| matches!(event, EventMsg::TurnComplete(_)),
        VIEW_IMAGE_TURN_COMPLETE_TIMEOUT,
    )
    .await;

    let req = mock.single_request();
    let function_output = req.function_call_output(call_id);
    let output_items = function_output
        .get("output")
        .and_then(Value::as_array)
        .expect("function_call_output should be a content item array");
    assert_eq!(output_items.len(), 1);
    assert_eq!(
        output_items[0].get("detail").and_then(Value::as_str),
        Some("high")
    );

    let image_url = output_items[0]
        .get("image_url")
        .and_then(Value::as_str)
        .expect("image_url present");

    let (prefix, encoded) = image_url
        .split_once(',')
        .expect("image url contains data prefix");
    assert_eq!(prefix, "data:image/png;base64");

    let decoded = BASE64_STANDARD
        .decode(encoded)
        .expect("image data decodes from base64 for request");
    let resized = load_from_memory(&decoded).expect("load resized image");
    let (resized_width, resized_height) = resized.dimensions();
    assert_eq!((resized_width, resized_height), (2048, 768));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn view_image_tool_does_not_force_original_resolution_with_capability_only()
-> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex().with_model("gpt-5.3-codex");
    let test = builder.build_remote_aware(&server).await?;
    let TestCodex {
        codex,
        session_configured,
        ..
    } = &test;

    let rel_path = "assets/original-example-capability-only.png";
    let original_width = 2304;
    let original_height = 864;
    write_workspace_png(
        &test,
        rel_path,
        original_width,
        original_height,
        [0u8, 80, 255, 255],
    )
    .await?;

    let call_id = "view-image-capability-only";
    let arguments = serde_json::json!({ "path": rel_path }).to_string();

    let first_response = sse(vec![
        ev_response_created("resp-1"),
        ev_function_call(call_id, "view_image", &arguments),
        ev_completed("resp-1"),
    ]);
    responses::mount_sse_once(&server, first_response).await;

    let second_response = sse(vec![
        ev_assistant_message("msg-1", "done"),
        ev_completed("resp-2"),
    ]);
    let mock = responses::mount_sse_once(&server, second_response).await;

    let session_model = session_configured.model.clone();

    codex
        .submit(disabled_user_turn(
            &test,
            vec![UserInput::Text {
                text: "please add the screenshot".into(),
                text_elements: Vec::new(),
            }],
            session_model,
        ))
        .await?;

    wait_for_event_with_timeout(
        codex,
        |event| matches!(event, EventMsg::TurnComplete(_)),
        VIEW_IMAGE_TURN_COMPLETE_TIMEOUT,
    )
    .await;

    let req = mock.single_request();
    let function_output = req.function_call_output(call_id);
    let output_items = function_output
        .get("output")
        .and_then(Value::as_array)
        .expect("function_call_output should be a content item array");
    assert_eq!(output_items.len(), 1);
    assert_eq!(
        output_items[0].get("detail").and_then(Value::as_str),
        Some("high")
    );
    let image_url = output_items[0]
        .get("image_url")
        .and_then(Value::as_str)
        .expect("image_url present");

    let (_, encoded) = image_url
        .split_once(',')
        .expect("image url contains data prefix");
    let decoded = BASE64_STANDARD
        .decode(encoded)
        .expect("image data decodes from base64 for request");
    let resized = load_from_memory(&decoded).expect("load resized image");
    let (resized_width, resized_height) = resized.dimensions();
    assert_eq!((resized_width, resized_height), (2048, 768));

    Ok(())
}
