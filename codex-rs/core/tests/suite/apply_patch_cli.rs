#![allow(clippy::expect_used)]
#![allow(dead_code)]

use anyhow::Result;
use codex_core_test_runtime::test_codex::TestCodexHarness;
use codex_protocol::models::PermissionProfile;
use codex_protocol::permissions::FileSystemAccessMode;
use codex_protocol::permissions::FileSystemPath;
use codex_protocol::permissions::FileSystemSandboxEntry;
use codex_protocol::permissions::FileSystemSandboxPolicy;
use codex_protocol::permissions::FileSystemSpecialPath;
use codex_protocol::permissions::NetworkSandboxPolicy;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::SandboxPolicy;
use codex_protocol::user_input::UserInput;
use codex_utils_absolute_path::AbsolutePathBuf;

pub(crate) async fn submit_without_wait(harness: &TestCodexHarness, prompt: &str) -> Result<()> {
    submit_without_wait_with_turn_permissions(
        harness,
        prompt,
        SandboxPolicy::DangerFullAccess,
        /*permission_profile*/ None,
    )
    .await
}

pub(crate) async fn submit_without_wait_with_turn_permissions(
    harness: &TestCodexHarness,
    prompt: &str,
    sandbox_policy: SandboxPolicy,
    permission_profile: Option<PermissionProfile>,
) -> Result<()> {
    let test = harness.test();
    let session_model = test.session_configured.model.clone();
    test.codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: prompt.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: harness.cwd().to_path_buf(),
            approval_policy: AskForApproval::Never,
            approvals_reviewer: None,
            sandbox_policy,
            permission_profile,
            model: session_model,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            personality: None,
        })
        .await?;
    Ok(())
}

pub(crate) fn restrictive_workspace_write_profile() -> PermissionProfile {
    PermissionProfile::workspace_write_with(
        &[],
        NetworkSandboxPolicy::Restricted,
        /*exclude_tmpdir_env_var*/ true,
        /*exclude_slash_tmp*/ true,
    )
}

pub(crate) fn workspace_write_with_read_only_root(
    read_only_root: AbsolutePathBuf,
) -> PermissionProfile {
    let file_system_sandbox_policy = FileSystemSandboxPolicy::restricted(vec![
        FileSystemSandboxEntry {
            path: FileSystemPath::Path {
                path: read_only_root,
            },
            access: FileSystemAccessMode::Read,
        },
        FileSystemSandboxEntry {
            path: FileSystemPath::Special {
                value: FileSystemSpecialPath::project_roots(/*subpath*/ None),
            },
            access: FileSystemAccessMode::Write,
        },
    ]);
    PermissionProfile::from_runtime_permissions(
        &file_system_sandbox_policy,
        NetworkSandboxPolicy::Restricted,
    )
}

#[cfg(unix)]
pub(crate) fn workspace_write_with_unreadable_path(
    unreadable_path: AbsolutePathBuf,
) -> PermissionProfile {
    let file_system_sandbox_policy = FileSystemSandboxPolicy::restricted(vec![
        FileSystemSandboxEntry {
            path: FileSystemPath::Path {
                path: unreadable_path,
            },
            access: FileSystemAccessMode::None,
        },
        FileSystemSandboxEntry {
            path: FileSystemPath::Special {
                value: FileSystemSpecialPath::project_roots(/*subpath*/ None),
            },
            access: FileSystemAccessMode::Write,
        },
    ]);
    PermissionProfile::from_runtime_permissions(
        &file_system_sandbox_policy,
        NetworkSandboxPolicy::Restricted,
    )
}

#[cfg(unix)]
pub(crate) fn create_file_symlink(
    source: &std::path::Path,
    link: &std::path::Path,
) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, link)
}

#[cfg(windows)]
pub(crate) fn create_file_symlink(
    source: &std::path::Path,
    link: &std::path::Path,
) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(source, link)
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn create_file_symlink(
    _source: &std::path::Path,
    _link: &std::path::Path,
) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "file symlinks are unsupported on this platform",
    ))
}
