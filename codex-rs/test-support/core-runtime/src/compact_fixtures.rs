use std::path::PathBuf;
use std::sync::Arc;

use codex_core::CodexThread;
use codex_core::compact::SUMMARIZATION_PROMPT;
use codex_core::compact::SUMMARY_PREFIX;
use codex_core::config::Config;
pub use codex_test_support_context_fixtures::compact_fixtures::{
    COMPACT_WARNING_MESSAGE, FIRST_REPLY, SUMMARY_TEXT, auto_summary, body_contains_text,
    read_hook_inputs, write_matching_compact_hooks, write_unsupported_blocking_pre_compact_hook,
};
use codex_protocol::config_types::CollaborationMode;
use codex_protocol::config_types::ContextBudgetMode;
use codex_protocol::config_types::ModeKind;
use codex_protocol::config_types::Settings;
use codex_protocol::items::TurnItem;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ItemCompletedEvent;
use codex_protocol::protocol::ItemStartedEvent;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;

use crate::test_codex::turn_permission_fields;

pub fn disabled_permission_user_turn(text: impl Into<String>, cwd: PathBuf, model: String) -> Op {
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(PermissionProfile::Disabled, cwd.as_path());
    Op::UserTurn {
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
        context_budget_mode: Some(ContextBudgetMode::Standard),
        collaboration_mode: None,
        personality: None,
    }
}

pub fn disabled_permission_plan_turn(text: impl Into<String>, cwd: PathBuf, model: String) -> Op {
    let mut op = disabled_permission_user_turn(text, cwd, model.clone());
    let Op::UserTurn {
        collaboration_mode, ..
    } = &mut op
    else {
        unreachable!("disabled_permission_user_turn always returns Op::UserTurn");
    };
    *collaboration_mode = Some(CollaborationMode {
        mode: ModeKind::Plan,
        settings: Settings {
            model,
            reasoning_effort: None,
            developer_instructions: None,
        },
    });
    op
}

pub fn summary_with_prefix(summary: &str) -> String {
    format!("{SUMMARY_PREFIX}\n{summary}")
}

pub fn set_test_compact_prompt(config: &mut Config) {
    config.compact_prompt = Some(SUMMARIZATION_PROMPT.to_string());
}

pub fn body_contains_compaction_summary_prefix(body: &str) -> bool {
    body_contains_text(body, SUMMARY_PREFIX)
}

pub fn body_contains_compaction_prompt(body: &str) -> bool {
    body_contains_text(body, SUMMARIZATION_PROMPT)
}

pub fn compact_prompt() -> &'static str {
    SUMMARIZATION_PROMPT
}

pub fn assert_pre_sampling_switch_compaction_requests(
    first: &serde_json::Value,
    compact: &serde_json::Value,
    follow_up: &serde_json::Value,
    previous_model: &str,
    next_model: &str,
) {
    assert_eq!(first["model"].as_str(), Some(previous_model));
    assert_eq!(compact["model"].as_str(), Some(previous_model));
    assert_eq!(follow_up["model"].as_str(), Some(next_model));

    let compact_body = compact.to_string();
    assert!(
        body_contains_text(&compact_body, SUMMARIZATION_PROMPT),
        "pre-sampling compact request should include summarization prompt"
    );
    assert!(
        !compact_body.contains("<model_switch>"),
        "pre-sampling compact request should strip trailing model-switch update item"
    );
    let follow_up_body = follow_up.to_string();
    assert!(
        follow_up_body.contains("<model_switch>"),
        "follow-up request after successful model-switch compaction should include model-switch update item"
    );
}

pub async fn assert_compaction_uses_turn_lifecycle_id(codex: &Arc<CodexThread>) {
    let mut turn_started_id = None;
    let mut turn_completed_id = None;
    let mut compact_started_id = None;
    let mut compact_completed_id = None;

    while turn_completed_id.is_none() {
        let event = codex.next_event().await.expect("next event");
        match event.msg {
            EventMsg::TurnStarted(_) => turn_started_id = Some(event.id.clone()),
            EventMsg::ItemStarted(ItemStartedEvent {
                item: TurnItem::ContextCompaction(_),
                ..
            }) => compact_started_id = Some(event.id.clone()),
            EventMsg::ItemCompleted(ItemCompletedEvent {
                item: TurnItem::ContextCompaction(_),
                ..
            }) => compact_completed_id = Some(event.id.clone()),
            EventMsg::TurnComplete(_) => turn_completed_id = Some(event.id.clone()),
            _ => {}
        }
    }

    let turn_started_id = turn_started_id.expect("turn started id");
    let turn_completed_id = turn_completed_id.expect("turn complete id");

    assert_eq!(
        turn_completed_id, turn_started_id,
        "turn start and complete should use the same event id"
    );
    assert_eq!(
        compact_started_id,
        Some(turn_started_id.clone()),
        "compaction item start should use the turn event id"
    );
    assert_eq!(
        compact_completed_id,
        Some(turn_started_id),
        "compaction item completion should use the turn event id"
    );
}
