use super::*;
use pretty_assertions::assert_eq;

#[test]
fn preset_names_use_mode_display_names() {
    assert_eq!(plan_preset().name, ModeKind::Plan.display_name());
    assert_eq!(default_preset().name, ModeKind::Default.display_name());
    assert_eq!(plan_preset().model, None);
    assert_eq!(
        plan_preset().reasoning_effort,
        Some(Some(ReasoningEffort::XHigh))
    );
    assert_eq!(default_preset().model, None);
    assert_eq!(default_preset().reasoning_effort, None);
}

#[test]
fn default_mode_instructions_replace_mode_names_placeholder() {
    let default_instructions = default_preset()
        .developer_instructions
        .expect("default preset should include instructions")
        .expect("default instructions should be set");

    assert!(!default_instructions.contains("{{KNOWN_MODE_NAMES}}"));

    let known_mode_names = format_mode_names(&TUI_VISIBLE_COLLABORATION_MODES);
    let expected_snippet = format!("Known mode names are {known_mode_names}.");
    assert!(default_instructions.contains(&expected_snippet));

    assert!(default_instructions.contains(
        "Use the `request_user_input` tool only when it is listed in the available tools"
    ));
    assert!(
        default_instructions.contains("ask the user directly with a concise plain-text question")
    );
}

#[test]
fn plan_mode_instructions_require_visible_delegation_decision() {
    let plan_instructions = plan_preset()
        .developer_instructions
        .expect("plan preset should include instructions")
        .expect("plan instructions should be set");

    assert!(
        plan_instructions
            .contains("A `Delegation`, `Work Split`, or `Agent ROI Estimate` section or line"),
        "plan instructions should require every proposed plan to show the agent ROI/delegation decision"
    );
    assert!(
        plan_instructions.contains("expected to lose on tokens"),
        "plan instructions should require a local-only ROI reason when no agents are intended"
    );
    assert!(
        plan_instructions.contains("loop_followup_gain"),
        "plan instructions should make loop continuations part of agent ROI"
    );
}
