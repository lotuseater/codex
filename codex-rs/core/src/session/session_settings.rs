use super::*;

impl Session {
    pub(crate) async fn app_server_client_metadata(&self) -> AppServerClientMetadata {
        let state = self.state.lock().await;
        AppServerClientMetadata {
            client_name: state.session_configuration.app_server_client_name.clone(),
            client_version: state
                .session_configuration
                .app_server_client_version
                .clone(),
        }
    }

    pub(crate) async fn configured_multi_agent_v2_usage_hint_texts(&self) -> Vec<String> {
        if !self.features.enabled(Feature::MultiAgentV2) {
            return Vec::new();
        }

        let state = self.state.lock().await;
        let config = &state.session_configuration.original_config_do_not_use;
        if !config.multi_agent_v2.usage_hint_enabled {
            return Vec::new();
        }
        [
            config.multi_agent_v2.root_agent_usage_hint_text.clone(),
            config.multi_agent_v2.subagent_usage_hint_text.clone(),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    pub(crate) async fn get_base_instructions(&self) -> BaseInstructions {
        let state = self.state.lock().await;
        BaseInstructions {
            text: state.session_configuration.base_instructions.clone(),
        }
    }

    // Merges connector IDs into the session-level explicit connector selection.
    pub(crate) async fn merge_connector_selection(
        &self,
        connector_ids: HashSet<String>,
    ) -> HashSet<String> {
        let mut state = self.state.lock().await;
        state.merge_connector_selection(connector_ids)
    }

    // Returns the connector IDs currently selected for this session.
    pub(crate) async fn get_connector_selection(&self) -> HashSet<String> {
        let state = self.state.lock().await;
        state.get_connector_selection()
    }

    // Clears connector IDs that were accumulated for explicit selection.
    pub(crate) async fn clear_connector_selection(&self) {
        let mut state = self.state.lock().await;
        state.clear_connector_selection();
    }

    pub(crate) fn maybe_refresh_shell_snapshot_for_cwd(
        &self,
        previous_cwd: &AbsolutePathBuf,
        next_cwd: &AbsolutePathBuf,
        codex_home: &AbsolutePathBuf,
        session_source: &SessionSource,
    ) {
        if previous_cwd == next_cwd {
            return;
        }

        if !self.features.enabled(Feature::ShellSnapshot) {
            return;
        }

        if matches!(
            session_source,
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. })
        ) {
            return;
        }

        ShellSnapshot::refresh_snapshot(
            codex_home.clone(),
            self.thread_id,
            next_cwd.clone(),
            self.services.user_shell.as_ref().clone(),
            self.services.shell_snapshot_tx.clone(),
            self.services.session_telemetry.clone(),
            self.services.state_db.clone(),
        );
    }

    pub(crate) async fn update_settings(
        &self,
        updates: SessionSettingsUpdate,
    ) -> ConstraintResult<()> {
        let notify_config_contributors = !self.services.extensions.config_contributors().is_empty();
        let (
            previous_config,
            new_config,
            previous_cwd,
            permission_profile_changed,
            next_cwd,
            codex_home,
            session_source,
        ) = {
            let mut state = self.state.lock().await;
            let updated = match state.session_configuration.apply(&updates) {
                Ok(updated) => updated,
                Err(err) => {
                    warn!("rejected session settings update: {err}");
                    return Err(err);
                }
            };

            let previous_config = notify_config_contributors
                .then(|| Self::build_effective_session_config(&state.session_configuration));
            let new_config =
                notify_config_contributors.then(|| Self::build_effective_session_config(&updated));
            let previous_cwd = state.session_configuration.cwd().clone();
            let previous_permission_profile = state.session_configuration.permission_profile();
            let updated_permission_profile = updated.permission_profile();
            let permission_profile_changed =
                previous_permission_profile != updated_permission_profile;
            let next_cwd = updated.cwd().clone();
            let codex_home = updated.codex_home.clone();
            let session_source = updated.session_source.clone();
            state.session_configuration = updated;
            (
                previous_config,
                new_config,
                previous_cwd,
                permission_profile_changed,
                next_cwd,
                codex_home,
                session_source,
            )
        };

        self.emit_config_changed_contributors(previous_config.as_ref(), new_config.as_ref());
        self.maybe_refresh_shell_snapshot_for_cwd(
            &previous_cwd,
            &next_cwd,
            &codex_home,
            &session_source,
        );
        if permission_profile_changed {
            self.refresh_managed_network_proxy_for_current_permission_profile()
                .await;
        }

        Ok(())
    }

    pub(crate) async fn validate_settings(
        &self,
        updates: &SessionSettingsUpdate,
    ) -> ConstraintResult<()> {
        let state = self.state.lock().await;
        state.session_configuration.apply(updates).map(|_| ())
    }

    pub(crate) async fn preview_settings(
        &self,
        updates: &SessionSettingsUpdate,
    ) -> ConstraintResult<ThreadConfigSnapshot> {
        let state = self.state.lock().await;
        state
            .session_configuration
            .apply(updates)
            .map(|configuration| configuration.thread_config_snapshot())
    }

    pub(crate) async fn set_session_startup_prewarm(
        &self,
        startup_prewarm: SessionStartupPrewarmHandle,
    ) {
        let mut state = self.state.lock().await;
        state.set_session_startup_prewarm(startup_prewarm);
    }

    pub(crate) async fn take_session_startup_prewarm(&self) -> Option<SessionStartupPrewarmHandle> {
        let mut state = self.state.lock().await;
        state.take_session_startup_prewarm()
    }

    pub(crate) async fn get_config(&self) -> std::sync::Arc<Config> {
        let state = self.state.lock().await;
        state
            .session_configuration
            .original_config_do_not_use
            .clone()
    }

    pub(crate) async fn provider(&self) -> ModelProviderInfo {
        let state = self.state.lock().await;
        state.session_configuration.provider.clone()
    }

    pub(crate) async fn refresh_runtime_config(&self, next_config: Config) {
        // Refresh only the user layer from the incoming snapshot. Preserve thread-local
        // layers such as request/session overrides that were present when this session
        // was created.
        let notify_config_contributors = !self.services.extensions.config_contributors().is_empty();
        let (previous_config, new_config, config) = {
            let mut state = self.state.lock().await;
            let previous_config = notify_config_contributors
                .then(|| Self::build_effective_session_config(&state.session_configuration));
            let mut config = (*state.session_configuration.original_config_do_not_use).clone();
            config.config_layer_stack = config
                .config_layer_stack
                .with_user_layer_from(&next_config.config_layer_stack);
            config.tool_suggest =
                resolve_tool_suggest_config_from_layer_stack(&config.config_layer_stack);
            let config = Arc::new(config);
            state.session_configuration.original_config_do_not_use = Arc::clone(&config);
            let new_config = notify_config_contributors
                .then(|| Self::build_effective_session_config(&state.session_configuration));
            (previous_config, new_config, config)
        };
        self.emit_config_changed_contributors(previous_config.as_ref(), new_config.as_ref());
        self.services.skills_service.clear_cache();
        self.services.plugins_manager.clear_cache();
        let hooks = build_hooks_for_config(
            config.as_ref(),
            self.services.plugins_manager.as_ref(),
            self.services.user_shell.as_ref(),
        )
        .await;

        let state = self.state.lock().await;
        // A newer refresh may have updated the config while this hook build was in flight.
        // Only publish hooks derived from the current config snapshot.
        if Arc::ptr_eq(
            &state.session_configuration.original_config_do_not_use,
            &config,
        ) {
            self.services.hooks.store(Arc::new(hooks));
        }
    }

    pub(crate) fn emit_config_changed_contributors(
        &self,
        previous_config: Option<&Config>,
        new_config: Option<&Config>,
    ) {
        let (Some(previous_config), Some(new_config)) = (previous_config, new_config) else {
            return;
        };
        if previous_config == new_config {
            return;
        }
        for contributor in self.services.extensions.config_contributors() {
            contributor.on_config_changed(
                &self.services.session_extension_data,
                &self.services.thread_extension_data,
                previous_config,
                new_config,
            );
        }
    }

    pub(crate) async fn reload_user_config_layer(&self) {
        // Refresh layer-backed runtime state for an existing session, including enabled plugin,
        // skill, and hook state. Derived config fields such as feature gates and legacy notify
        // settings remain session-static.
        //
        // Prefer `refresh_runtime_config()` when the host can already provide a materialized
        // config snapshot. This file-based path exists for legacy local reload flows.
        let config_toml_path = {
            let state = self.state.lock().await;
            state
                .session_configuration
                .codex_home
                .join(CONFIG_TOML_FILE)
        };

        let user_config = match std::fs::read_to_string(&config_toml_path) {
            Ok(contents) => match toml::from_str::<toml::Value>(&contents) {
                Ok(config) => config,
                Err(err) => {
                    warn!("failed to parse user config while reloading layer: {err}");
                    return;
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                toml::Value::Table(Default::default())
            }
            Err(err) => {
                warn!("failed to read user config while reloading layer: {err}");
                return;
            }
        };

        let next_config = {
            let state = self.state.lock().await;
            let mut config = (*state.session_configuration.original_config_do_not_use).clone();
            config.config_layer_stack = config
                .config_layer_stack
                .with_user_config(&config_toml_path, user_config);
            config.tool_suggest =
                resolve_tool_suggest_config_from_layer_stack(&config.config_layer_stack);
            config
        };
        self.refresh_runtime_config(next_config).await;
    }

    pub(crate) async fn build_settings_update_items(
        &self,
        reference_context_item: Option<&TurnContextItem>,
        current_context: &TurnContext,
    ) -> Vec<ResponseItem> {
        // TODO: Make context updates a pure diff of persisted previous/current TurnContextItem
        // state so replay/backtracking is deterministic. Runtime inputs that affect model-visible
        // context (shell, exec policy, feature gates, previous-turn bridge) should be persisted
        // state or explicit non-state replay events.
        let previous_turn_settings = {
            let state = self.state.lock().await;
            state.previous_turn_settings()
        };
        let exec_policy = self.services.exec_policy.current();
        let mut items = crate::context_manager::updates::build_settings_update_items(
            reference_context_item,
            previous_turn_settings.as_ref(),
            current_context,
            exec_policy.as_ref(),
            self.features.enabled(Feature::Personality),
        );
        let previous_mode = reference_context_item
            .and_then(|item| item.collaboration_mode.as_ref().map(|mode| mode.mode));
        if current_context.collaboration_mode.mode == ModeKind::Plan
            && previous_mode != Some(ModeKind::Plan)
            && let Some(usage_hint_text) =
                multi_agents::usage_hint_text(current_context, &current_context.session_source)
            // fork-local: route the plan-mode hint through the shared builder so
            // it honors the configured delegation injection role (user-role
            // fragment by default, developer fallback).
            && let Some(usage_hint_message) = multi_agents::build_usage_hint_item(
                &current_context.config.multi_agent_v2,
                vec![usage_hint_text],
            )
        {
            items.push(usage_hint_message);
        }
        items
    }

    pub fn enabled(&self, feature: Feature) -> bool {
        self.features.enabled(feature)
    }

    pub(crate) fn features(&self) -> ManagedFeatures {
        self.features.clone()
    }

    pub(crate) async fn collaboration_mode(&self) -> CollaborationMode {
        let state = self.state.lock().await;
        state.session_configuration.collaboration_mode.clone()
    }
}
