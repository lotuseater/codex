//! Regression coverage for resuming sessions recorded before the
//! `SessionId`/`ThreadId` identity split. Those older rollouts wrote a
//! `session_meta` line whose `payload` carries only `id` (no `session_id`).
//! The reader must still deserialize that first line so `codex --resume` works
//! on pre-split sessions instead of failing with the misleading
//! "does not start with session metadata".
//!
//! Lives in `tests/` (fork-owned, separate compilation unit) so it survives
//! upstream convergence of `src/protocol/rollout.rs`.

use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::RolloutLine;

/// A first rollout line as written by older builds. Mirrors the real on-disk
/// payload key set, with `session_id` deliberately absent.
const LEGACY_SESSION_META_LINE: &str = r#"{"timestamp":"2026-06-21T11:30:15.941Z","type":"session_meta","payload":{"id":"019ee9f0-6650-7083-abdc-01c794741ed5","timestamp":"2026-06-21T11:28:26.823Z","cwd":"C:\\Users\\Oleh\\Documents\\GitHub\\DonutGame","originator":"codex-tui","cli_version":"0.0.0","source":"cli","thread_source":"user","model_provider":"openai","base_instructions":{"text":"You are Codex."},"git":{"commit_hash":"0000000000000000000000000000000000000000","branch":"main","repository_url":"https://example.invalid/repo.git"}}}"#;

#[test]
fn legacy_rollout_without_session_id_is_resumable() {
    let line: RolloutLine = serde_json::from_str(LEGACY_SESSION_META_LINE)
        .expect("legacy session_meta line (no session_id) must still deserialize");
    let RolloutItem::SessionMeta(meta) = line.item else {
        panic!("first rollout item must be session metadata");
    };
    // The thread identity must round-trip; `session_id` is defaulted (absent).
    assert_eq!(
        meta.meta.id.to_string(),
        "019ee9f0-6650-7083-abdc-01c794741ed5"
    );
}
