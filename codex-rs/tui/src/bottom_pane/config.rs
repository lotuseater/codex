//! Configuration / option-toggle accessors for the bottom pane.
//!
//! These setters mostly forward a feature flag, command-availability toggle, or
//! presentation option down to the owned `ChatComposer` (and occasionally to the
//! inline previews) and then request a redraw. They are grouped here to keep the
//! `BottomPane` controller in `mod.rs` focused on input routing and view-stack logic.
use super::*;

impl BottomPane {
    pub fn set_skills(&mut self, skills: Option<Vec<SkillMetadata>>) {
        self.composer.set_skill_mentions(skills);
        self.request_redraw();
    }

    /// Update image-paste behavior for the active composer and repaint immediately.
    ///
    /// Callers use this to keep composer affordances aligned with model capabilities.
    pub fn set_image_paste_enabled(&mut self, enabled: bool) {
        self.composer.set_image_paste_enabled(enabled);
        self.request_redraw();
    }

    pub fn set_connectors_snapshot(&mut self, snapshot: Option<ConnectorsSnapshot>) {
        self.composer.set_connector_mentions(snapshot);
        self.request_redraw();
    }

    pub fn set_plugin_mentions(&mut self, plugins: Option<Vec<PluginCapabilitySummary>>) {
        self.composer.set_plugin_mentions(plugins);
        self.request_redraw();
    }

    pub fn set_plugins_command_enabled(&mut self, enabled: bool) {
        self.composer.set_plugins_command_enabled(enabled);
        self.request_redraw();
    }

    pub fn set_mentions_v2_enabled(&mut self, enabled: bool) {
        self.composer.set_mentions_v2_enabled(enabled);
        self.request_redraw();
    }

    pub fn set_collaboration_modes_enabled(&mut self, enabled: bool) {
        self.composer.set_collaboration_modes_enabled(enabled);
        self.request_redraw();
    }

    pub fn set_connectors_enabled(&mut self, enabled: bool) {
        self.composer.set_connectors_enabled(enabled);
    }

    #[cfg(target_os = "windows")]
    pub fn set_windows_degraded_sandbox_active(&mut self, enabled: bool) {
        self.composer.set_windows_degraded_sandbox_active(enabled);
        self.request_redraw();
    }

    pub fn set_collaboration_mode_indicator(
        &mut self,
        indicator: Option<CollaborationModeIndicator>,
    ) {
        self.composer.set_collaboration_mode_indicator(indicator);
        self.request_redraw();
    }

    pub fn set_goal_status_indicator(&mut self, indicator: Option<GoalStatusIndicator>) {
        self.composer.set_goal_status_indicator(indicator);
        self.request_redraw();
    }

    pub fn set_ide_context_active(&mut self, active: bool) {
        self.composer.set_ide_context_active(active);
        self.request_redraw();
    }

    pub fn set_personality_command_enabled(&mut self, enabled: bool) {
        self.composer.set_personality_command_enabled(enabled);
        self.request_redraw();
    }

    pub fn set_service_tier_commands_enabled(&mut self, enabled: bool) {
        self.composer.set_service_tier_commands_enabled(enabled);
        self.request_redraw();
    }

    pub fn set_service_tier_commands(&mut self, commands: Vec<ServiceTierCommand>) {
        self.composer.set_service_tier_commands(commands);
        self.request_redraw();
    }

    pub fn set_goal_command_enabled(&mut self, enabled: bool) {
        self.composer.set_goal_command_enabled(enabled);
        self.request_redraw();
    }

    pub fn set_realtime_conversation_enabled(&mut self, enabled: bool) {
        self.composer.set_realtime_conversation_enabled(enabled);
        self.request_redraw();
    }

    pub fn set_audio_device_selection_enabled(&mut self, enabled: bool) {
        self.composer.set_audio_device_selection_enabled(enabled);
        self.request_redraw();
    }

    pub(crate) fn set_side_conversation_active(&mut self, active: bool) {
        self.composer.set_side_conversation_active(active);
        self.request_redraw();
    }

    pub(crate) fn set_placeholder_text(&mut self, placeholder: String) {
        self.composer.set_placeholder_text(placeholder);
        self.request_redraw();
    }

    /// Update the key hint shown next to queued messages so it matches the
    /// binding that `ChatWidget` actually listens for.
    pub(crate) fn set_queued_message_edit_binding(&mut self, binding: Option<KeyBinding>) {
        self.pending_input_preview.set_edit_binding(binding);
        self.request_redraw();
    }

    pub(crate) fn set_vim_enabled(&mut self, enabled: bool) {
        self.composer.set_vim_enabled(enabled);
        self.request_redraw();
    }

    pub(crate) fn toggle_vim_enabled(&mut self) -> bool {
        let enabled = self.composer.toggle_vim_enabled();
        self.request_redraw();
        enabled
    }
}
