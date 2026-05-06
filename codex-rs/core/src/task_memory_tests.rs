use super::*;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use pretty_assertions::assert_eq;

fn user_message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: text.to_string(),
        }],
        phase: None,
    }
}

fn assistant_message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: text.to_string(),
        }],
        phase: None,
    }
}

fn update_plan_call(arguments: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "update_plan".to_string(),
        namespace: None,
        arguments: arguments.to_string(),
        call_id: "call-1".to_string(),
    }
}

#[test]
fn builds_near_verbatim_memory_from_latest_proposed_plan() {
    let items = vec![
        user_message("Build the native task memory feature before the final build."),
        assistant_message(
            "<proposed_plan>\n# Plan\n- inspect history\n- patch memory\n</proposed_plan>",
        ),
        user_message("go on"),
        user_message("Also throttle repeated pre-compact injections."),
    ];

    let memory = build_task_memory(&items).expect("expected task memory");

    assert!(memory.body.contains("Digest: "));
    assert!(
        memory
            .body
            .contains("Build the native task memory feature before the final build.")
    );
    assert!(memory.body.contains("- inspect history"));
    assert!(
        memory
            .body
            .contains("Also throttle repeated pre-compact injections.")
    );
    assert!(!memory.body.contains("go on"));
}

#[test]
fn latest_update_plan_is_used_when_it_is_newer_than_proposed_plan() {
    let items = vec![
        user_message("Fix the feature."),
        assistant_message("<proposed_plan>\n# Old Plan\n- old step\n</proposed_plan>"),
        update_plan_call(
            r#"{"explanation":"Need a safer order.","plan":[{"step":"inspect","status":"completed"},{"step":"patch","status":"in_progress"}]}"#,
        ),
    ];

    let memory = build_task_memory(&items).expect("expected task memory");

    assert!(memory.body.contains("Need a safer order."));
    assert!(memory.body.contains("- [completed] inspect"));
    assert!(memory.body.contains("- [in_progress] patch"));
    assert!(!memory.body.contains("old step"));
}

#[test]
fn task_memory_items_are_contextual_not_real_user_messages() {
    let item = build_task_memory_item(&[
        user_message("Keep this task visible."),
        assistant_message("<proposed_plan>\n# Plan\n- keep memory\n</proposed_plan>"),
    ])
    .expect("expected task memory item");

    assert!(task_memory_item_digest(&item).is_some());
    assert!(crate::event_mapping::parse_turn_item(&item).is_none());
    assert_eq!(real_user_message_count(&[item]), 0);
}

#[test]
fn removes_only_task_memory_items() {
    let memory = build_task_memory_item(&[
        user_message("Remember me."),
        assistant_message("<proposed_plan>\n# Plan\n- remember\n</proposed_plan>"),
    ])
    .expect("expected task memory item");
    let real = user_message("Real prompt");
    let mut items = vec![memory, real.clone()];

    remove_task_memory_items(&mut items);

    assert_eq!(items, vec![real]);
}

#[test]
fn removes_task_memory_items_even_without_digest() {
    let malformed_memory = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "<task_memory>\n# Task Memory\nmissing digest\n</task_memory>".to_string(),
        }],
        phase: None,
    };
    let real = user_message("Real prompt");
    let mut items = vec![malformed_memory, real.clone()];

    assert!(contains_task_memory_item(&items));
    assert_eq!(find_task_memory_digest(&items), None);

    remove_task_memory_items(&mut items);

    assert_eq!(items, vec![real]);
}

#[test]
fn pressure_throttle_limits_repeated_same_digest_injections() {
    let mut state = TaskMemoryThrottleState::default();
    let start = Instant::now();

    assert!(state.should_inject("digest-a", 1, start));
    assert!(!state.should_inject("digest-a", 1, start + Duration::from_secs(60)));
    assert!(state.should_inject("digest-a", 4, start + Duration::from_secs(60)));
    assert!(!state.should_inject("digest-a", 8, start + Duration::from_secs(20 * 60)));
    assert!(state.should_inject("digest-b", 8, start + Duration::from_secs(20 * 60)));
}

#[test]
fn pressure_threshold_uses_one_third_of_auto_compact_limit_capped_at_64k() {
    assert!(!should_inject_under_pressure(9_999, 30_000));
    assert!(should_inject_under_pressure(10_000, 30_000));
    assert!(!should_inject_under_pressure(63_999, 300_000));
    assert!(should_inject_under_pressure(64_000, 300_000));
}
