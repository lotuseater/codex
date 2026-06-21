//! Initial-context assembly for [`Session`].
//!
//! Moved verbatim from `session/mod.rs` (pure code-motion). Builds the developer
//! and contextual-user instruction items injected at the start of a turn/thread.

use super::multi_agents;
use super::*;
use crate::context::UserInstructions;

impl Session {
    pub(crate) async fn build_initial_context(
        &self,
        turn_context: &TurnContext,
    ) -> Vec<ResponseItem> {
        let mut developer_sections = Vec::<String>::with_capacity(8);
        let mut contextual_user_sections = Vec::<String>::with_capacity(2);
        let mut separate_developer_sections = Vec::<String>::new();
        let (reference_context_item, previous_turn_settings, base_instructions, session_source) = {
            let state = self.state.lock().await;
            (
                state.reference_context_item(),
                state.previous_turn_settings(),
                state.session_configuration.base_instructions.clone(),
                state.session_configuration.session_source.clone(),
            )
        };
        if let Some(model_switch_message) =
            crate::context_manager::updates::build_model_instructions_update_item(
                previous_turn_settings.as_ref(),
                turn_context,
            )
        {
            developer_sections.push(model_switch_message);
        }
        if turn_context.config.include_permissions_instructions {
            developer_sections.push(
                PermissionsInstructions::from_permission_profile(
                    &turn_context.permission_profile,
                    turn_context.approval_policy.value(),
                    turn_context.config.approvals_reviewer,
                    self.services.exec_policy.current().as_ref(),
                    #[allow(deprecated)]
                    &turn_context.cwd,
                    turn_context
                        .features
                        .enabled(Feature::ExecPermissionApprovals),
                    turn_context
                        .features
                        .enabled(Feature::RequestPermissionsTool),
                )
                .render(),
            );
        }
        let separate_guardian_developer_message =
            crate::guardian::is_guardian_reviewer_source(&session_source);
        // Keep the guardian policy prompt out of the aggregated developer bundle so it
        // stays isolated as its own top-level developer message for guardian subagents.
        if !separate_guardian_developer_message
            && let Some(developer_instructions) = turn_context.developer_instructions.as_deref()
            && !developer_instructions.is_empty()
        {
            developer_sections.push(developer_instructions.to_string());
        }
        // Memory tool developer instructions are contributed by the `ext/memories`
        // extension through the context-contributor mechanism below (upstream moved the
        // prompt builder into that extension in #24558), so no direct call is needed here.
        // Add developer instructions from collaboration_mode if they exist and are non-empty
        if turn_context.config.include_collaboration_mode_instructions
            && let Some(collab_instructions) =
                CollaborationModeInstructions::from_collaboration_mode(
                    &turn_context.collaboration_mode,
                )
        {
            developer_sections.push(collab_instructions.render());
        }
        let is_first_turn = reference_context_item.is_none();
        let render_action_optimization_instructions =
            match turn_context.config.action_optimization_instructions.mode {
                crate::config::ActionOptimizationInstructionsMode::Off => false,
                crate::config::ActionOptimizationInstructionsMode::Plan => {
                    turn_context.collaboration_mode.mode
                        == codex_protocol::config_types::ModeKind::Plan
                }
                crate::config::ActionOptimizationInstructionsMode::FirstTurn => is_first_turn,
                // Reserved for a future tool-turn update hook; initial context
                // rendering intentionally stays off for this mode.
                crate::config::ActionOptimizationInstructionsMode::ToolTurn => false,
                crate::config::ActionOptimizationInstructionsMode::Always => true,
            };
        if render_action_optimization_instructions
            && let Some(instructions) = crate::context::ActionOptimizationInstructions::from_config(
                &turn_context.config.action_optimization_instructions,
            )
        {
            developer_sections.push(instructions.render());
        }
        if matches!(
            turn_context.config.batch_mini_programming_instructions.mode,
            crate::config::BatchMiniProgrammingInstructionsMode::Always
        ) && turn_context.tools_config.workflow_batch_enabled
            && turn_context.tools_config.environment_mode.has_environment()
        {
            developer_sections.push(
                BatchMiniProgrammingInstructions::from_config(
                    &turn_context.config.batch_mini_programming_instructions,
                )
                .render(),
            );
        }
        if let Some(realtime_update) = crate::context_manager::updates::build_initial_realtime_item(
            reference_context_item.as_ref(),
            previous_turn_settings.as_ref(),
            turn_context,
        ) {
            developer_sections.push(realtime_update);
        }
        if self.features.enabled(Feature::Personality)
            && let Some(personality) = turn_context.personality
        {
            let model_info = turn_context.model_info.clone();
            let has_baked_personality = model_info.supports_personality()
                && base_instructions == model_info.get_model_instructions(Some(personality));
            if !has_baked_personality
                && let Some(personality_message) =
                    crate::context_manager::updates::personality_message_for(
                        &model_info,
                        personality,
                    )
            {
                developer_sections
                    .push(PersonalitySpecInstructions::new(personality_message).render());
            }
        }
        if turn_context.config.include_apps_instructions && turn_context.apps_enabled() {
            let mcp_connection_manager = self.services.mcp_connection_manager.load_full();
            let accessible_and_enabled_connectors =
                connectors::list_accessible_and_enabled_connectors_from_manager(
                    &mcp_connection_manager,
                    &turn_context.config,
                )
                .await;
            if let Some(apps_instructions) =
                AppsInstructions::from_connectors(&accessible_and_enabled_connectors)
            {
                developer_sections.push(apps_instructions.render());
            }
        }
        if turn_context.config.include_skill_instructions {
            let available_skills = build_available_skills(
                &turn_context.turn_skills.outcome,
                default_skill_metadata_budget(turn_context.model_info.context_window),
                SkillRenderSideEffects::ThreadStart {
                    session_telemetry: &self.services.session_telemetry,
                },
            );
            if let Some(available_skills) = available_skills {
                let warning_message = available_skills.warning_message.clone();
                let skills_instructions = AvailableSkillsInstructions::from(available_skills);
                if let Some(warning_message) = warning_message {
                    self.send_event_raw(Event {
                        id: String::new(),
                        msg: EventMsg::Warning(WarningEvent {
                            message: warning_message,
                        }),
                    })
                    .await;
                }
                developer_sections.push(skills_instructions.render());
            }
        }
        let loaded_plugins = self
            .services
            .plugins_manager
            .plugins_for_config(&turn_context.config.plugins_config_input())
            .await;
        if let Some(plugin_instructions) =
            AvailablePluginsInstructions::from_plugins(loaded_plugins.capability_summaries())
        {
            developer_sections.push(plugin_instructions.render());
        }
        let context_contributors = self.services.extensions.context_contributors().to_vec();
        for contributor in context_contributors {
            for fragment in contributor
                .contribute_thread_context(
                    &self.services.session_extension_data,
                    &self.services.thread_extension_data,
                )
                .await
            {
                match fragment.slot() {
                    PromptSlot::DeveloperPolicy | PromptSlot::DeveloperCapabilities => {
                        developer_sections.push(fragment.text().to_string());
                    }
                    PromptSlot::ContextualUser => {
                        contextual_user_sections.push(fragment.text().to_string());
                    }
                    PromptSlot::SeparateDeveloper => {
                        separate_developer_sections.push(fragment.text().to_string());
                    }
                }
            }
        }
        if let Some(user_instructions) = turn_context.user_instructions.as_deref() {
            contextual_user_sections.push(
                UserInstructions {
                    text: user_instructions.to_string(),
                    #[allow(deprecated)]
                    directory: Some(turn_context.cwd.to_string_lossy().into_owned()),
                }
                .render(),
            );
        }
        if turn_context.config.include_environment_context {
            let shell = self.user_shell();
            let subagents = self
                .services
                .agent_control
                .format_environment_context_subagents(self.thread_id)
                .await;
            contextual_user_sections.push(
                crate::context::EnvironmentContext::from_turn_context(turn_context, shell.as_ref())
                    .with_subagents(subagents)
                    .render(),
            );
        }

        let multi_agent_v2_usage_hint_text =
            multi_agents::usage_hint_text(turn_context, &session_source);

        let mut items = Vec::with_capacity(4);
        if let Some(developer_message) =
            crate::context_manager::updates::build_developer_update_item(developer_sections)
        {
            items.push(developer_message);
        }
        for section in separate_developer_sections {
            if let Some(developer_message) =
                crate::context_manager::updates::build_developer_update_item(vec![section])
            {
                items.push(developer_message);
            }
        }
        if let Some(usage_hint_text) = multi_agent_v2_usage_hint_text
            && let Some(usage_hint_message) =
                crate::context_manager::updates::build_developer_update_item(vec![
                    usage_hint_text.to_string(),
                ])
        {
            items.push(usage_hint_message);
        }
        if let Some(contextual_user_message) =
            crate::context_manager::updates::build_contextual_user_message(contextual_user_sections)
        {
            items.push(contextual_user_message);
        }
        // Emit the guardian policy prompt as a separate developer item so the guardian
        // subagent sees a distinct, easy-to-audit instruction block.
        if separate_guardian_developer_message
            && let Some(developer_instructions) = turn_context.developer_instructions.as_deref()
            && !developer_instructions.is_empty()
            && let Some(guardian_developer_message) =
                crate::context_manager::updates::build_developer_update_item(vec![
                    developer_instructions.to_string(),
                ])
        {
            items.push(guardian_developer_message);
        }
        items
    }
}
