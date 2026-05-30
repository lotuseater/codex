use super::super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn workspace_member_credits_depleted_prompts_and_sends_credits() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let mut limits = snapshot(/*percent*/ 100.0);
    limits.rate_limit_reached_type = Some(RateLimitReachedType::WorkspaceMemberCreditsDepleted);
    chat.on_rate_limit_snapshot(Some(limits));

    chat.on_rate_limit_error(
        RateLimitErrorKind::Generic,
        "Usage limit reached.".to_string(),
    );
    let popup = render_bottom_popup(&chat, /*width*/ 90);
    assert_chatwidget_snapshot!("workspace_member_credits_depleted_prompt", popup);

    chat.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    let event = next_send_add_credits_nudge_email_event(&mut rx);
    assert_eq!(event, AddCreditsNudgeCreditType::Credits);
}

#[tokio::test]
async fn workspace_member_usage_limit_prompts_and_sends_usage_limit() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let mut limits = snapshot(/*percent*/ 100.0);
    limits.rate_limit_reached_type = Some(RateLimitReachedType::WorkspaceMemberUsageLimitReached);
    chat.on_rate_limit_snapshot(Some(limits));

    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "Usage limit reached.".to_string(),
    );
    let popup = render_bottom_popup(&chat, /*width*/ 100);
    assert_chatwidget_snapshot!("workspace_member_usage_limit_prompt", popup);

    chat.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    let event = next_send_add_credits_nudge_email_event(&mut rx);
    assert_eq!(event, AddCreditsNudgeCreditType::UsageLimit);
}

#[tokio::test]
async fn header_rate_limit_snapshot_preserves_member_limit_type_for_error_prompt() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let mut usage_limits = snapshot(/*percent*/ 100.0);
    usage_limits.rate_limit_reached_type =
        Some(RateLimitReachedType::WorkspaceMemberUsageLimitReached);
    chat.on_rate_limit_snapshot(Some(usage_limits));

    // Turn-failure snapshots are derived from response headers and do not carry
    // the backend-classified reached type. They arrive before the Error event.
    let mut header_limits = snapshot(/*percent*/ 100.0);
    header_limits.rate_limit_reached_type = None;
    chat.on_rate_limit_snapshot(Some(header_limits));

    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "Usage limit reached.".to_string(),
    );
    let popup = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        popup.contains("Request a limit increase from your owner"),
        "popup: {popup}"
    );

    chat.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    let event = next_send_add_credits_nudge_email_event(&mut rx);
    assert_eq!(event, AddCreditsNudgeCreditType::UsageLimit);
}

#[tokio::test]
async fn usage_limit_error_remaps_stale_member_credits_state_to_usage_limit_prompt() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let mut limits = snapshot(/*percent*/ 100.0);
    limits.rate_limit_reached_type = Some(RateLimitReachedType::WorkspaceMemberCreditsDepleted);
    chat.on_rate_limit_snapshot(Some(limits));

    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "Usage limit reached.".to_string(),
    );
    let popup = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        popup.contains("Request a limit increase from your owner"),
        "popup: {popup}"
    );

    chat.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    let event = next_send_add_credits_nudge_email_event(&mut rx);
    assert_eq!(event, AddCreditsNudgeCreditType::UsageLimit);
}

#[tokio::test]
async fn workspace_owner_limit_states_do_not_prompt_for_owner_nudge() {
    for (limit_type, error_kind) in [
        (
            RateLimitReachedType::WorkspaceOwnerCreditsDepleted,
            RateLimitErrorKind::Generic,
        ),
        (
            RateLimitReachedType::WorkspaceOwnerUsageLimitReached,
            RateLimitErrorKind::UsageLimit,
        ),
        (
            RateLimitReachedType::RateLimitReached,
            RateLimitErrorKind::Generic,
        ),
    ] {
        let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
        let mut limits = snapshot(/*percent*/ 100.0);
        limits.rate_limit_reached_type = Some(limit_type);
        chat.on_rate_limit_snapshot(Some(limits));

        chat.on_rate_limit_error(error_kind, "Usage limit reached.".to_string());
        let popup = render_bottom_popup(&chat, /*width*/ 90);
        assert!(!popup.contains("workspace owner"));
        assert_no_owner_nudge_or_rate_limit_refresh(&mut rx);
    }
}

#[tokio::test]
async fn workspace_owner_limit_states_render_state_specific_messages() {
    let cases = [
        (
            RateLimitReachedType::WorkspaceOwnerCreditsDepleted,
            RateLimitErrorKind::Generic,
            "You're out of credits. Your workspace is out of credits. Add credits to continue using Codex.",
        ),
        (
            RateLimitReachedType::WorkspaceOwnerUsageLimitReached,
            RateLimitErrorKind::UsageLimit,
            "Usage limit reached. You've reached your usage limit. Increase your limits to continue using codex.",
        ),
    ];

    let mut rendered_cases = Vec::new();
    for (limit_type, error_kind, expected) in cases {
        let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
        let mut limits = snapshot(/*percent*/ 100.0);
        limits.rate_limit_reached_type = Some(limit_type);
        chat.on_rate_limit_snapshot(Some(limits));

        chat.on_rate_limit_error(error_kind, "Usage limit reached.".to_string());
        let rendered = drain_insert_history(&mut rx)
            .into_iter()
            .map(|lines| lines_to_single_string(&lines))
            .collect::<String>();
        assert!(rendered.contains(expected), "rendered: {rendered}");
        rendered_cases.push(rendered);
    }

    assert_chatwidget_snapshot!(
        "workspace_owner_limit_state_messages",
        rendered_cases.join("\n---\n")
    );
}

#[tokio::test]
async fn missing_rate_limit_reached_type_does_not_prompt_or_refresh() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.on_rate_limit_snapshot(Some(snapshot(/*percent*/ 100.0)));

    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "Usage limit reached.".to_string(),
    );
    let popup = render_bottom_popup(&chat, /*width*/ 90);
    assert!(!popup.contains("workspace owner"));
    assert_no_owner_nudge_or_rate_limit_refresh(&mut rx);
}

#[tokio::test]
async fn workspace_owner_nudge_default_no_dismisses_without_sending() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let mut limits = snapshot(/*percent*/ 100.0);
    limits.rate_limit_reached_type = Some(RateLimitReachedType::WorkspaceMemberCreditsDepleted);
    chat.on_rate_limit_snapshot(Some(limits));

    chat.on_rate_limit_error(
        RateLimitErrorKind::Generic,
        "Usage limit reached.".to_string(),
    );
    chat.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_no_owner_nudge_or_rate_limit_refresh(&mut rx);
}

#[tokio::test]
async fn workspace_owner_nudge_reappears_after_dismissing_no() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let mut limits = snapshot(/*percent*/ 100.0);
    limits.rate_limit_reached_type = Some(RateLimitReachedType::WorkspaceMemberUsageLimitReached);
    chat.on_rate_limit_snapshot(Some(limits));

    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "Usage limit reached.".to_string(),
    );
    chat.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_no_owner_nudge_or_rate_limit_refresh(&mut rx);

    chat.on_rate_limit_error(
        RateLimitErrorKind::UsageLimit,
        "Usage limit reached.".to_string(),
    );
    let popup = render_bottom_popup(&chat, /*width*/ 100);
    assert!(
        popup.contains("Request a limit increase from your owner"),
        "popup: {popup}"
    );
}

#[tokio::test]
async fn workspace_owner_credits_nudge_completion_renders_feedback() {
    let cases = [
        (
            Ok(AddCreditsNudgeEmailStatus::Sent),
            "Workspace owner notified.",
        ),
        (
            Ok(AddCreditsNudgeEmailStatus::CooldownActive),
            "Workspace owner was already notified recently.",
        ),
        (
            Err("request failed".to_string()),
            "Could not notify your workspace owner. Please try again.",
        ),
    ];

    let mut rendered_cases = Vec::new();
    for (result, expected) in cases {
        let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
        chat.start_add_credits_nudge_email_request(AddCreditsNudgeCreditType::Credits);
        chat.finish_add_credits_nudge_email_request(result);
        let rendered = drain_insert_history(&mut rx)
            .into_iter()
            .map(|lines| lines_to_single_string(&lines))
            .collect::<String>();
        assert!(rendered.contains(expected), "rendered: {rendered}");
        rendered_cases.push(rendered);
    }

    assert_chatwidget_snapshot!(
        "workspace_owner_credits_nudge_completion_feedback",
        rendered_cases.join("\n---\n")
    );
}

#[tokio::test]
async fn workspace_owner_usage_limit_nudge_completion_renders_feedback() {
    let cases = [
        (
            Ok(AddCreditsNudgeEmailStatus::Sent),
            "Limit increase requested.",
        ),
        (
            Ok(AddCreditsNudgeEmailStatus::CooldownActive),
            "A limit increase was already requested recently.",
        ),
        (
            Err("request failed".to_string()),
            "Could not request a limit increase. Please try again.",
        ),
    ];

    let mut rendered_cases = Vec::new();
    for (result, expected) in cases {
        let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
        chat.start_add_credits_nudge_email_request(AddCreditsNudgeCreditType::UsageLimit);
        chat.finish_add_credits_nudge_email_request(result);
        let rendered = drain_insert_history(&mut rx)
            .into_iter()
            .map(|lines| lines_to_single_string(&lines))
            .collect::<String>();
        assert!(rendered.contains(expected), "rendered: {rendered}");
        rendered_cases.push(rendered);
    }

    assert_chatwidget_snapshot!(
        "workspace_owner_usage_limit_nudge_completion_feedback",
        rendered_cases.join("\n---\n")
    );
}

fn next_send_add_credits_nudge_email_event(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
) -> AddCreditsNudgeCreditType {
    while let Ok(event) = rx.try_recv() {
        if let AppEvent::SendAddCreditsNudgeEmail { credit_type } = event {
            return credit_type;
        }
    }
    panic!("expected SendAddCreditsNudgeEmail app event");
}

fn assert_no_owner_nudge_or_rate_limit_refresh(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
) {
    while let Ok(event) = rx.try_recv() {
        assert!(
            !matches!(
                event,
                AppEvent::SendAddCreditsNudgeEmail { .. } | AppEvent::RefreshRateLimits { .. }
            ),
            "unexpected event: {event:?}"
        );
    }
}
