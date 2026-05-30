use super::*;

/// Application configuration loaded from disk and merged with overrides.
#[derive(Debug, Clone, PartialEq)]
pub struct Permissions {
    /// Approval policy for executing commands.
    pub approval_policy: Constrained<AskForApproval>,
    /// Canonical effective runtime permissions after config requirements and
    /// runtime readable-root additions have been applied.
    pub permission_profile: Constrained<PermissionProfile>,
    /// Named or implicit built-in profile selected by config, rather than an
    /// ad-hoc override.
    pub active_permission_profile: Option<ActivePermissionProfile>,
    /// Workspace roots contributed by the active named permission profile.
    /// Empty when no named profile is active or the profile defines none.
    pub(crate) profile_workspace_roots: Vec<AbsolutePathBuf>,
    /// Thread-scoped runtime workspace roots. Symbolic `:project_roots`
    /// entries in the permission profile are materialized against these roots
    /// when computing the effective permission profile.
    pub(crate) workspace_roots: Vec<AbsolutePathBuf>,
    /// Effective network configuration applied to all spawned processes.
    pub network: Option<NetworkProxySpec>,
    /// Whether the model may request a login shell for shell-based tools.
    /// Default to `true`
    ///
    /// If `true`, the model may request a login shell (`login = true`), and
    /// omitting `login` defaults to using a login shell.
    /// If `false`, the model can never use a login shell: `login = true`
    /// requests are rejected, and omitting `login` defaults to a non-login
    /// shell.
    pub allow_login_shell: bool,
    /// Policy used to build process environments for shell/unified exec.
    pub shell_environment_policy: ShellEnvironmentPolicy,
    /// Effective Windows sandbox mode derived from `[windows].sandbox` or
    /// legacy feature keys.
    pub windows_sandbox_mode: Option<WindowsSandboxModeToml>,
    /// Whether the final Windows sandboxed child should run on a private desktop.
    pub windows_sandbox_private_desktop: bool,
}

impl Permissions {
    /// Construct permissions from an approval policy and a canonical permission
    /// profile, with no profile-declared or runtime workspace roots.
    ///
    /// This is the constructor used by lightweight consumers (e.g. the
    /// thread-manager sample binary) that build a [`Config`] by hand rather
    /// than through the config loader. Field defaults mirror the loader's
    /// behavior for a freshly constructed configuration.
    pub fn from_approval_and_profile(
        approval_policy: Constrained<AskForApproval>,
        permission_profile: Constrained<PermissionProfile>,
    ) -> ConstraintResult<Self> {
        permission_profile.can_set(permission_profile.get())?;
        Ok(Self {
            approval_policy,
            permission_profile,
            active_permission_profile: None,
            profile_workspace_roots: Vec::new(),
            workspace_roots: Vec::new(),
            network: None,
            allow_login_shell: true,
            shell_environment_policy: ShellEnvironmentPolicy::default(),
            windows_sandbox_mode: None,
            windows_sandbox_private_desktop: false,
        })
    }

    /// Effective runtime permissions after config requirements and runtime
    /// readable-root additions have been applied.
    pub fn permission_profile(&self) -> PermissionProfile {
        self.permission_profile.get().clone()
    }

    /// Named profile selected by config, if the current profile has one.
    pub fn active_permission_profile(&self) -> Option<ActivePermissionProfile> {
        self.active_permission_profile.clone()
    }

    /// Workspace roots contributed by the active named permission profile.
    pub fn profile_workspace_roots(&self) -> &[AbsolutePathBuf] {
        &self.profile_workspace_roots
    }

    /// Effective filesystem sandbox policy derived from the canonical profile.
    pub fn file_system_sandbox_policy(&self) -> FileSystemSandboxPolicy {
        self.permission_profile.get().file_system_sandbox_policy()
    }

    /// Effective network sandbox policy derived from the canonical profile.
    pub fn network_sandbox_policy(&self) -> NetworkSandboxPolicy {
        self.permission_profile.get().network_sandbox_policy()
    }

    /// Legacy compatibility projection derived from the canonical profile.
    pub fn legacy_sandbox_policy(&self, cwd: &Path) -> SandboxPolicy {
        let permission_profile = self.permission_profile.get();
        let file_system_sandbox_policy = permission_profile.file_system_sandbox_policy();
        compatibility_sandbox_policy_for_permission_profile(
            permission_profile,
            &file_system_sandbox_policy,
            permission_profile.network_sandbox_policy(),
            cwd,
        )
    }

    /// Check whether a legacy sandbox policy can be applied to this permission
    /// set after projecting it into the canonical permission profile.
    pub fn can_set_legacy_sandbox_policy(
        &self,
        sandbox_policy: &SandboxPolicy,
        cwd: &Path,
    ) -> ConstraintResult<()> {
        let file_system_sandbox_policy =
            FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(sandbox_policy, cwd);
        let network_sandbox_policy = NetworkSandboxPolicy::from(sandbox_policy);
        let permission_profile = PermissionProfile::from_runtime_permissions_with_enforcement(
            SandboxEnforcement::from_legacy_sandbox_policy(sandbox_policy),
            &file_system_sandbox_policy,
            network_sandbox_policy,
        );
        self.permission_profile.can_set(&permission_profile)
    }

    /// Replace permissions from a legacy sandbox policy and keep every
    /// permission projection in sync.
    pub fn set_legacy_sandbox_policy(
        &mut self,
        sandbox_policy: SandboxPolicy,
        cwd: &Path,
    ) -> ConstraintResult<()> {
        self.can_set_legacy_sandbox_policy(&sandbox_policy, cwd)?;
        let file_system_sandbox_policy =
            FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(&sandbox_policy, cwd);
        let network_sandbox_policy = NetworkSandboxPolicy::from(&sandbox_policy);
        let permission_profile = PermissionProfile::from_runtime_permissions_with_enforcement(
            SandboxEnforcement::from_legacy_sandbox_policy(&sandbox_policy),
            &file_system_sandbox_policy,
            network_sandbox_policy,
        );

        self.permission_profile.set(permission_profile)?;
        self.active_permission_profile = None;
        Ok(())
    }

    /// Replace permissions from the canonical profile.
    pub fn set_permission_profile(
        &mut self,
        permission_profile: PermissionProfile,
    ) -> ConstraintResult<()> {
        self.set_permission_profile_with_active_profile(
            permission_profile,
            /*active_permission_profile*/ None,
        )
    }

    /// Replace permissions from the canonical profile and record the named
    /// source profile, if one is known.
    pub fn set_permission_profile_with_active_profile(
        &mut self,
        permission_profile: PermissionProfile,
        active_permission_profile: Option<ActivePermissionProfile>,
    ) -> ConstraintResult<()> {
        self.permission_profile.can_set(&permission_profile)?;

        self.permission_profile.set(permission_profile)?;
        self.active_permission_profile = active_permission_profile;
        Ok(())
    }

    /// Replace the thread-scoped runtime workspace roots. These are
    /// materialized into the effective permission profile alongside any
    /// profile-declared roots.
    pub fn set_workspace_roots(&mut self, workspace_roots: Vec<AbsolutePathBuf>) {
        self.workspace_roots = workspace_roots;
    }

    /// Thread-scoped runtime workspace roots.
    pub fn workspace_roots(&self) -> &[AbsolutePathBuf] {
        &self.workspace_roots
    }

    /// Effective runtime permissions after runtime workspace-root
    /// materialization has been applied to the canonical profile.
    pub fn effective_permission_profile(&self) -> PermissionProfile {
        self.permission_profile
            .get()
            .clone()
            .materialize_project_roots_with_workspace_roots(&self.workspace_roots)
    }

    /// Apply a permission profile snapshot emitted by core session state.
    ///
    /// This is a trusted-state bridge for consumers of `SessionConfigured`.
    /// Config loading and app-server selection should resolve named profiles
    /// through config instead of constructing a snapshot directly.
    pub fn set_permission_profile_from_session_snapshot(
        &mut self,
        snapshot: PermissionProfileSnapshot,
    ) -> ConstraintResult<()> {
        let permission_profile = snapshot.permission_profile().clone();
        self.permission_profile.can_set(&permission_profile)?;
        self.permission_profile.set(permission_profile)?;
        self.active_permission_profile = snapshot.active_permission_profile();
        self.profile_workspace_roots = snapshot.profile_workspace_roots().to_vec();
        Ok(())
    }

    /// Replace the current permission constraints with a trusted session
    /// snapshot. This is only for clients that must mirror core session state
    /// after their local config constraints reject the snapshot.
    pub fn replace_permission_profile_from_session_snapshot(
        &mut self,
        snapshot: PermissionProfileSnapshot,
    ) -> ConstraintResult<()> {
        self.active_permission_profile = snapshot.active_permission_profile();
        self.profile_workspace_roots = snapshot.profile_workspace_roots().to_vec();
        self.permission_profile = Constrained::allow_only(snapshot.permission_profile().clone());
        Ok(())
    }
}
