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
async fn user_message_item_is_emitted() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;

    let TestCodex { codex, .. } = test_codex().build(&server).await?;

    let first_response = sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]);
    mount_sse_once(&server, first_response).await;

    let text_elements = vec![TextElement::new(
        ByteRange { start: 0, end: 6 },
        Some("<file>".into()),
    )];
    let expected_input = UserInput::Text {
        text: "please inspect sample.txt".into(),
        text_elements: text_elements.clone(),
    };

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![expected_input.clone()],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let started_item = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ItemStarted(ItemStartedEvent {
            item: TurnItem::UserMessage(item),
            ..
        }) => Some(item.clone()),
        _ => None,
    })
    .await;
    let completed_item = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ItemCompleted(ItemCompletedEvent {
            item: TurnItem::UserMessage(item),
            ..
        }) => Some(item.clone()),
        _ => None,
    })
    .await;

    assert_eq!(started_item.id, completed_item.id);
    assert_eq!(started_item.content, vec![expected_input.clone()]);
    assert_eq!(completed_item.content, vec![expected_input]);

    let legacy_message = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::UserMessage(event) => Some(event.clone()),
        _ => None,
    })
    .await;
    assert_eq!(legacy_message.message, "please inspect sample.txt");
    assert_eq!(legacy_message.text_elements, text_elements);
    Ok(())
}
async fn assistant_message_item_is_emitted() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;

    let TestCodex { codex, .. } = test_codex().build(&server).await?;

    let first_response = sse(vec![
        ev_response_created("resp-1"),
        ev_assistant_message("msg-1", "all done"),
        ev_completed("resp-1"),
    ]);
    mount_sse_once(&server, first_response).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "please summarize results".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let started = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ItemStarted(ItemStartedEvent {
            item: TurnItem::AgentMessage(item),
            ..
        }) => Some(item.clone()),
        _ => None,
    })
    .await;
    let completed = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ItemCompleted(ItemCompletedEvent {
            item: TurnItem::AgentMessage(item),
            ..
        }) => Some(item.clone()),
        _ => None,
    })
    .await;

    assert_eq!(started.id, completed.id);
    let Some(codex_protocol::items::AgentMessageContent::Text { text }) = completed.content.first()
    else {
        panic!("expected agent message text content");
    };
    assert_eq!(text, "all done");

    Ok(())
}
async fn reasoning_item_is_emitted() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;

    let TestCodex { codex, .. } = test_codex().build(&server).await?;

    let reasoning_item = ev_reasoning_item(
        "reasoning-1",
        &["Consider inputs", "Compute output"],
        &["Detailed reasoning trace"],
    );

    let first_response = sse(vec![
        ev_response_created("resp-1"),
        reasoning_item,
        ev_completed("resp-1"),
    ]);
    mount_sse_once(&server, first_response).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "explain your reasoning".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let started = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ItemStarted(ItemStartedEvent {
            item: TurnItem::Reasoning(item),
            ..
        }) => Some(item.clone()),
        _ => None,
    })
    .await;
    let completed = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ItemCompleted(ItemCompletedEvent {
            item: TurnItem::Reasoning(item),
            ..
        }) => Some(item.clone()),
        _ => None,
    })
    .await;

    assert_eq!(started.id, completed.id);
    assert_eq!(
        completed.summary_text,
        vec!["Consider inputs".to_string(), "Compute output".to_string()]
    );
    assert_eq!(
        completed.raw_content,
        vec!["Detailed reasoning trace".to_string()]
    );

    Ok(())
}
async fn web_search_item_is_emitted() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;

    let TestCodex { codex, .. } = test_codex().build(&server).await?;

    let web_search_added = ev_web_search_call_added_partial("web-search-1", "in_progress");
    let web_search_done = ev_web_search_call_done("web-search-1", "completed", "weather seattle");

    let first_response = sse(vec![
        ev_response_created("resp-1"),
        web_search_added,
        web_search_done,
        ev_completed("resp-1"),
    ]);
    mount_sse_once(&server, first_response).await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "find the weather".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let started = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ItemStarted(ItemStartedEvent {
            item: TurnItem::WebSearch(item),
            started_at_ms,
            ..
        }) => Some((item.clone(), *started_at_ms)),
        _ => None,
    })
    .await;
    let begin = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::WebSearchBegin(event) => Some(event.clone()),
        _ => None,
    })
    .await;
    let completed = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ItemCompleted(ItemCompletedEvent {
            item: TurnItem::WebSearch(item),
            completed_at_ms,
            ..
        }) => Some((item.clone(), *completed_at_ms)),
        _ => None,
    })
    .await;

    assert_eq!(begin.call_id, "web-search-1");
    assert_eq!(started.0.id, begin.call_id);
    assert!(started.1 > 0);
    assert_eq!(completed.0.id, begin.call_id);
    assert!(completed.1 > 0);
    assert_eq!(
        completed.0.action,
        WebSearchAction::Search {
            query: Some("weather seattle".to_string()),
            queries: None,
        }
    );

    Ok(())
}
