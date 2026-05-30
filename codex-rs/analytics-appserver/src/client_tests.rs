//! Tests for the app-server analytics request-batching behavior.
//!
//! Historically this module also exercised `AnalyticsEventsClient::track_request`
//! / `track_response` enqueue filtering by constructing the (then lower-crate)
//! `AnalyticsEventsClient`/`AnalyticsEventsQueue` by field. After the analytics
//! crate split:
//!   * `track_request`/`track_response` live on the
//!     [`crate::client_ext::AppServerAnalyticsExt`] trait and emit the opaque
//!     `codex_analytics::AnalyticsFact::AppServer` payload, and
//!   * `AnalyticsEventsClient`/`AnalyticsEventsQueue` are owned by
//!     `codex-analytics` with private fields, so they can no longer be built by
//!     field from this crate.
//!
//! The enqueue-filtering behavior those tests covered is now exercised at the
//! reducer layer (see `analytics_client_tests::reducer_lifecycle`'s
//! "unrelated_client_requests/responses_are_ignored_by_reducer"). What remains
//! crate-local here is the request-isolation batching rule, which is a property
//! of [`crate::events::TrackEventRequest`].

use crate::events::CodexAcceptedLineFingerprintsEventParams;
use crate::events::CodexAcceptedLineFingerprintsEventRequest;
use crate::events::InvocationType;
use crate::events::SkillInvocationEventParams;
use crate::events::SkillInvocationEventRequest;
use crate::events::TrackEventRequest;

fn sample_accepted_line_fingerprint_event(thread_id: &str) -> TrackEventRequest {
    TrackEventRequest::AcceptedLineFingerprints(Box::new(
        CodexAcceptedLineFingerprintsEventRequest {
            event_type: "codex_accepted_line_fingerprints",
            event_params: CodexAcceptedLineFingerprintsEventParams {
                event_type: "codex.accepted_line_fingerprints",
                turn_id: "turn-1".to_string(),
                thread_id: thread_id.to_string(),
                product_surface: Some("codex".to_string()),
                model_slug: Some("gpt-5.1-codex".to_string()),
                completed_at: 1,
                repo_hash: None,
                accepted_added_lines: 1,
                accepted_deleted_lines: 0,
                line_fingerprints: Vec::new(),
            },
        },
    ))
}

fn sample_regular_track_event(thread_id: &str) -> TrackEventRequest {
    TrackEventRequest::SkillInvocation(SkillInvocationEventRequest {
        event_type: "skill_invocation",
        skill_id: format!("skill-{thread_id}"),
        skill_name: "doc".to_string(),
        event_params: SkillInvocationEventParams {
            product_client_id: None,
            skill_scope: None,
            plugin_id: None,
            repo_url: None,
            thread_id: Some(thread_id.to_string()),
            turn_id: Some("turn-1".to_string()),
            invoke_type: Some(InvocationType::Explicit),
            model_slug: Some("gpt-5.1-codex".to_string()),
        },
    })
}

#[test]
fn only_accepted_line_fingerprint_events_are_sent_in_isolated_requests() {
    assert!(
        sample_accepted_line_fingerprint_event("thread-1").should_send_in_isolated_request(),
        "accepted-line-fingerprint events must be isolated"
    );
    assert!(
        !sample_regular_track_event("thread-1").should_send_in_isolated_request(),
        "regular events must batch together"
    );
}
