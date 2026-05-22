use anyhow::Result;
use codex_core_test_runtime::responses::ev_apply_patch_custom_tool_call;
use codex_core_test_runtime::responses::ev_apply_patch_shell_command_call_via_heredoc;
use codex_core_test_runtime::responses::ev_assistant_message;
use codex_core_test_runtime::responses::ev_completed;
use codex_core_test_runtime::responses::ev_response_created;
use codex_core_test_runtime::responses::mount_sse_sequence;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::test_codex::ApplyPatchModelOutput;
use codex_core_test_runtime::test_codex::TestCodexBuilder;
use codex_core_test_runtime::test_codex::TestCodexHarness;
use codex_core_test_runtime::test_codex::test_codex;
use serde_json::Value;

pub(crate) async fn apply_patch_harness() -> Result<TestCodexHarness> {
    apply_patch_harness_with(|builder| builder).await
}

pub(crate) async fn apply_patch_harness_with(
    configure: impl FnOnce(TestCodexBuilder) -> TestCodexBuilder,
) -> Result<TestCodexHarness> {
    let builder = configure(test_codex()).with_config(|config| {
        config.include_apply_patch_tool = true;
    });
    // Box harness construction so apply_patch_cli tests do not inline the
    // full test-thread startup path into each test future.
    Box::pin(TestCodexHarness::with_remote_aware_builder(builder)).await
}

pub(crate) async fn mount_apply_patch(
    harness: &TestCodexHarness,
    call_id: &str,
    patch: &str,
    assistant_msg: &str,
    output_type: ApplyPatchModelOutput,
) {
    mount_sse_sequence(
        harness.server(),
        apply_patch_responses(call_id, patch, assistant_msg, output_type),
    )
    .await;
}

fn apply_patch_responses(
    call_id: &str,
    patch: &str,
    assistant_msg: &str,
    output_type: ApplyPatchModelOutput,
) -> Vec<String> {
    vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_apply_patch_call(call_id, patch, output_type),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_assistant_message("msg-1", assistant_msg),
            ev_completed("resp-2"),
        ]),
    ]
}

fn ev_apply_patch_call(call_id: &str, patch: &str, output_type: ApplyPatchModelOutput) -> Value {
    match output_type {
        ApplyPatchModelOutput::Freeform => ev_apply_patch_custom_tool_call(call_id, patch),
        ApplyPatchModelOutput::ShellCommandViaHeredoc => {
            ev_apply_patch_shell_command_call_via_heredoc(call_id, patch)
        }
    }
}
