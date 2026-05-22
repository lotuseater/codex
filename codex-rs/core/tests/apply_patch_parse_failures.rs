#![allow(clippy::expect_used)]

mod support;

#[path = "suite/apply_patch_harness.rs"]
mod apply_patch_harness;

use anyhow::Result;
use apply_patch_harness::apply_patch_harness;
use apply_patch_harness::mount_apply_patch;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::test_codex::ApplyPatchModelOutput;
use pretty_assertions::assert_eq;
use test_case::test_case;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_rejects_invalid_hunk_header(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    let patch = "*** Begin Patch\n*** Frobnicate File: foo\n*** End Patch";
    let call_id = "apply-invalid-header";
    mount_apply_patch(&harness, call_id, patch, "ok", model_output).await;

    harness.submit("apply invalid header patch").await?;

    let out = harness.apply_patch_output(call_id, model_output).await;

    assert!(
        out.contains("apply_patch verification failed"),
        "expected verification failure message"
    );
    assert!(
        out.contains("is not a valid hunk header"),
        "expected parse diagnostics in output: {out:?}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_reports_missing_context(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    harness.write_file("modify.txt", "line1\nline2\n").await?;

    let patch =
        "*** Begin Patch\n*** Update File: modify.txt\n@@\n-missing\n+changed\n*** End Patch";
    let call_id = "apply-missing-context";
    mount_apply_patch(&harness, call_id, patch, "ok", model_output).await;

    harness.submit("apply missing context patch").await?;

    let out = harness.apply_patch_output(call_id, model_output).await;

    assert!(
        out.contains("apply_patch verification failed"),
        "expected verification failure message"
    );
    assert!(out.contains("Failed to find expected lines in"));
    assert_eq!(
        harness.read_file_text("modify.txt").await?,
        "line1\nline2\n"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_reports_missing_target_file(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    let patch = "*** Begin Patch\n*** Update File: missing.txt\n@@\n-nope\n+better\n*** End Patch";
    let call_id = "apply-missing-file";
    mount_apply_patch(&harness, call_id, patch, "fail", model_output).await;

    harness.submit("attempt to update a missing file").await?;

    let out = harness.apply_patch_output(call_id, model_output).await;
    assert!(
        out.contains("apply_patch verification failed"),
        "expected verification failure message"
    );
    assert!(
        out.contains("Failed to read file to update"),
        "expected missing file diagnostics: {out}"
    );
    assert!(
        out.contains("missing.txt"),
        "expected missing file path in diagnostics: {out}"
    );
    assert!(!harness.path_exists("missing.txt").await?);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_delete_missing_file_reports_error(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    let patch = "*** Begin Patch\n*** Delete File: missing.txt\n*** End Patch";
    let call_id = "apply-delete-missing";
    mount_apply_patch(&harness, call_id, patch, "fail", model_output).await;

    harness.submit("attempt to delete missing file").await?;

    let out = harness.apply_patch_output(call_id, model_output).await;

    assert!(
        out.contains("apply_patch verification failed"),
        "expected verification failure message: {out}"
    );
    assert!(
        out.contains("Failed to read"),
        "missing delete diagnostics should mention read failure: {out}"
    );
    assert!(
        out.contains("missing.txt"),
        "missing delete diagnostics should surface target path: {out}"
    );
    assert!(!harness.path_exists("missing.txt").await?);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_rejects_empty_patch(model_output: ApplyPatchModelOutput) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    let patch = "*** Begin Patch\n*** End Patch";
    let call_id = "apply-empty";
    mount_apply_patch(&harness, call_id, patch, "ok", model_output).await;

    harness.submit("apply empty patch").await?;

    let out = harness.apply_patch_output(call_id, model_output).await;
    assert!(
        out.contains("patch rejected: empty patch"),
        "expected rejection for empty patch: {out}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::Freeform)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_cli_missing_second_chunk_context_rejected(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    harness.write_file("two_chunks.txt", "a\nb\nc\nd\n").await?;

    // First chunk has @@, second chunk intentionally omits @@ to trigger parse error.
    let patch =
        "*** Begin Patch\n*** Update File: two_chunks.txt\n@@\n-b\n+B\n\n-d\n+D\n*** End Patch";
    let call_id = "apply-missing-ctx-2nd";
    mount_apply_patch(&harness, call_id, patch, "fail", model_output).await;

    harness.submit("apply missing context second chunk").await?;

    let out = harness.apply_patch_output(call_id, model_output).await;
    assert!(out.contains("apply_patch verification failed"));
    assert!(
        out.contains("Failed to find expected lines in"),
        "expected hunk context diagnostics: {out}"
    );
    // Original file unchanged on failure
    assert_eq!(
        harness.read_file_text("two_chunks.txt").await?,
        "a\nb\nc\nd\n"
    );
    Ok(())
}
