use super::super::*;
use pretty_assertions::assert_eq;

/// Receiving a token usage update without usage clears the context indicator.
#[tokio::test]
async fn token_count_none_resets_context_indicator() {
    let (mut chat, _rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;

    let context_window = 13_000;
    let pre_compact_tokens = 12_700;

    handle_token_count(
        &mut chat,
        Some(make_token_info(pre_compact_tokens, context_window)),
    );
    assert_eq!(chat.bottom_pane.context_window_percent(), Some(30));

    handle_token_count(&mut chat, /*info*/ None);
    assert_eq!(chat.bottom_pane.context_window_percent(), None);
}

#[tokio::test]
async fn app_server_cyber_policy_error_renders_dedicated_notice() {
    let (mut chat, mut rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;

    handle_error(
        &mut chat,
        "server fallback message",
        Some(CodexErrorInfo::CyberPolicy),
    );

    let cells = drain_insert_history(&mut rx);
    assert_eq!(cells.len(), 1);
    let rendered = lines_to_single_string(&cells[0]);
    assert!(rendered.contains("This chat was flagged for possible cybersecurity risk"));
    assert!(rendered.contains("Trusted Access for Cyber"));
    assert!(!rendered.contains("server fallback message"));
}

#[tokio::test]
async fn app_server_model_verification_renders_warning() {
    let (mut chat, mut rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;

    handle_model_verification(
        &mut chat,
        vec![AppServerModelVerification::TrustedAccessForCyber],
    );

    let cells = drain_insert_history(&mut rx);
    assert_eq!(cells.len(), 1);
    let rendered = lines_to_single_string(&cells[0]);
    assert!(rendered.contains("multiple flags for possible cybersecurity risk"));
    assert!(rendered.contains("extra safety checks are on"));
    assert!(rendered.contains("Trusted Access for Cyber"));
    assert!(rendered.contains("https://chatgpt.com/cyber"));
}

#[tokio::test]
async fn context_indicator_shows_used_tokens_when_window_unknown() {
    let (mut chat, _rx, _ops) = make_chatwidget_manual(Some("unknown-model")).await;

    chat.config.model_context_window = None;
    let auto_compact_limit = 200_000;
    chat.config.model_auto_compact_token_limit = Some(auto_compact_limit);

    // No model window, so the indicator should fall back to showing tokens used.
    let total_tokens = 106_000;
    let token_usage = TokenUsage {
        total_tokens,
        ..TokenUsage::default()
    };
    let token_info = TokenUsageInfo {
        total_token_usage: token_usage.clone(),
        last_token_usage: token_usage,
        model_context_window: None,
    };

    handle_token_count(&mut chat, Some(token_info));

    assert_eq!(chat.bottom_pane.context_window_percent(), None);
    assert_eq!(
        chat.bottom_pane.context_window_used_tokens(),
        Some(total_tokens)
    );
}

#[tokio::test]
async fn token_usage_update_uses_runtime_context_window() {
    let (mut chat, mut rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.config.model_context_window = Some(1_000_000);

    handle_token_count(
        &mut chat,
        Some(make_token_info(
            /*total_tokens*/ 0, /*context_window*/ 950_000,
        )),
    );

    assert_eq!(
        chat.status_line_value_for_item(crate::bottom_pane::StatusLineItem::ContextWindowSize),
        Some("950K window".to_string())
    );
    assert_eq!(chat.bottom_pane.context_window_percent(), Some(100));

    chat.add_status_output(
        /*refreshing_rate_limits*/ false, /*request_id*/ None,
    );

    let cells = drain_insert_history(&mut rx);
    let context_line = cells
        .last()
        .expect("status output inserted")
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .find(|line| line.contains("Context window"))
        .expect("context window line");

    assert!(
        context_line.contains("950K"),
        "expected /status to use runtime context window, got: {context_line}"
    );
    assert!(
        !context_line.contains("1M"),
        "expected /status to avoid raw config context window, got: {context_line}"
    );
}
