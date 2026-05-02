use super::*;
use chrono::Duration as ChronoDuration;
use chrono::Local;

#[tokio::test]
async fn renders_in_bottom_right_context() {
    let (mut chat, _rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
    let now = Local::now();
    chat.set_token_info(Some(make_token_info(
        /*total_tokens*/ 73_600, /*context_window*/ 100_000,
    )));
    chat.on_rate_limit_snapshot(Some(RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        primary: Some(RateLimitWindow {
            used_percent: 25.0,
            window_minutes: Some(300),
            resets_at: Some((now + ChronoDuration::minutes(150)).timestamp()),
        }),
        secondary: None,
        credits: None,
        plan_type: None,
        rate_limit_reached_type: None,
    }));

    assert_chatwidget_snapshot!(
        "session_limit_footer_right_status",
        render_bottom_popup(&chat, /*width*/ 80)
    );
}

#[tokio::test]
async fn combines_with_side_context_label() {
    let (mut chat, _rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
    let now = Local::now();
    chat.set_token_info(Some(make_token_info(
        /*total_tokens*/ 73_600, /*context_window*/ 100_000,
    )));
    chat.on_rate_limit_snapshot(Some(RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        primary: Some(RateLimitWindow {
            used_percent: 25.0,
            window_minutes: Some(300),
            resets_at: Some((now + ChronoDuration::minutes(150)).timestamp()),
        }),
        secondary: None,
        credits: None,
        plan_type: None,
        rate_limit_reached_type: None,
    }));
    chat.set_side_conversation_context_label(Some("Side from main thread".to_string()));

    assert_chatwidget_snapshot!(
        "session_limit_footer_with_side_context",
        render_bottom_popup(&chat, /*width*/ 110)
    );
}

#[tokio::test]
async fn compact_clear_removes_stale_token_percent() {
    let (mut chat, _rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
    let now = Local::now();
    chat.set_token_info(Some(make_token_info(
        /*total_tokens*/ 73_600, /*context_window*/ 100_000,
    )));
    chat.on_rate_limit_snapshot(Some(RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        primary: Some(RateLimitWindow {
            used_percent: 25.0,
            window_minutes: Some(300),
            resets_at: Some((now + ChronoDuration::minutes(150)).timestamp()),
        }),
        secondary: None,
        credits: None,
        plan_type: None,
        rate_limit_reached_type: None,
    }));

    let before = render_bottom_popup(&chat, /*width*/ 80);
    assert!(before.contains("70% tokens"), "before compact: {before}");
    assert!(before.contains("5h"), "before compact: {before}");

    chat.clear_token_usage();

    let after = render_bottom_popup(&chat, /*width*/ 80);
    assert!(!after.contains("tokens"), "after compact: {after}");
    assert!(after.contains("5h"), "after compact: {after}");
}
