use super::*;
use pretty_assertions::assert_eq;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

const REGULAR_FILE_MODE: &str = "100644";

fn git_blob_sha1_hex(data: &str) -> String {
    git_blob_sha1_hex_bytes(data.as_bytes())
}

fn get_unified_diff(tracker: &mut TurnDiffTracker) -> Option<String> {
    tracker.get_unified_diff().expect("diff should render")
}

fn normalize_diff_for_test(diff: &str, root: &Path) -> String {
    diff.replace(&root.display().to_string().replace('\\', "/"), "<TMP>")
}

fn begin_change(tracker: &mut TurnDiffTracker, path: &Path, change: TrackedFileChange) {
    tracker.on_patch_begin(&HashMap::from([(path.to_path_buf(), change)]));
}

fn commit_add(path: &Path, content: &str) -> CommittedFileChange {
    fs::write(path, content).expect("write add");
    CommittedFileChange::Add {
        path: path.to_path_buf(),
        overwritten_content: None,
    }
}

fn commit_delete(path: &Path, content: &str) -> CommittedFileChange {
    fs::remove_file(path).expect("delete file");
    CommittedFileChange::Delete {
        path: path.to_path_buf(),
        content: content.to_string(),
    }
}

fn commit_update(path: &Path, old_content: &str, new_content: &str) -> CommittedFileChange {
    fs::write(path, new_content).expect("write update");
    CommittedFileChange::Update {
        path: path.to_path_buf(),
        move_path: None,
        old_content: old_content.to_string(),
        overwritten_move_content: None,
    }
}

fn commit_move(
    path: &Path,
    dest: &Path,
    old_content: &str,
    new_content: &str,
    overwritten_move_content: Option<&str>,
) -> CommittedFileChange {
    let _ = fs::remove_file(path);
    fs::write(dest, new_content).expect("write move destination");
    CommittedFileChange::Update {
        path: path.to_path_buf(),
        move_path: Some(dest.to_path_buf()),
        old_content: old_content.to_string(),
        overwritten_move_content: overwritten_move_content.map(str::to_string),
    }
}

#[test]
fn accumulates_add_then_update_as_single_add() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("a.txt");
    let mut tracker = TurnDiffTracker::with_display_root(dir.path().to_path_buf());

    begin_change(
        &mut tracker,
        &file,
        TrackedFileChange::Add {
            content: "foo\n".to_string(),
        },
    );
    let add = commit_add(&file, "foo\n");
    tracker.track_delta(&[add]);

    begin_change(
        &mut tracker,
        &file,
        TrackedFileChange::Update { move_path: None },
    );
    let update = commit_update(&file, "foo\n", "bar\n");
    tracker.track_delta(&[update]);

    let right_oid = git_blob_sha1_hex("bar\n");
    let expected = format!(
        r#"diff --git a/a.txt b/a.txt
new file mode {REGULAR_FILE_MODE}
index {ZERO_OID}..{right_oid}
--- {DEV_NULL}
+++ b/a.txt
@@ -0,0 +1 @@
+bar
"#,
    );
    assert_eq!(get_unified_diff(&mut tracker), Some(expected));
}

#[test]
fn tracks_delete_from_existing_file() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("a.txt");
    fs::write(&file, "gone\n").expect("seed file");
    let mut tracker = TurnDiffTracker::with_display_root(dir.path().to_path_buf());

    begin_change(
        &mut tracker,
        &file,
        TrackedFileChange::Delete {
            content: "gone\n".to_string(),
        },
    );
    let delete = commit_delete(&file, "gone\n");
    tracker.track_delta(&[delete]);

    let left_oid = git_blob_sha1_hex("gone\n");
    let expected = format!(
        r#"diff --git a/a.txt b/a.txt
deleted file mode {REGULAR_FILE_MODE}
index {left_oid}..{ZERO_OID}
--- a/a.txt
+++ {DEV_NULL}
@@ -1 +0,0 @@
-gone
"#,
    );
    assert_eq!(get_unified_diff(&mut tracker), Some(expected));
}

#[test]
fn tracks_move_with_destination_overwrite() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("a.txt");
    let dest = dir.path().join("b.txt");
    fs::write(&source, "from\n").expect("seed source");
    fs::write(&dest, "existing\n").expect("seed destination");
    let mut tracker = TurnDiffTracker::with_display_root(dir.path().to_path_buf());

    begin_change(
        &mut tracker,
        &source,
        TrackedFileChange::Update {
            move_path: Some(dest.clone()),
        },
    );
    let moved = commit_move(&source, &dest, "from\n", "new\n", Some("existing\n"));
    tracker.track_delta(&[moved]);

    let left_oid_source = git_blob_sha1_hex("from\n");
    let left_oid_dest = git_blob_sha1_hex("existing\n");
    let right_oid_dest = git_blob_sha1_hex("new\n");
    let expected = format!(
        r#"diff --git a/a.txt b/a.txt
deleted file mode {REGULAR_FILE_MODE}
index {left_oid_source}..{ZERO_OID}
--- a/a.txt
+++ {DEV_NULL}
@@ -1 +0,0 @@
-from
diff --git a/b.txt b/b.txt
index {left_oid_dest}..{right_oid_dest}
--- a/b.txt
+++ b/b.txt
@@ -1 +1 @@
-existing
+new
"#,
    );
    assert_eq!(get_unified_diff(&mut tracker), Some(expected));
}

#[test]
fn binary_files_differ_update() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("bin.dat");
    let left_bytes: Vec<u8> = vec![0xff, 0xfe, 0xfd, 0x00];
    let right_bytes: Vec<u8> = vec![0x01, 0x02, 0x03, 0x00];
    fs::write(&file, &left_bytes).unwrap();

    let mut tracker = TurnDiffTracker::new();
    begin_change(
        &mut tracker,
        &file,
        TrackedFileChange::Update { move_path: None },
    );
    fs::write(&file, &right_bytes).unwrap();

    let diff = tracker.get_unified_diff().unwrap().unwrap();
    let diff = normalize_diff_for_test(&diff, dir.path());
    let left_oid = git_blob_sha1_hex_bytes(&left_bytes);
    let right_oid = git_blob_sha1_hex_bytes(&right_bytes);
    let expected = format!(
        r#"diff --git a/<TMP>/bin.dat b/<TMP>/bin.dat
index {left_oid}..{right_oid}
--- a/<TMP>/bin.dat
+++ b/<TMP>/bin.dat
Binary files differ
"#
    );
    assert_eq!(diff, expected);
}
