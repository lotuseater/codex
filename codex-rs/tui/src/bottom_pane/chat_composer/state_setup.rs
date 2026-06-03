//! Construction and configuration for [`ChatComposer`]: constructors plus the
//! `set_*`/feature-flag mutators and simple state accessors.
use super::*;

impl ChatComposer {
    pub(crate) fn builtin_command_flags(&self) -> BuiltinCommandFlags {
        BuiltinCommandFlags {
            collaboration_modes_enabled: self.collaboration_modes_enabled,
            connectors_enabled: self.connectors_enabled,
            plugins_command_enabled: self.plugins_command_enabled,
            service_tier_commands_enabled: self.service_tier_commands_enabled,
            goal_command_enabled: self.goal_command_enabled,
            personality_command_enabled: self.personality_command_enabled,
            realtime_conversation_enabled: self.realtime_conversation_enabled,
            audio_device_selection_enabled: self.audio_device_selection_enabled,
            allow_elevate_sandbox: self.windows_degraded_sandbox_active,
            side_conversation_active: self.side_conversation_active,
        }
    }

    pub fn new(
        has_input_focus: bool,
        app_event_tx: AppEventSender,
        enhanced_keys_supported: bool,
        placeholder_text: String,
        disable_paste_burst: bool,
    ) -> Self {
        Self::new_with_config(
            has_input_focus,
            app_event_tx,
            enhanced_keys_supported,
            placeholder_text,
            disable_paste_burst,
            ChatComposerConfig::default(),
        )
    }

    /// Construct a composer with explicit feature gating.
    ///
    /// This enables reuse in contexts like request-user-input where we want
    /// the same visuals and editing behavior without slash commands or popups.
    pub(crate) fn new_with_config(
        has_input_focus: bool,
        app_event_tx: AppEventSender,
        enhanced_keys_supported: bool,
        placeholder_text: String,
        disable_paste_burst: bool,
        config: ChatComposerConfig,
    ) -> Self {
        let use_shift_enter_hint = enhanced_keys_supported;
        let default_keymap = RuntimeKeymap::defaults();
        let default_editor_keymap = default_keymap.editor.clone();
        let default_vim_normal_keymap = default_keymap.vim_normal.clone();

        let mut this = Self {
            textarea: TextArea::new(),
            textarea_state: RefCell::new(TextAreaState::default()),
            is_bash_mode: false,
            active_popup: ActivePopup::None,
            app_event_tx,
            history: ChatComposerHistory::new(),
            quit_shortcut_expires_at: None,
            quit_shortcut_key: key_hint::ctrl(KeyCode::Char('c')),
            esc_backtrack_hint: false,
            use_shift_enter_hint,
            dismissed_file_popup_token: None,
            current_file_query: None,
            pending_pastes: Vec::new(),
            has_focus: has_input_focus,
            frame_requester: None,
            attached_images: Vec::new(),
            placeholder_text,
            is_task_running: false,
            input_enabled: true,
            input_disabled_placeholder: None,
            paste_burst: PasteBurst::default(),
            disable_paste_burst: false,
            footer_mode: FooterMode::ComposerEmpty,
            footer_hint_override: None,
            plan_mode_nudge_visible: false,
            remote_image_urls: Vec::new(),
            selected_remote_image_index: None,
            queue_submissions: false,
            pending_slash_command_history: None,
            footer_flash: None,
            context_window_percent: None,
            #[cfg(not(target_os = "linux"))]
            next_element_id: 0,
            context_window_used_tokens: None,
            skills: None,
            plugins: None,
            connectors_snapshot: None,
            dismissed_mention_popup_token: None,
            mention_bindings: HashMap::new(),
            recent_submission_mention_bindings: Vec::new(),
            collaboration_modes_enabled: false,
            config,
            collaboration_mode_indicator: None,
            goal_status_indicator: None,
            ide_context_active: false,
            connectors_enabled: false,
            plugins_command_enabled: false,
            service_tier_commands_enabled: false,
            service_tier_commands: Vec::new(),
            mentions_v2_enabled: false,
            goal_command_enabled: false,
            personality_command_enabled: false,
            realtime_conversation_enabled: false,
            audio_device_selection_enabled: false,
            windows_degraded_sandbox_active: false,
            side_conversation_active: false,
            status_line_value: None,
            status_line_hyperlink_url: None,
            status_line_enabled: false,
            session_limit_status_line: None,
            side_conversation_context_label: None,
            active_agent_label: None,
            history_search: None,
            submit_keys: vec![key_hint::plain(KeyCode::Enter)],
            queue_keys: vec![key_hint::plain(KeyCode::Tab)],
            toggle_shortcuts_keys: vec![
                key_hint::plain(KeyCode::Char('?')),
                key_hint::shift(KeyCode::Char('?')),
            ],
            history_search_previous_keys: default_keymap.composer.history_search_previous.clone(),
            history_search_next_keys: default_keymap.composer.history_search_next.clone(),
            editor_keymap: default_editor_keymap,
            vim_normal_keymap: default_vim_normal_keymap,
            footer_external_editor_key: Some(key_hint::ctrl(KeyCode::Char('g'))),
            footer_show_transcript_key: Some(key_hint::ctrl(KeyCode::Char('t'))),
            footer_insert_newline_key: footer_insert_newline_key(
                &default_keymap.editor.insert_newline,
                use_shift_enter_hint,
            ),
            footer_queue_key: Some(key_hint::plain(KeyCode::Tab)),
            footer_toggle_shortcuts_key: Some(key_hint::plain(KeyCode::Char('?'))),
            footer_history_search_key: primary_binding(
                &default_keymap.composer.history_search_previous,
            ),
            footer_reasoning_down_key: primary_binding(
                &default_keymap.chat.decrease_reasoning_effort,
            ),
            footer_reasoning_up_key: primary_binding(
                &default_keymap.chat.increase_reasoning_effort,
            ),
        };
        // Apply configuration via the setter to keep side-effects centralized.
        this.set_disable_paste_burst(disable_paste_burst);
        this
    }

    #[cfg(not(target_os = "linux"))]
    pub(crate) fn next_id(&mut self) -> String {
        let id = self.next_element_id;
        self.next_element_id = self.next_element_id.wrapping_add(1);
        id.to_string()
    }

    pub(crate) fn set_frame_requester(&mut self, frame_requester: FrameRequester) {
        self.frame_requester = Some(frame_requester);
    }

    pub fn set_skill_mentions(&mut self, skills: Option<Vec<SkillMetadata>>) {
        self.skills = skills;
        self.sync_popups();
    }

    pub fn set_plugin_mentions(&mut self, plugins: Option<Vec<PluginCapabilitySummary>>) {
        self.plugins = plugins;
        self.sync_popups();
    }

    pub fn set_plugins_command_enabled(&mut self, enabled: bool) {
        self.plugins_command_enabled = enabled;
    }

    pub fn set_mentions_v2_enabled(&mut self, enabled: bool) {
        self.mentions_v2_enabled = enabled;
        self.sync_popups();
    }

    /// Toggle composer-side image paste handling.
    ///
    /// This only affects whether image-like paste content is converted into attachments; the
    /// `ChatWidget` layer still performs capability checks before images are submitted.
    pub fn set_image_paste_enabled(&mut self, enabled: bool) {
        self.config.image_paste_enabled = enabled;
    }

    pub fn set_connector_mentions(&mut self, connectors_snapshot: Option<ConnectorsSnapshot>) {
        self.connectors_snapshot = connectors_snapshot;
        self.sync_popups();
    }

    pub(crate) fn take_mention_bindings(&mut self) -> Vec<MentionBinding> {
        let elements = self.current_mention_elements();
        let mut ordered = Vec::new();
        for (id, mention) in elements {
            if let Some(binding) = self.mention_bindings.remove(&id)
                && binding.mention == mention
            {
                ordered.push(MentionBinding {
                    sigil: binding.sigil,
                    mention: binding.mention,
                    path: binding.path,
                });
            }
        }
        self.mention_bindings.clear();
        ordered
    }

    pub fn set_collaboration_modes_enabled(&mut self, enabled: bool) {
        self.collaboration_modes_enabled = enabled;
    }

    pub fn set_connectors_enabled(&mut self, enabled: bool) {
        self.connectors_enabled = enabled;
    }

    pub fn set_service_tier_commands_enabled(&mut self, enabled: bool) {
        self.service_tier_commands_enabled = enabled;
    }

    pub fn set_service_tier_commands(&mut self, commands: Vec<ServiceTierCommand>) {
        self.service_tier_commands = commands;
        self.sync_popups();
    }

    pub fn set_goal_command_enabled(&mut self, enabled: bool) {
        self.goal_command_enabled = enabled;
    }

    /// Replace composer, editor, and footer-hint key bindings from one runtime snapshot.
    ///
    /// Submit and queue bindings are cached here because composer dispatch must
    /// check them before generic textarea editing. The embedded textarea receives
    /// the same snapshot's editor bindings so a live remap cannot leave submit
    /// keys updated while cursor/editing keys still use old defaults.
    pub(crate) fn set_keymap_bindings(&mut self, keymap: &RuntimeKeymap) {
        self.submit_keys = keymap.composer.submit.clone();
        self.queue_keys = keymap.composer.queue.clone();
        self.toggle_shortcuts_keys = keymap.composer.toggle_shortcuts.clone();
        self.history_search_previous_keys = keymap.composer.history_search_previous.clone();
        self.history_search_next_keys = keymap.composer.history_search_next.clone();
        self.editor_keymap = keymap.editor.clone();
        self.vim_normal_keymap = keymap.vim_normal.clone();
        self.textarea.set_keymap_bindings(keymap);
        self.footer_external_editor_key = primary_binding(&keymap.app.open_external_editor);
        self.footer_show_transcript_key = primary_binding(&keymap.app.open_transcript);
        self.footer_insert_newline_key =
            footer_insert_newline_key(&keymap.editor.insert_newline, self.use_shift_enter_hint);
        self.footer_queue_key = primary_binding(&keymap.composer.queue);
        self.footer_toggle_shortcuts_key = primary_binding(&keymap.composer.toggle_shortcuts);
        self.footer_history_search_key = primary_binding(&keymap.composer.history_search_previous);
        self.footer_reasoning_down_key = primary_binding(&keymap.chat.decrease_reasoning_effort);
        self.footer_reasoning_up_key = primary_binding(&keymap.chat.increase_reasoning_effort);
    }

    pub fn set_collaboration_mode_indicator(
        &mut self,
        indicator: Option<CollaborationModeIndicator>,
    ) {
        self.collaboration_mode_indicator = indicator;
    }

    pub fn set_goal_status_indicator(&mut self, indicator: Option<GoalStatusIndicator>) {
        self.goal_status_indicator = indicator;
    }

    pub fn set_ide_context_active(&mut self, active: bool) {
        self.ide_context_active = active;
    }

    pub fn set_personality_command_enabled(&mut self, enabled: bool) {
        self.personality_command_enabled = enabled;
    }

    pub fn set_realtime_conversation_enabled(&mut self, enabled: bool) {
        self.realtime_conversation_enabled = enabled;
    }

    pub fn set_audio_device_selection_enabled(&mut self, enabled: bool) {
        self.audio_device_selection_enabled = enabled;
    }

    pub fn set_side_conversation_active(&mut self, active: bool) {
        self.side_conversation_active = active;
    }

    /// Compatibility shim for tests that still toggle the removed steer mode flag.
    #[cfg(test)]
    pub fn set_steer_enabled(&mut self, _enabled: bool) {}
    /// Centralized feature gating keeps config checks out of call sites.
    pub(crate) fn popups_enabled(&self) -> bool {
        self.config.popups_enabled
    }

    pub(crate) fn slash_commands_enabled(&self) -> bool {
        self.config.slash_commands_enabled
    }

    pub(crate) fn image_paste_enabled(&self) -> bool {
        self.config.image_paste_enabled
    }
    #[cfg(target_os = "windows")]
    pub fn set_windows_degraded_sandbox_active(&mut self, enabled: bool) {
        self.windows_degraded_sandbox_active = enabled;
    }
    /// Returns true if the composer currently contains no user-entered input.
    pub(crate) fn is_empty(&self) -> bool {
        self.textarea.is_empty()
            && !self.is_bash_mode
            && self.attached_images.is_empty()
            && self.remote_image_urls.is_empty()
    }

    /// Record local persistent-history metadata so the composer can navigate
    /// cross-session history.
    pub(crate) fn set_history_metadata(
        &mut self,
        thread_id: ThreadId,
        log_id: u64,
        entry_count: usize,
    ) {
        self.history.set_metadata(thread_id, log_id, entry_count);
    }

    /// Integrate an asynchronous response to an on-demand history lookup.
    ///
    /// If the entry is present and the offset still matches the active history cursor, the
    /// composer rehydrates the entry immediately. This path intentionally routes through
    /// [`Self::apply_history_entry`] so cursor placement remains aligned with keyboard history
    /// recall semantics.
    pub(crate) fn on_history_entry_response(
        &mut self,
        log_id: u64,
        offset: usize,
        entry: Option<String>,
    ) -> bool {
        match self
            .history
            .on_entry_response(log_id, offset, entry, &self.app_event_tx)
        {
            HistoryEntryResponse::Found(entry) => {
                // Persistent ↑/↓ history is text-only (backwards-compatible and avoids persisting
                // attachments), but local in-session ↑/↓ history can rehydrate elements and image paths.
                self.apply_history_entry(entry);
                true
            }
            HistoryEntryResponse::Search(result) => {
                self.apply_history_search_result(result);
                true
            }
            HistoryEntryResponse::Ignored => false,
        }
    }

    /// Seed local ↑/↓ recall with a user message restored from a replayed/resumed session.
    ///
    /// Replayed submissions are recorded like local submissions but also tracked so the replay
    /// seed can be distinguished from genuinely new in-session input.
    pub(crate) fn record_replayed_user_message_history(&mut self, entry: HistoryEntry) {
        self.history.record_replayed_submission(entry);
    }

}
