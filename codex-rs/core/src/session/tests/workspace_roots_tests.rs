use super::*;
use crate::config::ThreadStoreConfig;
use codex_thread_store::InMemoryThreadStore;
use codex_thread_store::LocalThreadStore;
use codex_thread_store::LocalThreadStoreConfig;
use codex_thread_store::thread_store_from_config;

#[tokio::test]
async fn session_configuration_apply_preserves_absolute_cwd_write_root_on_cwd_update() {
    let mut session_configuration = make_session_configuration_for_tests().await;
    let workspace = tempfile::tempdir().expect("create temp dir");
    let original_cwd = workspace.path().join("repo-a");
    let next_cwd = workspace.path().join("repo-b");
    std::fs::create_dir_all(&original_cwd).expect("create original cwd");
    std::fs::create_dir_all(&next_cwd).expect("create next cwd");
    let original_cwd = original_cwd.abs();

    session_configuration.cwd = original_cwd.clone();
    let file_system_sandbox_policy = FileSystemSandboxPolicy::restricted(vec![
        FileSystemSandboxEntry {
            path: FileSystemPath::Special {
                value: FileSystemSpecialPath::Root,
            },
            access: FileSystemAccessMode::Read,
        },
        FileSystemSandboxEntry {
            path: FileSystemPath::Path {
                path: original_cwd.clone(),
            },
            access: FileSystemAccessMode::Write,
        },
    ]);
    session_configuration.permission_profile = codex_config::Constrained::allow_any(
        PermissionProfile::from_runtime_permissions_with_enforcement(
            SandboxEnforcement::Managed,
            &file_system_sandbox_policy,
            NetworkSandboxPolicy::Restricted,
        ),
    );

    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            cwd: Some(next_cwd.clone()),
            ..Default::default()
        })
        .expect("cwd-only update should succeed");

    assert_eq!(
        updated.file_system_sandbox_policy(),
        file_system_sandbox_policy
    );
    assert!(
        updated
            .file_system_sandbox_policy()
            .can_write_path_with_cwd(original_cwd.as_path(), updated.cwd.as_path()),
        "absolute grant to the old cwd must remain writable"
    );
    assert!(
        !updated
            .file_system_sandbox_policy()
            .can_write_path_with_cwd(next_cwd.as_path(), updated.cwd.as_path()),
        "cwd-only update must not reinterpret an absolute old-cwd grant as :project_roots"
    );
}

#[tokio::test]
async fn session_settings_update_preserves_workspace_roots_in_snapshot() {
    let (session, _turn_context, _rx) = make_session_and_context_with_rx().await;
    let config = session.get_config().await;
    let runtime_root =
        AbsolutePathBuf::resolve_path_against_base("runtime-root", config.cwd.as_path());
    let profile_root =
        AbsolutePathBuf::resolve_path_against_base("profile-root", config.cwd.as_path());

    session
        .update_settings(SessionSettingsUpdate {
            workspace_roots: Some(vec![runtime_root.clone()]),
            profile_workspace_roots: Some(vec![profile_root.clone()]),
            ..Default::default()
        })
        .await
        .expect("workspace roots update should be accepted");

    let snapshot = {
        let state = session.state.lock().await;
        state.session_configuration.thread_config_snapshot()
    };
    assert_eq!(snapshot.workspace_roots, vec![runtime_root]);
    assert_eq!(snapshot.profile_workspace_roots, vec![profile_root]);
}
