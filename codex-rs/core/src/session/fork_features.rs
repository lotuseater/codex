//! Bundles this fork's session-scoped feature state.
//!
//! The fork adds three session-scoped features on top of upstream Codex:
//! `collaboration_mode`, `context_budget_mode`, and `personality`. Historically
//! these were threaded as three independent sibling fields at every carrier hop
//! (`SessionConfiguration`, `SessionSettingsUpdate`, `TurnContext`, ...), which
//! scattered fork-specific concerns across the highest-touch core paths.
//!
//! [`ForkFeaturesState`] gathers the three values into a single unit so they can
//! be threaded as one bundle. [`ForkFeaturesUpdate`] is the optional-delta
//! counterpart used when merging session-settings updates.
//!
//! This is intentionally a plain module of `codex-core` (not a new crate): the
//! underlying types already live in the `codex-config-types` leaf crate, so the
//! bundle is pure core-session glue with no new layering benefit from a crate.

use codex_protocol::config_types::CollaborationMode;
use codex_protocol::config_types::ContextBudgetMode;
use codex_protocol::config_types::Personality;
use codex_protocol::config_types::ReasoningEffort;

/// Session-scoped state for the three fork-specific features, threaded as one
/// unit instead of three sibling fields.
#[derive(Clone, Debug)]
pub(crate) struct ForkFeaturesState {
    pub(crate) collaboration_mode: CollaborationMode,
    pub(crate) context_budget_mode: ContextBudgetMode,
    pub(crate) personality: Option<Personality>,
}

/// Optional-delta counterpart of [`ForkFeaturesState`] used by
/// `SessionSettingsUpdate` and wire mapping. Every field is `None` by default,
/// meaning "leave the corresponding value unchanged".
#[derive(Clone, Debug, Default)]
pub(crate) struct ForkFeaturesUpdate {
    pub(crate) collaboration_mode: Option<CollaborationMode>,
    pub(crate) context_budget_mode: Option<ContextBudgetMode>,
    pub(crate) personality: Option<Personality>,
}

impl ForkFeaturesState {
    /// Build a bundle from the three constituent values.
    pub(crate) fn new(
        collaboration_mode: CollaborationMode,
        context_budget_mode: ContextBudgetMode,
        personality: Option<Personality>,
    ) -> Self {
        Self {
            collaboration_mode,
            context_budget_mode,
            personality,
        }
    }

    /// Merge an optional-delta update in place: each `Some` field overwrites the
    /// corresponding value, each `None` leaves it unchanged.
    pub(crate) fn apply(&mut self, update: ForkFeaturesUpdate) {
        if let Some(collaboration_mode) = update.collaboration_mode {
            self.collaboration_mode = collaboration_mode;
        }
        if let Some(context_budget_mode) = update.context_budget_mode {
            self.context_budget_mode = context_budget_mode;
        }
        if let Some(personality) = update.personality {
            self.personality = Some(personality);
        }
    }

    /// Convenience projection of the active model, sourced from the
    /// collaboration mode. Mirrors the derivation sites that flatten the
    /// collaboration mode onto `Config`/`lock_config`.
    pub(crate) fn model(&self) -> &str {
        self.collaboration_mode.model()
    }

    /// Convenience projection of the active reasoning effort, sourced from the
    /// collaboration mode.
    pub(crate) fn reasoning_effort(&self) -> Option<ReasoningEffort> {
        self.collaboration_mode.reasoning_effort()
    }

    /// Borrow the bundled collaboration mode.
    pub(crate) fn collaboration_mode(&self) -> &CollaborationMode {
        &self.collaboration_mode
    }

    /// Copy the bundled context budget mode.
    pub(crate) fn context_budget_mode(&self) -> ContextBudgetMode {
        self.context_budget_mode
    }

    /// Copy the bundled personality preference.
    pub(crate) fn personality(&self) -> Option<Personality> {
        self.personality
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::config_types::ModeKind;
    use codex_protocol::config_types::Settings;

    fn collaboration_mode_with(model: &str, effort: Option<ReasoningEffort>) -> CollaborationMode {
        CollaborationMode {
            mode: ModeKind::Default,
            settings: Settings {
                model: model.to_string(),
                reasoning_effort: effort,
                developer_instructions: None,
            },
        }
    }

    #[test]
    fn apply_overwrites_only_some_fields() {
        let mut state = ForkFeaturesState::new(
            collaboration_mode_with("gpt-5.2-codex", Some(ReasoningEffort::Low)),
            ContextBudgetMode::Slow,
            Some(Personality::Friendly),
        );

        state.apply(ForkFeaturesUpdate {
            collaboration_mode: None,
            context_budget_mode: Some(ContextBudgetMode::Standard),
            personality: Some(Personality::Pragmatic),
        });

        // Untouched: collaboration_mode (update was None).
        assert_eq!(state.model(), "gpt-5.2-codex");
        assert_eq!(state.reasoning_effort(), Some(ReasoningEffort::Low));
        // Overwritten by the update.
        assert_eq!(state.context_budget_mode(), ContextBudgetMode::Standard);
        assert_eq!(state.personality(), Some(Personality::Pragmatic));
    }

    #[test]
    fn apply_with_empty_update_is_noop() {
        let mut state = ForkFeaturesState::new(
            collaboration_mode_with("gpt-5.2-codex", None),
            ContextBudgetMode::Standard,
            None,
        );

        state.apply(ForkFeaturesUpdate::default());

        assert_eq!(state.model(), "gpt-5.2-codex");
        assert_eq!(state.context_budget_mode(), ContextBudgetMode::Standard);
        assert_eq!(state.personality(), None);
    }

    #[test]
    fn apply_can_set_collaboration_mode() {
        let mut state = ForkFeaturesState::new(
            collaboration_mode_with("gpt-5.2-codex", None),
            ContextBudgetMode::Slow,
            None,
        );

        state.apply(ForkFeaturesUpdate {
            collaboration_mode: Some(collaboration_mode_with(
                "gpt-5.4",
                Some(ReasoningEffort::High),
            )),
            context_budget_mode: None,
            personality: None,
        });

        assert_eq!(state.model(), "gpt-5.4");
        assert_eq!(state.reasoning_effort(), Some(ReasoningEffort::High));
    }
}
