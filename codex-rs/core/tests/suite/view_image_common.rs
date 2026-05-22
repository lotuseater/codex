#![cfg(not(target_os = "windows"))]

pub(super) use anyhow::Context;
pub(super) use base64::Engine;
pub(super) use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
pub(super) use codex_core_test_runtime::PathBufExt;
pub(super) use codex_core_test_runtime::PathExt;
pub(super) use codex_core_test_runtime::get_remote_test_env;
pub(super) use codex_core_test_runtime::responses;
pub(super) use codex_core_test_runtime::responses::ev_assistant_message;
pub(super) use codex_core_test_runtime::responses::ev_completed;
pub(super) use codex_core_test_runtime::responses::ev_function_call;
pub(super) use codex_core_test_runtime::responses::ev_response_created;
pub(super) use codex_core_test_runtime::responses::mount_models_once;
pub(super) use codex_core_test_runtime::responses::mount_sse_sequence;
pub(super) use codex_core_test_runtime::responses::sse;
pub(super) use codex_core_test_runtime::responses::start_mock_server;
pub(super) use codex_core_test_runtime::skip_if_no_network;
pub(super) use codex_core_test_runtime::test_codex::TestCodex;
pub(super) use codex_core_test_runtime::test_codex::test_codex;
pub(super) use codex_core_test_runtime::test_codex::turn_permission_fields;
pub(super) use codex_core_test_runtime::wait_for_event_with_timeout;
pub(super) use codex_exec_server::CreateDirectoryOptions;
pub(super) use codex_exec_server::LOCAL_ENVIRONMENT_ID;
pub(super) use codex_exec_server::REMOTE_ENVIRONMENT_ID;
pub(super) use codex_exec_server::RemoveOptions;
pub(super) use codex_login::CodexAuth;
pub(super) use codex_protocol::config_types::ReasoningSummary;
pub(super) use codex_protocol::models::PermissionProfile;
pub(super) use codex_protocol::openai_models::ConfigShellToolType;
pub(super) use codex_protocol::openai_models::InputModality;
pub(super) use codex_protocol::openai_models::ModelInfo;
pub(super) use codex_protocol::openai_models::ModelVisibility;
pub(super) use codex_protocol::openai_models::ModelsResponse;
pub(super) use codex_protocol::openai_models::ReasoningEffort;
pub(super) use codex_protocol::openai_models::ReasoningEffortPreset;
pub(super) use codex_protocol::openai_models::TruncationPolicyConfig;
pub(super) use codex_protocol::permissions::FileSystemAccessMode;
pub(super) use codex_protocol::permissions::FileSystemPath;
pub(super) use codex_protocol::permissions::FileSystemSandboxEntry;
pub(super) use codex_protocol::permissions::FileSystemSandboxPolicy;
pub(super) use codex_protocol::permissions::NetworkSandboxPolicy;
pub(super) use codex_protocol::protocol::AskForApproval;
pub(super) use codex_protocol::protocol::EventMsg;
pub(super) use codex_protocol::protocol::Op;
pub(super) use codex_protocol::protocol::TurnEnvironmentSelection;
pub(super) use codex_protocol::user_input::UserInput;
pub(super) use image::DynamicImage;
pub(super) use image::GenericImageView;
pub(super) use image::ImageBuffer;
pub(super) use image::Rgba;
pub(super) use image::load_from_memory;
pub(super) use pretty_assertions::assert_eq;
pub(super) use serde_json::Value;
pub(super) use serde_json::json;
pub(super) use std::fs;
pub(super) use std::io::Cursor;
pub(super) use std::path::PathBuf;
pub(super) use std::time::SystemTime;
pub(super) use std::time::UNIX_EPOCH;
pub(super) use tempfile::TempDir;
pub(super) use tokio::time::Duration;
pub(super) use wiremock::BodyPrintLimit;
pub(super) use wiremock::MockServer;
#[cfg(not(debug_assertions))]
pub(super) use wiremock::ResponseTemplate;
#[cfg(not(debug_assertions))]
pub(super) use wiremock::matchers::body_string_contains;

pub(super) const VIEW_IMAGE_TURN_COMPLETE_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn disabled_user_turn(test: &TestCodex, items: Vec<UserInput>, model: String) -> Op {
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(PermissionProfile::Disabled, test.config.cwd.as_path());
    Op::UserTurn {
        environments: None,
        items,
        final_output_json_schema: None,
        cwd: test.config.cwd.to_path_buf(),
        approval_policy: AskForApproval::Never,
        approvals_reviewer: None,
        sandbox_policy,
        permission_profile,
        model,
        effort: None,
        summary: None,
        service_tier: None,
        context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
        collaboration_mode: None,
        personality: None,
    }
}

pub(super) fn image_messages(body: &Value) -> Vec<&Value> {
    body.get("input")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|item| {
                    item.get("type").and_then(Value::as_str) == Some("message")
                        && item
                            .get("content")
                            .and_then(Value::as_array)
                            .map(|content| {
                                content.iter().any(|span| {
                                    span.get("type").and_then(Value::as_str) == Some("input_image")
                                })
                            })
                            .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn find_image_message(body: &Value) -> Option<&Value> {
    image_messages(body).into_iter().next()
}

pub(super) fn png_bytes(width: u32, height: u32, rgba: [u8; 4]) -> anyhow::Result<Vec<u8>> {
    let image = ImageBuffer::from_pixel(width, height, Rgba(rgba));
    let mut cursor = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image).write_to(&mut cursor, image::ImageFormat::Png)?;
    Ok(cursor.into_inner())
}

pub(super) async fn create_workspace_directory(
    test: &TestCodex,
    rel_path: &str,
) -> anyhow::Result<PathBuf> {
    let abs_path = test.config.cwd.join(rel_path);
    test.fs()
        .create_directory(
            &abs_path,
            CreateDirectoryOptions { recursive: true },
            /*sandbox*/ None,
        )
        .await?;
    Ok(abs_path.into_path_buf())
}

pub(super) async fn write_workspace_file(
    test: &TestCodex,
    rel_path: &str,
    contents: Vec<u8>,
) -> anyhow::Result<PathBuf> {
    let abs_path = test.config.cwd.join(rel_path);
    if let Some(parent) = abs_path.parent() {
        test.fs()
            .create_directory(
                &parent,
                CreateDirectoryOptions { recursive: true },
                /*sandbox*/ None,
            )
            .await?;
    }
    test.fs()
        .write_file(&abs_path, contents, /*sandbox*/ None)
        .await?;
    Ok(abs_path.into_path_buf())
}

pub(super) async fn write_workspace_png(
    test: &TestCodex,
    rel_path: &str,
    width: u32,
    height: u32,
    rgba: [u8; 4],
) -> anyhow::Result<PathBuf> {
    write_workspace_file(test, rel_path, png_bytes(width, height, rgba)?).await
}

pub(super) async fn assert_user_turn_local_image_resizes_to(
    original_dimensions: (u32, u32),
    expected_dimensions: (u32, u32),
) -> anyhow::Result<()> {
    let server = start_mock_server().await;

    let mut builder = test_codex();
    let test = builder.build_remote_aware(&server).await?;
    let TestCodex {
        codex,
        session_configured,
        ..
    } = &test;

    let (original_width, original_height) = original_dimensions;
    let local_image_dir = tempfile::tempdir()?;
    let abs_path = local_image_dir.path().join("example.png");
    let image = ImageBuffer::from_pixel(original_width, original_height, Rgba([20u8, 40, 60, 255]));
    image.save(&abs_path)?;

    let response = sse(vec![
        ev_response_created("resp-1"),
        ev_assistant_message("msg-1", "done"),
        ev_completed("resp-1"),
    ]);
    let mock = responses::mount_sse_once(&server, response).await;

    let session_model = session_configured.model.clone();

    codex
        .submit(disabled_user_turn(
            &test,
            vec![UserInput::LocalImage {
                path: abs_path.clone(),
            }],
            session_model,
        ))
        .await?;

    wait_for_event_with_timeout(
        codex,
        |event| matches!(event, EventMsg::TurnComplete(_)),
        // Empirically, image attachment can be slow under Bazel/RBE.
        VIEW_IMAGE_TURN_COMPLETE_TIMEOUT,
    )
    .await;

    let body = mock.single_request().body_json();
    let image_message =
        find_image_message(&body).context("pending input image message not included in request")?;
    let image_url = image_message
        .get("content")
        .and_then(Value::as_array)
        .and_then(|content| {
            content.iter().find_map(|span| {
                if span.get("type").and_then(Value::as_str) == Some("input_image") {
                    span.get("image_url").and_then(Value::as_str)
                } else {
                    None
                }
            })
        })
        .context("image_url present")?;

    let (prefix, encoded) = image_url
        .split_once(',')
        .context("image url contains data prefix")?;
    assert_eq!(prefix, "data:image/png;base64");

    let decoded = BASE64_STANDARD
        .decode(encoded)
        .context("image data decodes from base64 for request")?;
    let resized = load_from_memory(&decoded).context("load resized image")?;
    let (width, height) = resized.dimensions();
    assert_eq!((width, height), expected_dimensions);

    Ok(())
}
