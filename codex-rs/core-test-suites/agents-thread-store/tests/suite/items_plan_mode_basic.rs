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
async fn plan_mode_emits_plan_item_from_proposed_plan_block() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;

    let TestCodex {
        codex,
        session_configured,
        ..
    } = test_codex().build(&server).await?;

    let plan_block = "<proposed_plan>\n- Step 1\n- Step 2\n</proposed_plan>\n";
    let full_message = format!("Intro\n{plan_block}Outro");
    let stream = sse(vec![
        ev_response_created("resp-1"),
        ev_message_item_added("msg-1", ""),
        ev_output_text_delta(&full_message),
        ev_assistant_message("msg-1", &full_message),
        ev_completed("resp-1"),
    ]);
    mount_sse_once(&server, stream).await;

    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Plan,
        settings: Settings {
            model: session_configured.model.clone(),
            reasoning_effort: None,
            developer_instructions: None,
        },
    };

    codex
        .submit(disabled_plan_turn(
            "please plan",
            session_configured.model.clone(),
            collaboration_mode,
        )?)
        .await?;

    let plan_delta = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::PlanDelta(event) => Some(event.clone()),
        _ => None,
    })
    .await;

    let plan_completed = wait_for_event_match(&codex, |ev| match ev {
        EventMsg::ItemCompleted(ItemCompletedEvent {
            item: TurnItem::Plan(item),
            ..
        }) => Some(item.clone()),
        _ => None,
    })
    .await;

    assert_eq!(
        plan_delta.thread_id,
        session_configured.thread_id.to_string()
    );
    assert_eq!(plan_delta.delta, "- Step 1\n- Step 2\n");
    assert_eq!(plan_completed.text, "- Step 1\n- Step 2\n");

    Ok(())
}
async fn plan_mode_strips_plan_from_agent_messages() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;

    let TestCodex {
        codex,
        session_configured,
        ..
    } = test_codex().build(&server).await?;

    let plan_block = "<proposed_plan>\n- Step 1\n- Step 2\n</proposed_plan>\n";
    let full_message = format!("Intro\n{plan_block}Outro");
    let stream = sse(vec![
        ev_response_created("resp-1"),
        ev_message_item_added("msg-1", ""),
        ev_output_text_delta(&full_message),
        ev_assistant_message("msg-1", &full_message),
        ev_completed("resp-1"),
    ]);
    mount_sse_once(&server, stream).await;

    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Plan,
        settings: Settings {
            model: session_configured.model.clone(),
            reasoning_effort: None,
            developer_instructions: None,
        },
    };

    codex
        .submit(disabled_plan_turn(
            "please plan",
            session_configured.model.clone(),
            collaboration_mode,
        )?)
        .await?;

    let mut agent_deltas = Vec::new();
    let mut plan_delta = None;
    let mut agent_item = None;
    let mut plan_item = None;

    while plan_delta.is_none() || agent_item.is_none() || plan_item.is_none() {
        let ev = wait_for_event(&codex, |_| true).await;
        match ev {
            EventMsg::AgentMessageContentDelta(event) => {
                agent_deltas.push(event.delta);
            }
            EventMsg::PlanDelta(event) => {
                plan_delta = Some(event.delta);
            }
            EventMsg::ItemCompleted(ItemCompletedEvent {
                item: TurnItem::AgentMessage(item),
                ..
            }) => {
                agent_item = Some(item);
            }
            EventMsg::ItemCompleted(ItemCompletedEvent {
                item: TurnItem::Plan(item),
                ..
            }) => {
                plan_item = Some(item);
            }
            _ => {}
        }
    }

    let agent_text = agent_deltas.concat();
    assert_eq!(agent_text, "Intro\nOutro");
    assert_eq!(plan_delta.unwrap(), "- Step 1\n- Step 2\n");
    assert_eq!(plan_item.unwrap().text, "- Step 1\n- Step 2\n");
    let agent_text_from_item: String = agent_item
        .unwrap()
        .content
        .iter()
        .map(|entry| match entry {
            AgentMessageContent::Text { text } => text.as_str(),
        })
        .collect();
    assert_eq!(agent_text_from_item, "Intro\nOutro");

    Ok(())
}
