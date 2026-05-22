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
async fn plan_mode_streaming_citations_are_stripped_across_added_deltas_and_done()
-> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;

    let TestCodex {
        codex,
        session_configured,
        ..
    } = test_codex().build(&server).await?;

    let added_text = "Intro <oai-mem-";
    let deltas = [
        "citation>outer-doc</oai-mem-citation>\n<proposed",
        "_plan>\n- Step 1<oai-mem-",
        "citation>plan-doc</oai-mem-citation>\n- Step 2\n</proposed_plan>\nOu",
        "tro",
    ];
    let full_message = format!("{added_text}{}", deltas.concat());

    let mut events = vec![
        ev_response_created("resp-1"),
        ev_message_item_added("msg-1", added_text),
    ];
    for delta in deltas {
        events.push(ev_output_text_delta(delta));
    }
    events.push(ev_assistant_message("msg-1", &full_message));
    events.push(ev_completed("resp-1"));
    mount_sse_once(&server, sse(events)).await;

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
            "please plan with citations",
            session_configured.model.clone(),
            collaboration_mode,
        )?)
        .await?;

    let mut agent_started = None;
    let mut agent_started_idx = None;
    let mut agent_completed = None;
    let mut agent_completed_idx = None;
    let mut plan_started = None;
    let mut plan_started_idx = None;
    let mut plan_completed = None;
    let mut plan_completed_idx = None;
    let mut agent_deltas = Vec::new();
    let mut plan_deltas = Vec::new();
    let mut first_agent_delta_idx = None;
    let mut last_agent_delta_idx = None;
    let mut first_plan_delta_idx = None;
    let mut last_plan_delta_idx = None;
    let mut idx = 0usize;

    let turn_complete_idx = loop {
        let ev = wait_for_event(&codex, |_| true).await;
        match ev {
            EventMsg::ItemStarted(ItemStartedEvent {
                item: TurnItem::AgentMessage(item),
                ..
            }) => {
                agent_started_idx = Some(idx);
                agent_started = Some(item);
            }
            EventMsg::ItemStarted(ItemStartedEvent {
                item: TurnItem::Plan(item),
                ..
            }) => {
                plan_started_idx = Some(idx);
                plan_started = Some(item);
            }
            EventMsg::AgentMessageContentDelta(event) => {
                if first_agent_delta_idx.is_none() {
                    first_agent_delta_idx = Some(idx);
                }
                last_agent_delta_idx = Some(idx);
                agent_deltas.push(event.delta);
            }
            EventMsg::PlanDelta(event) => {
                if first_plan_delta_idx.is_none() {
                    first_plan_delta_idx = Some(idx);
                }
                last_plan_delta_idx = Some(idx);
                plan_deltas.push(event.delta);
            }
            EventMsg::ItemCompleted(ItemCompletedEvent {
                item: TurnItem::AgentMessage(item),
                ..
            }) => {
                agent_completed_idx = Some(idx);
                agent_completed = Some(item);
            }
            EventMsg::ItemCompleted(ItemCompletedEvent {
                item: TurnItem::Plan(item),
                ..
            }) => {
                plan_completed_idx = Some(idx);
                plan_completed = Some(item);
            }
            EventMsg::TurnComplete(_) => {
                break idx;
            }
            _ => {}
        }
        idx += 1;
    };

    let agent_started = agent_started.expect("agent item start should be emitted");
    let agent_completed = agent_completed.expect("agent item completion should be emitted");
    let plan_started = plan_started.expect("plan item start should be emitted");
    let plan_completed = plan_completed.expect("plan item completion should be emitted");

    assert_eq!(agent_started.id, agent_completed.id);
    assert_eq!(plan_started.id, plan_completed.id);
    assert_eq!(plan_started.text, "");

    let agent_started_text: String = agent_started
        .content
        .iter()
        .map(|entry| match entry {
            AgentMessageContent::Text { text } => text.as_str(),
        })
        .collect();
    let agent_completed_text: String = agent_completed
        .content
        .iter()
        .map(|entry| match entry {
            AgentMessageContent::Text { text } => text.as_str(),
        })
        .collect();
    let agent_delta_text = agent_deltas.concat();
    let plan_delta_text = plan_deltas.concat();

    assert_eq!(agent_started_text, "");
    assert_eq!(agent_delta_text, "Intro \nOutro");
    assert_eq!(agent_completed_text, "Intro \nOutro");
    assert_eq!(plan_delta_text, "- Step 1\n- Step 2\n");
    assert_eq!(plan_completed.text, "- Step 1\n- Step 2\n");

    for text in [
        agent_started_text.as_str(),
        agent_delta_text.as_str(),
        agent_completed_text.as_str(),
        plan_delta_text.as_str(),
        plan_completed.text.as_str(),
    ] {
        assert!(!text.contains("<oai-mem-citation>"));
        assert!(!text.contains("</oai-mem-citation>"));
    }

    let agent_started_idx = agent_started_idx.expect("agent start index");
    let agent_completed_idx = agent_completed_idx.expect("agent completion index");
    let plan_started_idx = plan_started_idx.expect("plan start index");
    let plan_completed_idx = plan_completed_idx.expect("plan completion index");
    let first_agent_delta_idx = first_agent_delta_idx.expect("agent delta index");
    let last_agent_delta_idx = last_agent_delta_idx.expect("agent delta index");
    let first_plan_delta_idx = first_plan_delta_idx.expect("plan delta index");
    let last_plan_delta_idx = last_plan_delta_idx.expect("plan delta index");
    assert!(agent_started_idx < first_agent_delta_idx);
    assert!(plan_started_idx < first_plan_delta_idx);
    assert!(last_agent_delta_idx < agent_completed_idx);
    assert!(last_plan_delta_idx < plan_completed_idx);
    assert!(agent_completed_idx < turn_complete_idx);
    assert!(plan_completed_idx < turn_complete_idx);

    Ok(())
}
async fn plan_mode_streaming_proposed_plan_tag_split_across_added_and_delta_is_parsed()
-> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;

    let TestCodex {
        codex,
        session_configured,
        ..
    } = test_codex().build(&server).await?;

    let added_text = "Intro\n<proposed";
    let deltas = ["_plan>\n- Step 1\n</proposed_plan>\nOutro"];
    let full_message = format!("{added_text}{}", deltas.concat());

    let mut events = vec![
        ev_response_created("resp-1"),
        ev_message_item_added("msg-1", added_text),
    ];
    for delta in deltas {
        events.push(ev_output_text_delta(delta));
    }
    events.push(ev_assistant_message("msg-1", &full_message));
    events.push(ev_completed("resp-1"));
    mount_sse_once(&server, sse(events)).await;

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

    let mut agent_started = None;
    let mut agent_completed = None;
    let mut plan_started = None;
    let mut plan_completed = None;
    let mut agent_deltas = Vec::new();
    let mut plan_deltas = Vec::new();

    loop {
        let ev = wait_for_event(&codex, |_| true).await;
        match ev {
            EventMsg::ItemStarted(ItemStartedEvent {
                item: TurnItem::AgentMessage(item),
                ..
            }) => agent_started = Some(item),
            EventMsg::ItemStarted(ItemStartedEvent {
                item: TurnItem::Plan(item),
                ..
            }) => plan_started = Some(item),
            EventMsg::AgentMessageContentDelta(event) => agent_deltas.push(event.delta),
            EventMsg::PlanDelta(event) => plan_deltas.push(event.delta),
            EventMsg::ItemCompleted(ItemCompletedEvent {
                item: TurnItem::AgentMessage(item),
                ..
            }) => agent_completed = Some(item),
            EventMsg::ItemCompleted(ItemCompletedEvent {
                item: TurnItem::Plan(item),
                ..
            }) => plan_completed = Some(item),
            EventMsg::TurnComplete(_) => break,
            _ => {}
        }
    }

    let agent_started = agent_started.expect("agent item start should be emitted");
    let agent_completed = agent_completed.expect("agent item completion should be emitted");
    let plan_started = plan_started.expect("plan item start should be emitted");
    let plan_completed = plan_completed.expect("plan item completion should be emitted");

    let agent_started_text: String = agent_started
        .content
        .iter()
        .map(|entry| match entry {
            AgentMessageContent::Text { text } => text.as_str(),
        })
        .collect();
    let agent_completed_text: String = agent_completed
        .content
        .iter()
        .map(|entry| match entry {
            AgentMessageContent::Text { text } => text.as_str(),
        })
        .collect();

    assert_eq!(agent_started_text, "");
    assert_eq!(agent_deltas.concat(), "Intro\nOutro");
    assert_eq!(agent_completed_text, "Intro\nOutro");
    assert_eq!(plan_started.text, "");
    assert_eq!(plan_deltas.concat(), "- Step 1\n");
    assert_eq!(plan_completed.text, "- Step 1\n");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn plan_mode_handles_missing_plan_close_tag() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;

    let TestCodex {
        codex,
        session_configured,
        ..
    } = test_codex().build(&server).await?;

    let full_message = "Intro\n<proposed_plan>\n- Step 1\n";
    let stream = sse(vec![
        ev_response_created("resp-1"),
        ev_message_item_added("msg-1", ""),
        ev_output_text_delta(full_message),
        ev_assistant_message("msg-1", full_message),
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

    let mut plan_delta = None;
    let mut plan_item = None;
    let mut agent_item = None;

    while plan_delta.is_none() || plan_item.is_none() || agent_item.is_none() {
        let ev = wait_for_event(&codex, |_| true).await;
        match ev {
            EventMsg::PlanDelta(event) => {
                plan_delta = Some(event.delta);
            }
            EventMsg::ItemCompleted(ItemCompletedEvent {
                item: TurnItem::Plan(item),
                ..
            }) => {
                plan_item = Some(item);
            }
            EventMsg::ItemCompleted(ItemCompletedEvent {
                item: TurnItem::AgentMessage(item),
                ..
            }) => {
                agent_item = Some(item);
            }
            _ => {}
        }
    }

    assert_eq!(plan_delta.unwrap(), "- Step 1\n");
    assert_eq!(plan_item.unwrap().text, "- Step 1\n");
    let agent_text_from_item: String = agent_item
        .unwrap()
        .content
        .iter()
        .map(|entry| match entry {
            AgentMessageContent::Text { text } => text.as_str(),
        })
        .collect();
    assert_eq!(agent_text_from_item, "Intro\n");

    Ok(())
}
