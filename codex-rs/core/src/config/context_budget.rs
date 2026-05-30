//! Resolution of the effective [`ContextBudgetMode`] for a loaded config.
//!
//! This is a fork-local concept: `context_budget_mode` may be set via a CLI/
//! session override, a named config profile, or the top-level `config.toml`,
//! and falls back to [`ContextBudgetMode::Slow`] when none is present. Keeping
//! the precedence logic here (rather than inline in the large config builder)
//! reduces merge-conflict surface in the upstream-hot config loader.

use codex_protocol::config_types::ContextBudgetMode;

/// Resolve the effective context-budget mode from its layered sources.
///
/// Precedence, highest first:
/// 1. an explicit override (CLI flag / session override),
/// 2. the selected config profile's `context_budget_mode`,
/// 3. the top-level `config.toml` `context_budget_mode`,
/// 4. otherwise the default ([`ContextBudgetMode::Slow`]).
pub(crate) fn resolve_context_budget_mode(
    override_mode: Option<ContextBudgetMode>,
    profile_mode: Option<ContextBudgetMode>,
    config_mode: Option<ContextBudgetMode>,
) -> ContextBudgetMode {
    override_mode
        .or(profile_mode)
        .or(config_mode)
        .unwrap_or(ContextBudgetMode::Slow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_slow_when_unset() {
        assert_eq!(
            resolve_context_budget_mode(None, None, None),
            ContextBudgetMode::Slow
        );
    }

    #[test]
    fn override_takes_precedence_over_profile_and_config() {
        assert_eq!(
            resolve_context_budget_mode(
                Some(ContextBudgetMode::Standard),
                Some(ContextBudgetMode::Slow),
                Some(ContextBudgetMode::Slow),
            ),
            ContextBudgetMode::Standard
        );
    }

    #[test]
    fn profile_takes_precedence_over_config() {
        assert_eq!(
            resolve_context_budget_mode(
                None,
                Some(ContextBudgetMode::Standard),
                Some(ContextBudgetMode::Slow),
            ),
            ContextBudgetMode::Standard
        );
    }

    #[test]
    fn falls_back_to_config_when_override_and_profile_unset() {
        assert_eq!(
            resolve_context_budget_mode(None, None, Some(ContextBudgetMode::Standard)),
            ContextBudgetMode::Standard
        );
    }
}
