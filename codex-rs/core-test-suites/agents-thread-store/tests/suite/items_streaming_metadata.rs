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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_message_content_delta_has_item_metadata() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;

    let TestCodex {
        codex,
        session_configured,
        ..
    } = test_codex().build(&server).await?;

    let stream = sse(vec![
        ev_response_created("resp-1"),
        ev_message_item_added("msg-1", ""),
        ev_output_text_delta("streamed response"),
        ev_assistant_message("msg-1", "streamed response"),
        ev_completed("resp-1"),
    ]);
    mount_sse_once(&server, stream).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "please stream text".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let (started_turn_id, started_item) = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ItemStarted(ItemStartedEvent {
            turn_id,
            item: TurnItem::AgentMessage(item),
            ..
        }) => Some((turn_id.clone(), item.clone())),
        _ => None,
    })
    .await;

    let delta_event = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::AgentMessageContentDelta(event) => Some(event.clone()),
        _ => None,
    })
    .await;
    let completed_item = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ItemCompleted(ItemCompletedEvent {
            item: TurnItem::AgentMessage(item),
            ..
        }) => Some(item.clone()),
        _ => None,
    })
    .await;

    let thread_id = session_configured.thread_id.to_string();
    assert_eq!(delta_event.thread_id, thread_id);
    assert_eq!(delta_event.turn_id, started_turn_id);
    assert_eq!(delta_event.item_id, started_item.id);
    assert_eq!(delta_event.delta, "streamed response");
    assert_eq!(completed_item.id, started_item.id);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]

async fn reasoning_content_delta_has_item_metadata() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;

    let TestCodex { codex, .. } = test_codex().build(&server).await?;

    let stream = sse(vec![
        ev_response_created("resp-1"),
        ev_reasoning_item_added("reasoning-1", &[""]),
        ev_reasoning_summary_text_delta("step one"),
        ev_reasoning_item("reasoning-1", &["step one"], &[]),
        ev_completed("resp-1"),
    ]);
    mount_sse_once(&server, stream).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "reason through it".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let reasoning_item = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ItemStarted(ItemStartedEvent {
            item: TurnItem::Reasoning(item),
            ..
        }) => Some(item.clone()),
        _ => None,
    })
    .await;

    let delta_event = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ReasoningContentDelta(event) => Some(event.clone()),
        _ => None,
    })
    .await;
    assert_eq!(delta_event.item_id, reasoning_item.id);
    assert_eq!(delta_event.delta, "step one");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reasoning_raw_content_delta_respects_flag() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;

    let TestCodex { codex, .. } = test_codex()
        .with_config(|config| {
            config.show_raw_agent_reasoning = true;
        })
        .build(&server)
        .await?;

    let stream = sse(vec![
        ev_response_created("resp-1"),
        ev_reasoning_item_added("reasoning-raw", &[""]),
        ev_reasoning_text_delta("raw detail"),
        ev_reasoning_item("reasoning-raw", &["complete"], &["raw detail"]),
        ev_completed("resp-1"),
    ]);
    mount_sse_once(&server, stream).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "show raw reasoning".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let reasoning_item = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ItemStarted(ItemStartedEvent {
            item: TurnItem::Reasoning(item),
            ..
        }) => Some(item.clone()),
        _ => None,
    })
    .await;

    let delta_event = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ReasoningRawContentDelta(event) => Some(event.clone()),
        _ => None,
    })
    .await;
    assert_eq!(delta_event.item_id, reasoning_item.id);
    assert_eq!(delta_event.delta, "raw detail");

    Ok(())
}
