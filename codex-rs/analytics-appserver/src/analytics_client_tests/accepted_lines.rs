use super::*;
use pretty_assertions::assert_eq;
use super::common::*;

#[tokio::test]
async fn reducer_emits_large_accepted_line_aggregates_without_fingerprints() {
    let mut reducer = AnalyticsReducer::default();
    let mut events = Vec::new();

    ingest_turn_prerequisites(
        &mut reducer,
        &mut events,
        /*include_initialize*/ true,
        /*include_resolved_config*/ true,
        /*include_started*/ true,
        /*include_token_usage*/ true,
    )
    .await;
    events.clear();

    let mut diff = "\
diff --git a/src/lib.rs b/src/lib.rs
index 1111111..2222222
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -0,0 +1,20000 @@
"
    .to_string();
    for index in 0..20_000 {
        diff.push_str(&format!("+let value_{index} = {index};\n"));
    }

    reducer
        .ingest(
            AnalyticsFact::Notification(Box::new(ServerNotification::TurnDiffUpdated(
                TurnDiffUpdatedNotification {
                    thread_id: "thread-2".to_string(),
                    turn_id: "turn-2".to_string(),
                    diff,
                },
            ))),
            &mut events,
        )
        .await;
    assert!(events.is_empty());

    reducer
        .ingest(
            AnalyticsFact::Notification(Box::new(sample_turn_completed_notification(
                "thread-2",
                "turn-2",
                AppServerTurnStatus::Completed,
                /*codex_error_info*/ None,
            ))),
            &mut events,
        )
        .await;

    let accepted_line_events = events
        .iter()
        .filter_map(|event| match event {
            TrackEventRequest::AcceptedLineFingerprints(event) => Some(event),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(accepted_line_events.len(), 1);
    let event = accepted_line_events[0];
    assert_eq!(event.event_params.turn_id, "turn-2");
    assert_eq!(event.event_params.thread_id, "thread-2");
    assert_eq!(event.event_params.accepted_added_lines, 20_000);
    assert_eq!(event.event_params.accepted_deleted_lines, 0);
    assert!(event.event_params.line_fingerprints.is_empty());
    assert!(serde_json::to_vec(event).expect("serialize event").len() < 2_100_000);
}

#[tokio::test]
async fn reducer_emits_accepted_line_fingerprints_once_from_latest_turn_diff_on_completion() {
    let mut reducer = AnalyticsReducer::default();
    let mut events = Vec::new();

    ingest_turn_prerequisites(
        &mut reducer,
        &mut events,
        /*include_initialize*/ true,
        /*include_resolved_config*/ true,
        /*include_started*/ true,
        /*include_token_usage*/ true,
    )
    .await;
    events.clear();

    for line in ["let old_value = 1;", "let latest_value = 2;"] {
        let diff = format!(
            "\
diff --git a/src/lib.rs b/src/lib.rs
index 1111111..2222222
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -0,0 +1 @@
+{line}
"
        );
        reducer
            .ingest(
                AnalyticsFact::Notification(Box::new(ServerNotification::TurnDiffUpdated(
                    TurnDiffUpdatedNotification {
                        thread_id: "thread-2".to_string(),
                        turn_id: "turn-2".to_string(),
                        diff,
                    },
                ))),
                &mut events,
            )
            .await;
    }
    assert!(events.is_empty());

    reducer
        .ingest(
            AnalyticsFact::Notification(Box::new(sample_turn_completed_notification(
                "thread-2",
                "turn-2",
                AppServerTurnStatus::Completed,
                /*codex_error_info*/ None,
            ))),
            &mut events,
        )
        .await;

    let accepted_line_events = events
        .iter()
        .filter_map(|event| match event {
            TrackEventRequest::AcceptedLineFingerprints(event) => Some(event),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(accepted_line_events.len(), 1);
    let event = accepted_line_events[0];
    assert_eq!(event.event_params.accepted_added_lines, 1);
    assert!(event.event_params.line_fingerprints.is_empty());
}
