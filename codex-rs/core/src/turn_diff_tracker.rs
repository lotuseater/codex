use std::collections::HashMap;
use std::path::PathBuf;

use codex_apply_patch::AppliedPatchDelta;
use codex_apply_patch::AppliedPatchFileChange;
use codex_protocol::protocol::FileChange;
pub use codex_turn_diff::CommittedFileChange;
pub use codex_turn_diff::TrackedFileChange;
pub use codex_turn_diff::TurnDiffTracker;

// fork-local: turn-diff tracking lives in the `codex-turn-diff` crate; this module keeps only
// the thin protocol<->crate adapters that delegate to it. Upstream's in-core monolith (the
// re-introduced `TurnDiffTracker`/`TrackedPath` implementation, including #26433 multi-env
// logic) belongs in that crate, not here.
pub(crate) fn tracked_file_changes_from_protocol(
    changes: &HashMap<PathBuf, FileChange>,
) -> HashMap<PathBuf, TrackedFileChange> {
    changes
        .iter()
        .map(|(path, change)| {
            let change = match change {
                FileChange::Add { content } => TrackedFileChange::Add {
                    content: content.clone(),
                },
                FileChange::Delete { content } => TrackedFileChange::Delete {
                    content: content.clone(),
                },
                FileChange::Update { move_path, .. } => TrackedFileChange::Update {
                    move_path: move_path.clone(),
                },
            };
            (path.clone(), change)
        })
        .collect()
}

pub(crate) fn committed_file_changes_from_apply_patch_delta(
    delta: &AppliedPatchDelta,
) -> Vec<CommittedFileChange> {
    delta
        .changes()
        .iter()
        .map(|change| {
            let path = change.path.clone();
            match &change.change {
                AppliedPatchFileChange::Add {
                    overwritten_content,
                    ..
                } => CommittedFileChange::Add {
                    path,
                    overwritten_content: overwritten_content.clone(),
                },
                AppliedPatchFileChange::Delete { content } => CommittedFileChange::Delete {
                    path,
                    content: content.clone(),
                },
                AppliedPatchFileChange::Update {
                    move_path,
                    old_content,
                    overwritten_move_content,
                    ..
                } => CommittedFileChange::Update {
                    path,
                    move_path: move_path.clone(),
                    old_content: old_content.clone(),
                    overwritten_move_content: overwritten_move_content.clone(),
                },
            }
        })
        .collect()
}
