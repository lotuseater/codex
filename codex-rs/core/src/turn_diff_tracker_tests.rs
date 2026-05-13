use super::*;
use codex_apply_patch::AppliedPatchDelta;
use codex_apply_patch::ApplyPatchAction;
use codex_apply_patch::ApplyPatchFileChange;
use codex_apply_patch::MaybeApplyPatchVerified;
use codex_exec_server::LOCAL_FS;
use codex_protocol::protocol::FileChange;
use codex_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tempfile::tempdir;

const REGULAR_FILE_MODE: &str = "100644";

fn git_blob_sha1_hex(data: &str) -> String {
    git_blob_sha1_hex_bytes(data.as_bytes())
}

fn protocol_changes(action: &ApplyPatchAction) -> HashMap<PathBuf, FileChange> {
    action
        .changes()
        .iter()
        .map(|(path, change)| {
            let change = match change {
                ApplyPatchFileChange::Add { content } => FileChange::Add {
                    content: content.clone(),
                },
                ApplyPatchFileChange::Delete { content } => FileChange::Delete {
                    content: content.clone(),
                },
                ApplyPatchFileChange::Update {
                    unified_diff,
                    move_path,
                    ..
                } => FileChange::Update {
                    unified_diff: unified_diff.clone(),
                    move_path: move_path.clone(),
                },
            };
            (path.clone(), change)
        })
        .collect()
}

fn get_unified_diff(tracker: &mut TurnDiffTracker) -> Option<String> {
    tracker.get_unified_diff().expect("diff should render")
}

fn normalize_diff_for_test(diff: &str, root: &Path) -> String {
    diff.replace(&root.display().to_string().replace('\\', "/"), "<TMP>")
}

async fn apply_verified_patch(
    tracker: &mut TurnDiffTracker,
    root: &Path,
    patch: &str,
) -> AppliedPatchDelta {
    let cwd = AbsolutePathBuf::from_absolute_path(root).expect("absolute tempdir path");
    let argv = vec!["apply_patch".to_string(), patch.to_string()];
    let action = match codex_apply_patch::maybe_parse_apply_patch_verified(
        &argv,
        &cwd,
        LOCAL_FS.as_ref(),
        /*sandbox*/ None,
    )
    .await
    {
        MaybeApplyPatchVerified::Body(action) => action,
        other => panic!("expected verified patch action, got {other:?}"),
    };
    tracker.on_patch_begin(&protocol_changes(&action));

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    codex_apply_patch::apply_patch(
        patch,
        &cwd,
        &mut stdout,
        &mut stderr,
        LOCAL_FS.as_ref(),
        /*sandbox*/ None,
    )
    .await
    .expect("patch should apply")
}

#[tokio::test]
async fn accumulates_add_then_update_as_single_add() {
    let dir = tempdir().expect("tempdir");
    let mut tracker = TurnDiffTracker::with_display_root(dir.path().to_path_buf());

    let add = apply_verified_patch(
        &mut tracker,
        dir.path(),
        "*** Begin Patch\n*** Add File: a.txt\n+foo\n*** End Patch",
    )
    .await;
    tracker.track_delta(&add);

    let update = apply_verified_patch(
        &mut tracker,
        dir.path(),
        "*** Begin Patch\n*** Update File: a.txt\n@@\n foo\n+bar\n*** End Patch",
    )
    .await;
    tracker.track_delta(&update);

    let right_oid = git_blob_sha1_hex("foo\nbar\n");
    let expected = format!(
        r#"diff --git a/a.txt b/a.txt
new file mode {REGULAR_FILE_MODE}
index {ZERO_OID}..{right_oid}
--- {DEV_NULL}
+++ b/a.txt
@@ -0,0 +1,2 @@
+foo
+bar
"#,
    );
    assert_eq!(get_unified_diff(&mut tracker), Some(expected));
}

#[tokio::test]
async fn invalidated_tracker_suppresses_existing_diff() {
    let dir = tempdir().expect("tempdir");
    let mut tracker = TurnDiffTracker::with_display_root(dir.path().to_path_buf());

    let add = apply_verified_patch(
        &mut tracker,
        dir.path(),
        "*** Begin Patch\n*** Add File: a.txt\n+foo\n*** End Patch",
    )
    .await;
    tracker.track_delta(&add);

    tracker.invalidate();

    assert_eq!(get_unified_diff(&mut tracker), None);
}

#[tokio::test]
async fn accumulates_delete() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("b.txt"), "x\n").expect("seed file");

    let mut tracker = TurnDiffTracker::with_display_root(dir.path().to_path_buf());
    let delete = apply_verified_patch(
        &mut tracker,
        dir.path(),
        "*** Begin Patch\n*** Delete File: b.txt\n*** End Patch",
    )
    .await;
    tracker.track_delta(&delete);

    let left_oid = git_blob_sha1_hex("x\n");
    let expected = format!(
        r#"diff --git a/b.txt b/b.txt
deleted file mode {REGULAR_FILE_MODE}
index {left_oid}..{ZERO_OID}
--- a/b.txt
+++ {DEV_NULL}
@@ -1 +0,0 @@
-x
"#,
    );
    assert_eq!(get_unified_diff(&mut tracker), Some(expected));
}

#[tokio::test]
async fn accumulates_move_and_update() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("src.txt"), "line\n").expect("seed file");

    let mut tracker = TurnDiffTracker::with_display_root(dir.path().to_path_buf());
    let update = apply_verified_patch(
        &mut tracker,
        dir.path(),
        "*** Begin Patch\n*** Update File: src.txt\n*** Move to: dst.txt\n@@\n-line\n+line2\n*** End Patch",
    )
    .await;
    tracker.track_delta(&update);

    let left_oid = git_blob_sha1_hex("line\n");
    let right_oid = git_blob_sha1_hex("line2\n");
    let expected = format!(
        r#"diff --git a/src.txt b/dst.txt
index {left_oid}..{right_oid}
--- a/src.txt
+++ b/dst.txt
@@ -1 +1 @@
-line
+line2
"#,
    );
    assert_eq!(get_unified_diff(&mut tracker), Some(expected));
}

#[tokio::test]
async fn pure_rename_yields_no_diff() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("old.txt"), "same\n").expect("seed file");

    let mut tracker = TurnDiffTracker::with_display_root(dir.path().to_path_buf());
    let rename = apply_verified_patch(
        &mut tracker,
        dir.path(),
        "*** Begin Patch\n*** Update File: old.txt\n*** Move to: new.txt\n@@\n same\n*** End Patch",
    )
    .await;
    tracker.track_delta(&rename);

    assert_eq!(get_unified_diff(&mut tracker), None);
}

#[tokio::test]
async fn add_over_existing_file_becomes_update() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("dup.txt"), "before\n").expect("seed file");

    let mut tracker = TurnDiffTracker::with_display_root(dir.path().to_path_buf());
    let add = apply_verified_patch(
        &mut tracker,
        dir.path(),
        "*** Begin Patch\n*** Add File: dup.txt\n+after\n*** End Patch",
    )
    .await;
    tracker.track_delta(&add);

    let left_oid = git_blob_sha1_hex("before\n");
    let right_oid = git_blob_sha1_hex("after\n");
    let expected = format!(
        r#"diff --git a/dup.txt b/dup.txt
index {left_oid}..{right_oid}
--- a/dup.txt
+++ b/dup.txt
@@ -1 +1 @@
-before
+after
"#,
    );
    assert_eq!(get_unified_diff(&mut tracker), Some(expected));
}

#[tokio::test]
async fn non_git_display_root_keeps_diff_paths_relative() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("file.txt"), "before\n").expect("seed file");

    let mut tracker = TurnDiffTracker::with_display_root(dir.path().to_path_buf());
    let update = apply_verified_patch(
        &mut tracker,
        dir.path(),
        "*** Begin Patch\n*** Update File: file.txt\n@@\n-before\n+after\n*** End Patch",
    )
    .await;
    tracker.track_delta(&update);

    let diff = get_unified_diff(&mut tracker).expect("diff should render");
    assert!(!diff.contains(&dir.path().display().to_string().replace('\\', "/")));
    assert!(diff.contains("diff --git a/file.txt b/file.txt"));
}

#[tokio::test]
async fn delete_then_readd_same_path_becomes_update() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("cycle.txt"), "before\n").expect("seed file");

    let mut tracker = TurnDiffTracker::with_display_root(dir.path().to_path_buf());
    let delete = apply_verified_patch(
        &mut tracker,
        dir.path(),
        "*** Begin Patch\n*** Delete File: cycle.txt\n*** End Patch",
    )
    .await;
    tracker.track_delta(&delete);

    let add = apply_verified_patch(
        &mut tracker,
        dir.path(),
        "*** Begin Patch\n*** Add File: cycle.txt\n+after\n*** End Patch",
    )
    .await;
    tracker.track_delta(&add);

    let left_oid = git_blob_sha1_hex("before\n");
    let right_oid = git_blob_sha1_hex("after\n");
    let expected = format!(
        r#"diff --git a/cycle.txt b/cycle.txt
index {left_oid}..{right_oid}
--- a/cycle.txt
+++ b/cycle.txt
@@ -1 +1 @@
-before
+after
"#,
    );
    assert_eq!(get_unified_diff(&mut tracker), Some(expected));
}

#[tokio::test]
async fn move_over_existing_destination_without_content_change_deletes_source_only() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), "same\n").expect("seed source");
    fs::write(dir.path().join("b.txt"), "same\n").expect("seed destination");

    let mut tracker = TurnDiffTracker::with_display_root(dir.path().to_path_buf());
    let move_overwrite = apply_verified_patch(
        &mut tracker,
        dir.path(),
        "*** Begin Patch\n*** Update File: a.txt\n*** Move to: b.txt\n@@\n same\n*** End Patch",
    )
    .await;
    tracker.track_delta(&move_overwrite);

    let left_oid = git_blob_sha1_hex("same\n");
    let expected = format!(
        r#"diff --git a/a.txt b/a.txt
deleted file mode {REGULAR_FILE_MODE}
index {left_oid}..{ZERO_OID}
--- a/a.txt
+++ {DEV_NULL}
@@ -1 +0,0 @@
-same
"#,
    );
    assert_eq!(get_unified_diff(&mut tracker), Some(expected));
}

#[test]
fn binary_files_differ_update() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("bin.dat");

    // Initial non-UTF8 bytes
    let left_bytes: Vec<u8> = vec![0xff, 0xfe, 0xfd, 0x00];
    // Updated non-UTF8 bytes
    let right_bytes: Vec<u8> = vec![0x01, 0x02, 0x03, 0x00];

    fs::write(&file, &left_bytes).unwrap();

    let mut acc = TurnDiffTracker::new();
    let update_changes = HashMap::from([(
        file.clone(),
        FileChange::Update {
            unified_diff: "".to_owned(),
            move_path: None,
        },
    )]);
    acc.on_patch_begin(&update_changes);

    // Apply update on disk
    fs::write(&file, &right_bytes).unwrap();

    let diff = acc.get_unified_diff().unwrap().unwrap();
    let diff = normalize_diff_for_test(&diff, dir.path());
    let expected = {
        let left_oid = git_blob_sha1_hex_bytes(&left_bytes);
        let right_oid = git_blob_sha1_hex_bytes(&right_bytes);
        format!(
            r#"diff --git a/<TMP>/bin.dat b/<TMP>/bin.dat
index {left_oid}..{right_oid}
--- a/<TMP>/bin.dat
+++ b/<TMP>/bin.dat
Binary files differ
"#
        )
    };
    assert_eq!(diff, expected);
}

#[tokio::test]
async fn preserves_committed_change_order_with_delete_then_move_overwrite() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), "from\n").expect("seed source");
    fs::write(dir.path().join("b.txt"), "existing\n").expect("seed destination");

    let mut tracker = TurnDiffTracker::with_display_root(dir.path().to_path_buf());
    let ordered_patch = apply_verified_patch(
        &mut tracker,
        dir.path(),
        "*** Begin Patch\n*** Delete File: b.txt\n*** Update File: a.txt\n*** Move to: b.txt\n@@\n-from\n+new\n*** End Patch",
    )
    .await;
    tracker.track_delta(&ordered_patch);

    let left_oid_a = git_blob_sha1_hex("from\n");
    let left_oid_b = git_blob_sha1_hex("existing\n");
    let right_oid_b = git_blob_sha1_hex("new\n");
    let expected = format!(
        r#"diff --git a/a.txt b/a.txt
deleted file mode {REGULAR_FILE_MODE}
index {left_oid_a}..{ZERO_OID}
--- a/a.txt
+++ {DEV_NULL}
@@ -1 +0,0 @@
-from
diff --git a/b.txt b/b.txt
index {left_oid_b}..{right_oid_b}
--- a/b.txt
+++ b/b.txt
@@ -1 +1 @@
-existing
+new
"#,
    );
    assert_eq!(get_unified_diff(&mut tracker), Some(expected));
}
