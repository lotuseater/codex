use super::*;

impl Config {
    pub fn to_models_manager_config(&self) -> ModelsManagerConfig {
        ModelsManagerConfig {
            model_context_window: self.model_context_window,
            model_auto_compact_token_limit: self.model_auto_compact_token_limit,
            tool_output_token_limit: self.tool_output_token_limit,
            context_budget_mode: self.context_budget_mode,
            base_instructions: self.base_instructions.clone(),
            personality_enabled: self.features.enabled(Feature::Personality),
            model_supports_reasoning_summaries: self.model_supports_reasoning_summaries,
            model_catalog: self.model_catalog.clone(),
        }
    }

    /// Build the plugin-manager input from the effective config.
    pub fn plugins_config_input(&self) -> PluginsConfigInput {
        PluginsConfigInput::new(
            self.config_layer_stack.clone(),
            self.features.enabled(Feature::Plugins),
            self.features.enabled(Feature::RemotePlugin),
            self.chatgpt_base_url.clone(),
        )
    }

    /// Applies managed MCP requirements to servers supplied by one plugin.
    ///
    /// Mirrors the per-plugin filtering performed inline in [`Self::to_mcp_config`]
    /// so external contributors (e.g. the MCP executor-plugin extension) can apply
    /// the same requirement gating to plugin-contributed servers.
    pub fn apply_plugin_mcp_server_requirements(
        &self,
        plugin_id: &str,
        mcp_servers: &mut HashMap<String, McpServerConfig>,
    ) {
        filter_plugin_mcp_servers_by_requirements(
            plugin_id,
            mcp_servers,
            self.config_layer_stack.requirements().plugins.as_ref(),
        );
        // A present empty allowlist bans configurable MCPs, including
        // plugin-contributed servers.
        let empty_mcp_allowlist = self
            .config_layer_stack
            .requirements()
            .mcp_servers
            .as_ref()
            .filter(|requirements| requirements.value.is_empty());
        filter_mcp_servers_by_requirements(mcp_servers, empty_mcp_allowlist);
    }

    pub async fn to_mcp_config(
        &self,
        plugins_manager: &codex_core_plugins::PluginsManager,
    ) -> McpConfig {
        let plugins_input = self.plugins_config_input();
        let loaded_plugins = plugins_manager.plugins_for_config(&plugins_input).await;

        // A present empty allowlist bans configurable MCPs, including config-file
        // and plugin-contributed servers. Apply it to both sets before they are
        // registered into the resolved catalog so the ban is preserved.
        let empty_mcp_allowlist = self
            .config_layer_stack
            .requirements()
            .mcp_servers
            .as_ref()
            .filter(|requirements| requirements.value.is_empty());

        let mut catalog = ResolvedMcpCatalog::builder();

        let mut configured_mcp_servers = self.mcp_servers.get().clone();
        filter_mcp_servers_by_requirements(&mut configured_mcp_servers, empty_mcp_allowlist);
        for (name, server) in configured_mcp_servers {
            catalog.register(McpServerRegistration::from_config(name, server));
        }

        for (plugin_order, plugin) in loaded_plugins
            .plugins()
            .iter()
            .filter(|plugin| plugin.is_active())
            .enumerate()
        {
            let mut plugin_mcp_servers = plugin.mcp_servers.clone();
            filter_plugin_mcp_servers_by_requirements(
                &plugin.config_name,
                &mut plugin_mcp_servers,
                self.config_layer_stack.requirements().plugins.as_ref(),
            );
            filter_mcp_servers_by_requirements(&mut plugin_mcp_servers, empty_mcp_allowlist);
            let attribution = McpPluginAttribution::new(
                plugin.config_name.clone(),
                plugin.display_name().to_string(),
            );
            for (name, plugin_server) in plugin_mcp_servers {
                catalog.register(McpServerRegistration::from_plugin(
                    name,
                    attribution.clone(),
                    plugin_order,
                    plugin_server,
                ));
            }
        }

        McpConfig {
            chatgpt_base_url: self.chatgpt_base_url.clone(),
            apps_mcp_product_sku: self.apps_mcp_product_sku.clone(),
            codex_home: self.codex_home.to_path_buf(),
            mcp_oauth_credentials_store_mode: self.mcp_oauth_credentials_store_mode,
            auth_keyring_backend_kind: self.auth_keyring_backend_kind(),
            mcp_oauth_callback_port: self.mcp_oauth_callback_port,
            mcp_oauth_callback_url: self.mcp_oauth_callback_url.clone(),
            skill_mcp_dependency_install_enabled: self
                .features
                .enabled(Feature::SkillMcpDependencyInstall),
            approval_policy: self.permissions.approval_policy.clone(),
            codex_linux_sandbox_exe: self.codex_linux_sandbox_exe.clone(),
            use_legacy_landlock: self.features.use_legacy_landlock(),
            apps_enabled: self.features.enabled(Feature::Apps),
            prefix_mcp_tool_names: self.prefix_mcp_tool_names(),
            client_elicitation_capability: if self.features.enabled(Feature::AuthElicitation) {
                ElicitationCapability {
                    form: Some(FormElicitationCapability::default()),
                    url: Some(UrlElicitationCapability::default()),
                }
            } else {
                // https://modelcontextprotocol.io/specification/2025-06-18/client/elicitation#capabilities
                // indicates this should be an empty object.
                ElicitationCapability::default()
            },
            mcp_server_catalog: catalog.build(),
            plugin_capability_summaries: loaded_plugins.capability_summaries().to_vec(),
        }
    }

    pub async fn rebuild_preserving_session_layers(
        &self,
        refreshed_config: &Config,
    ) -> std::io::Result<Self> {
        let mut layers = refreshed_config
            .config_layer_stack
            .get_layers(
                ConfigLayerStackOrdering::LowestPrecedenceFirst,
                /*include_disabled*/ true,
            )
            .into_iter()
            .filter(|layer| !is_session_layer(&layer.name))
            .cloned()
            .collect::<Vec<_>>();
        layers.extend(
            self.config_layer_stack
                .get_layers(
                    ConfigLayerStackOrdering::LowestPrecedenceFirst,
                    /*include_disabled*/ true,
                )
                .into_iter()
                .filter(|layer| is_session_layer(&layer.name))
                .cloned(),
        );
        layers.sort_by_key(|layer| layer.name.precedence());

        let config_layer_stack = ConfigLayerStack::new(
            layers,
            refreshed_config.config_layer_stack.requirements().clone(),
            refreshed_config
                .config_layer_stack
                .requirements_toml()
                .clone(),
        )?
        .with_user_and_project_exec_policy_rules_ignored(
            refreshed_config
                .config_layer_stack
                .ignore_user_and_project_exec_policy_rules(),
        );
        let cfg: ConfigToml = config_layer_stack
            .effective_config()
            .try_into()
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        let default_zsh_path = refreshed_config
            .zsh_path
            .clone()
            .map(AbsolutePathBuf::try_from)
            .transpose()?;

        Self::load_config_with_layer_stack(
            LOCAL_FS.as_ref(),
            cfg,
            ConfigOverrides {
                cwd: Some(self.cwd.to_path_buf()),
                default_zsh_path,
                ..Default::default()
            },
            refreshed_config.codex_home.clone(),
            config_layer_stack,
        )
        .await
    }
}
