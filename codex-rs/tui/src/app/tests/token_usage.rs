use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn token_usage_update_refreshes_status_line_with_runtime_context_window() {
    let mut app = make_test_app().await;
    app.chat_widget.setup_status_line(
        vec![crate::bottom_pane::StatusLineItem::ContextWindowSize],
        /*use_theme_colors*/ true,
    );

    assert_eq!(app.chat_widget.status_line_text(), None);

    app.handle_thread_event_now(ThreadBufferedEvent::Notification(token_usage_notification(
        ThreadId::new(),
        "turn-1",
        Some(950_000),
    )));

    assert_eq!(
        app.chat_widget.status_line_text(),
        Some("950K window".into())
    );
}

#[tokio::test]
async fn token_usage_update_tracks_agent_current_context_not_cumulative_total() {
    let mut app = make_test_app().await;
    let agent_thread_id = ThreadId::new();
    app.agent_navigation.upsert(
        agent_thread_id,
        Some("Epicurus".to_string()),
        Some("explorer".to_string()),
        /*is_closed*/ false,
    );

    app.handle_thread_event_now(ThreadBufferedEvent::Notification(
        token_usage_notification_with_totals(
            agent_thread_id,
            "turn-1",
            796_051,
            8_213,
            Some(258_400),
        ),
    ));

    assert_eq!(
        app.agent_navigation
            .get(&agent_thread_id)
            .map(|entry| entry.token_context_percent_used),
        Some(Some(3))
    );
}
