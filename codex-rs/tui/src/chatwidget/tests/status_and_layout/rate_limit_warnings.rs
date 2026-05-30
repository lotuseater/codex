use super::super::*;
use crate::chatwidget::rate_limits::get_limits_duration;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn prefetch_rate_limits_is_gated_on_chatgpt_auth_provider() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    assert!(!chat.should_prefetch_rate_limits());

    set_chatgpt_auth(&mut chat);
    assert!(chat.should_prefetch_rate_limits());

    chat.config.model_provider.requires_openai_auth = false;
    assert!(!chat.should_prefetch_rate_limits());

    chat.prefetch_rate_limits();
    assert!(!chat.should_prefetch_rate_limits());
}

#[tokio::test]
async fn rate_limit_warnings_emit_thresholds() {
    let mut state = RateLimitWarningState::default();
    let mut warnings: Vec<String> = Vec::new();

    warnings.extend(state.take_warnings(Some(10.0), Some(10079), Some(55.0), Some(299)));
    warnings.extend(state.take_warnings(Some(55.0), Some(10081), Some(10.0), Some(299)));
    warnings.extend(state.take_warnings(Some(10.0), Some(10081), Some(80.0), Some(299)));
    warnings.extend(state.take_warnings(Some(80.0), Some(10081), Some(10.0), Some(299)));
    warnings.extend(state.take_warnings(Some(10.0), Some(10081), Some(95.0), Some(299)));
    warnings.extend(state.take_warnings(Some(95.0), Some(10079), Some(10.0), Some(299)));

    assert_eq!(
        warnings,
        vec![
            String::from(
                "Heads up, you have less than 25% of your 5h limit left. Run /status for a breakdown."
            ),
            String::from(
                "Heads up, you have less than 25% of your weekly limit left. Run /status for a breakdown.",
            ),
            String::from(
                "Heads up, you have less than 5% of your 5h limit left. Run /status for a breakdown."
            ),
            String::from(
                "Heads up, you have less than 5% of your weekly limit left. Run /status for a breakdown.",
            ),
        ],
        "expected one warning per limit for the highest crossed threshold"
    );
}

#[tokio::test]
async fn test_rate_limit_warnings_monthly() {
    let mut state = RateLimitWarningState::default();
    let mut warnings: Vec<String> = Vec::new();

    warnings.extend(state.take_warnings(
        Some(75.0),
        Some(43199),
        /*primary_used_percent*/ None,
        /*primary_window_minutes*/ None,
    ));
    assert_eq!(
        warnings,
        vec![String::from(
            "Heads up, you have less than 25% of your monthly limit left. Run /status for a breakdown.",
        ),],
        "expected one warning per limit for the highest crossed threshold"
    );
}

#[test]
fn rate_limit_duration_labels_only_render_supported_windows() {
    assert_eq!(get_limits_duration(2 * 60), None);
    assert_eq!(get_limits_duration(24 * 60).as_deref(), Some("daily"));
    assert_eq!(
        get_limits_duration(365 * 24 * 60).as_deref(),
        Some("annual")
    );
}

#[tokio::test]
async fn test_rate_limit_warnings_use_generic_fallback_labels() {
    let mut state = RateLimitWarningState::default();

    assert_eq!(
        state.take_warnings(
            /*secondary_used_percent*/ Some(75.0),
            /*secondary_window_minutes*/ None,
            /*primary_used_percent*/ Some(75.0),
            /*primary_window_minutes*/ None,
        ),
        vec![
            String::from(
                "Heads up, you have less than 25% of your secondary usage limit left. Run /status for a breakdown.",
            ),
            String::from(
                "Heads up, you have less than 25% of your usage limit left. Run /status for a breakdown.",
            ),
        ],
    );
}

#[tokio::test]
async fn test_rate_limit_warnings_use_secondary_fallback_for_unsupported_window() {
    let mut state = RateLimitWarningState::default();

    assert_eq!(
        state.take_warnings(
            /*secondary_used_percent*/ Some(75.0),
            /*secondary_window_minutes*/ Some(2 * 60),
            /*primary_used_percent*/ None,
            /*primary_window_minutes*/ None,
        ),
        vec![String::from(
            "Heads up, you have less than 25% of your secondary usage limit left. Run /status for a breakdown.",
        )],
    );
}

#[tokio::test]
async fn status_line_uses_secondary_fallback_for_unsupported_window() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.on_rate_limit_snapshot(Some(RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        primary: None,
        secondary: Some(RateLimitWindow {
            used_percent: 50,
            window_duration_mins: Some(2 * 60),
            resets_at: None,
        }),
        credits: None,
        plan_type: None,
        rate_limit_reached_type: None,
    }));

    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::WeeklyLimit),
        Some("secondary usage 50% left".to_string())
    );
}

#[tokio::test]
async fn status_line_legacy_limit_items_prefer_matching_windows() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.on_rate_limit_snapshot(Some(RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        primary: Some(RateLimitWindow {
            used_percent: 94,
            window_duration_mins: Some(7 * 24 * 60),
            resets_at: None,
        }),
        secondary: Some(RateLimitWindow {
            used_percent: 40,
            window_duration_mins: Some(5 * 60),
            resets_at: None,
        }),
        credits: None,
        plan_type: None,
        rate_limit_reached_type: None,
    }));

    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::FiveHourLimit),
        Some("5h 60% left".to_string())
    );
    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::WeeklyLimit),
        Some("weekly 6% left".to_string())
    );
}

#[tokio::test]
async fn status_line_shows_secondary_non_weekly_when_primary_is_weekly() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.on_rate_limit_snapshot(Some(RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        primary: Some(RateLimitWindow {
            used_percent: 94,
            window_duration_mins: Some(7 * 24 * 60),
            resets_at: None,
        }),
        secondary: Some(RateLimitWindow {
            used_percent: 35,
            window_duration_mins: Some(30 * 24 * 60),
            resets_at: None,
        }),
        credits: None,
        plan_type: None,
        rate_limit_reached_type: None,
    }));

    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::FiveHourLimit),
        Some("monthly 65% left".to_string())
    );
    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::WeeklyLimit),
        Some("weekly 6% left".to_string())
    );
}

#[tokio::test]
async fn status_line_five_hour_item_omits_weekly_only_limit() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.on_rate_limit_snapshot(Some(RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        primary: Some(RateLimitWindow {
            used_percent: 9,
            window_duration_mins: Some(7 * 24 * 60),
            resets_at: None,
        }),
        secondary: None,
        credits: None,
        plan_type: None,
        rate_limit_reached_type: None,
    }));

    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::FiveHourLimit),
        None
    );
    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::WeeklyLimit),
        Some("weekly 91% left".to_string())
    );
}

#[tokio::test]
async fn status_line_single_monthly_primary_omits_weekly_limit_item() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.on_rate_limit_snapshot(Some(RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        primary: Some(RateLimitWindow {
            used_percent: 35,
            window_duration_mins: Some(30 * 24 * 60),
            resets_at: None,
        }),
        secondary: None,
        credits: None,
        plan_type: None,
        rate_limit_reached_type: None,
    }));

    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::FiveHourLimit),
        Some("monthly 65% left".to_string())
    );
    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::WeeklyLimit),
        None
    );
}

#[tokio::test]
async fn status_line_secondary_only_non_weekly_limit_omits_primary_limit_item() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.on_rate_limit_snapshot(Some(RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        primary: None,
        secondary: Some(RateLimitWindow {
            used_percent: 35,
            window_duration_mins: Some(30 * 24 * 60),
            resets_at: None,
        }),
        credits: None,
        plan_type: None,
        rate_limit_reached_type: None,
    }));

    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::FiveHourLimit),
        None
    );
    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::WeeklyLimit),
        Some("monthly 65% left".to_string())
    );
}
