#![cfg(not(target_os = "windows"))]

use anyhow::Ok;
use codex_core_test_runtime::responses::ev_assistant_message;
use codex_core_test_runtime::responses::ev_completed;
use codex_core_test_runtime::responses::ev_image_generation_call;
use codex_core_test_runtime::responses::ev_message_item_added;
use codex_core_test_runtime::responses::ev_output_text_delta;
use codex_core_test_runtime::responses::ev_reasoning_item;
use codex_core_test_runtime::responses::ev_reasoning_item_added;
use codex_core_test_runtime::responses::ev_reasoning_summary_text_delta;
use codex_core_test_runtime::responses::ev_reasoning_text_delta;
use codex_core_test_runtime::responses::ev_response_created;
use codex_core_test_runtime::responses::ev_web_search_call_added_partial;
use codex_core_test_runtime::responses::ev_web_search_call_done;
use codex_core_test_runtime::responses::mount_sse_once;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::responses::start_mock_server;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::test_codex::TestCodex;
use codex_core_test_runtime::test_codex::test_codex;
use codex_core_test_runtime::test_codex::turn_permission_fields;
use codex_core_test_runtime::wait_for_event;
use codex_core_test_runtime::wait_for_event_match;
use codex_protocol::config_types::CollaborationMode;
use codex_protocol::config_types::ModeKind;
use codex_protocol::config_types::Settings;
use codex_protocol::items::AgentMessageContent;
use codex_protocol::items::TurnItem;
use codex_protocol::models::PermissionProfile;
use codex_protocol::models::WebSearchAction;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ItemCompletedEvent;
use codex_protocol::protocol::ItemStartedEvent;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::ByteRange;
use codex_protocol::user_input::TextElement;
use codex_protocol::user_input::UserInput;
use codex_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;
use std::path::Path;
use std::path::PathBuf;

fn disabled_plan_turn(
    text: &str,
    model: String,
    collaboration_mode: CollaborationMode,
) -> anyhow::Result<Op> {
    let cwd = std::env::current_dir()?;
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(PermissionProfile::Disabled, cwd.as_path());
    Ok(Op::UserTurn {
        environments: None,
        items: vec![UserInput::Text {
            text: text.into(),
            text_elements: Vec::new(),
        }],
        final_output_json_schema: None,
        cwd,
        approval_policy: AskForApproval::Never,
        approvals_reviewer: None,
        sandbox_policy,
        permission_profile,
        model,
        effort: None,
        summary: None,
        service_tier: None,
        context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
        collaboration_mode: Some(collaboration_mode),
        personality: None,
    })
}

fn image_generation_artifact_path(codex_home: &Path, session_id: &str, call_id: &str) -> PathBuf {
    fn sanitize(value: &str) -> String {
        let mut sanitized: String = value
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    ch
                } else {
                    '_'
                }
            })
            .collect();
        if sanitized.is_empty() {
            sanitized = "generated_image".to_string();
        }
        sanitized
    }

    codex_home
        .join("generated_images")
        .join(sanitize(session_id))
        .join(format!("{}.png", sanitize(call_id)))
}
async fn image_generation_call_event_is_emitted() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;

    let TestCodex {
        codex,
        config,
        session_configured,
        ..
    } = test_codex().build(&server).await?;
    let call_id = "ig_image_saved_to_temp_dir_default";
    let expected_saved_path = image_generation_artifact_path(
        config.codex_home.as_path(),
        &session_configured.thread_id.to_string(),
        call_id,
    );
    let _ = std::fs::remove_file(&expected_saved_path);

    let first_response = sse(vec![
        ev_response_created("resp-1"),
        ev_image_generation_call(call_id, "completed", "A tiny blue square", "Zm9v"),
        ev_completed("resp-1"),
    ]);
    mount_sse_once(&server, first_response).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "generate a tiny blue square".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let started = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ItemStarted(ItemStartedEvent {
            item: TurnItem::ImageGeneration(item),
            started_at_ms,
            ..
        }) => Some((item.clone(), *started_at_ms)),
        _ => None,
    })
    .await;
    let begin = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ImageGenerationBegin(event) => Some(event.clone()),
        _ => None,
    })
    .await;
    let completed = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ItemCompleted(ItemCompletedEvent {
            item: TurnItem::ImageGeneration(item),
            completed_at_ms,
            ..
        }) => Some((item.clone(), *completed_at_ms)),
        _ => None,
    })
    .await;
    let end = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ImageGenerationEnd(event) => Some(event.clone()),
        _ => None,
    })
    .await;

    assert_eq!(begin.call_id, call_id);
    assert_eq!(started.0.id, call_id);
    assert!(started.1 > 0);
    assert_eq!(completed.0.id, call_id);
    assert!(completed.1 > 0);
    assert_eq!(end.call_id, call_id);
    assert_eq!(end.status, "completed");
    assert_eq!(end.revised_prompt, Some("A tiny blue square".to_string()));
    assert_eq!(end.result, "Zm9v");
    assert_eq!(
        end.saved_path.as_ref().map(AbsolutePathBuf::as_path),
        Some(expected_saved_path.as_path())
    );
    assert_eq!(std::fs::read(&expected_saved_path)?, b"foo");
    let _ = std::fs::remove_file(&expected_saved_path);

    Ok(())
}
async fn image_generation_call_event_is_emitted_when_image_save_fails() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;

    let TestCodex {
        codex,
        config,
        session_configured,
        ..
    } = test_codex().build(&server).await?;
    let expected_saved_path = image_generation_artifact_path(
        config.codex_home.as_path(),
        &session_configured.thread_id.to_string(),
        "ig_invalid",
    );
    let _ = std::fs::remove_file(&expected_saved_path);

    let first_response = sse(vec![
        ev_response_created("resp-1"),
        ev_image_generation_call("ig_invalid", "completed", "broken payload", "_-8"),
        ev_completed("resp-1"),
    ]);
    mount_sse_once(&server, first_response).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "generate an image".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let begin = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ImageGenerationBegin(event) => Some(event.clone()),
        _ => None,
    })
    .await;
    let end = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ImageGenerationEnd(event) => Some(event.clone()),
        _ => None,
    })
    .await;

    assert_eq!(begin.call_id, "ig_invalid");
    assert_eq!(end.call_id, "ig_invalid");
    assert_eq!(end.status, "completed");
    assert_eq!(end.revised_prompt, Some("broken payload".to_string()));
    assert_eq!(end.result, "_-8");
    assert_eq!(end.saved_path, None);
    assert!(!expected_saved_path.exists());

    Ok(())
}
