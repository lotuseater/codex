use super::*;

impl Config {
    /// Whether model-visible MCP tool names should keep the legacy `mcp__` prefix.
    pub(crate) fn prefix_mcp_tool_names(&self) -> bool {
        !self.features.enabled(Feature::NonPrefixedMcpToolNames)
    }

    pub fn legacy_sandbox_policy(&self) -> SandboxPolicy {
        self.permissions.legacy_sandbox_policy(self.cwd.as_path())
    }

    pub fn set_legacy_sandbox_policy(
        &mut self,
        sandbox_policy: SandboxPolicy,
    ) -> ConstraintResult<()> {
        self.permissions
            .set_legacy_sandbox_policy(sandbox_policy, self.cwd.as_path())?;
        self.workspace_roots = self.permissions.workspace_roots().to_vec();
        Ok(())
    }

    /// Effective runtime workspace roots: thread-scoped roots plus any roots
    /// contributed by the active named permission profile, de-duplicated.
    pub fn effective_workspace_roots(&self) -> Vec<AbsolutePathBuf> {
        let mut workspace_roots = self.workspace_roots.clone();
        workspace_roots.extend(self.permissions.profile_workspace_roots().iter().cloned());
        dedupe_absolute_paths(&mut workspace_roots);
        workspace_roots
    }

    pub fn set_windows_sandbox_enabled(&mut self, value: bool) {
        self.permissions.windows_sandbox_mode = if value {
            Some(WindowsSandboxModeToml::Unelevated)
        } else if matches!(
            self.permissions.windows_sandbox_mode,
            Some(WindowsSandboxModeToml::Unelevated)
        ) {
            None
        } else {
            self.permissions.windows_sandbox_mode
        };
    }

    pub fn set_windows_elevated_sandbox_enabled(&mut self, value: bool) {
        self.permissions.windows_sandbox_mode = if value {
            Some(WindowsSandboxModeToml::Elevated)
        } else if matches!(
            self.permissions.windows_sandbox_mode,
            Some(WindowsSandboxModeToml::Elevated)
        ) {
            None
        } else {
            self.permissions.windows_sandbox_mode
        };
    }

    pub fn managed_network_requirements_enabled(&self) -> bool {
        !matches!(
            self.permissions.permission_profile.get(),
            PermissionProfile::Disabled
        ) && self
            .config_layer_stack
            .requirements_toml()
            .network
            .is_some()
    }

    pub(crate) fn network_proxy_spec_for_active_permission_profile(
        &self,
        active_permission_profile: &ActivePermissionProfile,
        permission_profile: &PermissionProfile,
    ) -> std::io::Result<Option<NetworkProxySpec>> {
        let profile_allows_network_proxy =
            profile_allows_configured_network_proxy(permission_profile);
        let configured_network_proxy_config = if profile_allows_network_proxy {
            let cfg: ConfigToml = self
                .config_layer_stack
                .effective_config()
                .try_into()
                .map_err(|err| {
                    std::io::Error::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "failed to read effective config for selected permission profile: {err}"
                        ),
                    )
                })?;
            let mut configured_network_proxy_config = network_proxy_config_for_profile_selection(
                cfg.permissions.as_ref(),
                active_permission_profile.id.as_str(),
            )?;
            if self.features.enabled(Feature::NetworkProxy)
                && permission_profile.network_sandbox_policy().is_enabled()
            {
                if let Some(network_proxy) = network_proxy_toml_config(cfg.features.as_ref()) {
                    apply_network_proxy_feature_config(
                        &mut configured_network_proxy_config,
                        network_proxy,
                    );
                }
                configured_network_proxy_config.network.enabled = true;
            }
            configured_network_proxy_config
        } else {
            NetworkProxyConfig::default()
        };

        build_network_proxy_spec(
            configured_network_proxy_config,
            self.config_layer_stack.requirements().network.clone(),
            permission_profile,
        )
    }

    pub fn bundled_skills_enabled(&self) -> bool {
        crate::manager::bundled_skills_enabled_from_stack(&self.config_layer_stack)
    }
}
