pub use codex_test_support_context_fixtures::context_snapshot::ContextSnapshotOptions;
pub use codex_test_support_context_fixtures::context_snapshot::ContextSnapshotRenderMode;
pub use codex_test_support_context_fixtures::context_snapshot::format_labeled_items_snapshot;
pub use codex_test_support_context_fixtures::context_snapshot::format_response_items_snapshot;

use serde_json::Value;

use crate::responses::ResponsesRequest;

pub fn format_request_input_snapshot(
    request: &ResponsesRequest,
    options: &ContextSnapshotOptions,
) -> String {
    let items = request.input();
    codex_test_support_context_fixtures::context_snapshot::format_request_input_snapshot(
        items.as_slice(),
        options,
    )
}

pub fn format_labeled_requests_snapshot(
    scenario: &str,
    sections: &[(&str, &ResponsesRequest)],
    options: &ContextSnapshotOptions,
) -> String {
    let request_inputs = sections
        .iter()
        .map(|(title, request)| (*title, request.input()))
        .collect::<Vec<(&str, Vec<Value>)>>();
    let borrowed_inputs = request_inputs
        .iter()
        .map(|(title, input)| (*title, input.as_slice()))
        .collect::<Vec<(&str, &[Value])>>();

    codex_test_support_context_fixtures::context_snapshot::format_labeled_requests_snapshot(
        scenario,
        borrowed_inputs.as_slice(),
        options,
    )
}

pub fn format_request_body_diff_snapshot(
    scenario: &str,
    before_title: &str,
    before_request: &ResponsesRequest,
    after_title: &str,
    after_request: &ResponsesRequest,
    options: &ContextSnapshotOptions,
) -> String {
    codex_test_support_context_fixtures::context_snapshot::format_request_body_diff_snapshot(
        scenario,
        before_title,
        &before_request.body_json(),
        after_title,
        &after_request.body_json(),
        options,
    )
}
