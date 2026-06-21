//! Configuration enums/structs shared across crates.
//!
//! ## Single canonical owner: `codex_config_types`
//!
//! These types are owned by the `codex_config_types` crate. Historically the fork
//! re-exported the whole crate here (`pub use codex_config_types::*;`) so that the
//! `codex_protocol::config_types::X` import path kept resolving to the single shared
//! type. The upstream merge replaced that stub with full enum *redefinitions*, which
//! made `codex_protocol::config_types::X` and `codex_config_types::X` two *distinct*
//! Rust types and caused ~100 `E0308`/`E0277`/`E0603` mismatches in `codex-core`.
//!
//! This module restores the fork's decoupling by **selectively re-exporting** the
//! shared types from their canonical owner, so `codex_protocol::config_types::X`
//! once again *is* `codex_config_types::X` (one type, one wire format). The
//! re-export is the fork's own design — `codex_config_types` is the owner and
//! `codex_protocol::config_types` is a stable alias path — not a foreign re-export
//! crutch.
//!
//! A blanket `pub use codex_config_types::*;` is intentionally **not** used: a few
//! types below are protocol-domain and differ from (or do not exist in) the owner
//! crate — most notably [`ProfileV2Name`], which is a validated newtype here but a
//! plain `String` alias in `codex_config_types`. Re-exporting selectively keeps
//! those protocol-only definitions authoritative.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;
use strum_macros::Display;
use ts_rs::TS;

// ---------------------------------------------------------------------------
// Canonical shared types — re-exported from their owning crate `codex_config_types`.
//
// `codex_protocol::config_types::X` resolves to `codex_config_types::X` (the single
// shared type with the fork's shipped serde wire format). NOTE on
// `ApprovalsReviewer`: the fork ships the `codex_config_types` wire name
// `"guardian_subagent"` for the `AutoReview` variant (both variants `alias` the
// `"auto_review"` spelling, so deserialization accepts either). Re-exporting the
// owner's definition preserves that shipped wire format.
// ---------------------------------------------------------------------------
pub use codex_config_types::AltScreenMode;
pub use codex_config_types::ApprovalsReviewer;
pub use codex_config_types::AutoCompactTokenLimitScope;
pub use codex_config_types::CollaborationMode;
pub use codex_config_types::CollaborationModeMask;
pub use codex_config_types::ConfigLayerSource;
pub use codex_config_types::ContextBudgetMode;
pub use codex_config_types::EnvironmentVariablePattern;
pub use codex_config_types::ForcedLoginMethod;
pub use codex_config_types::ModeKind;
pub use codex_config_types::ModelProviderAuthInfo;
pub use codex_config_types::Personality;
pub use codex_config_types::ReasoningEffort;
pub use codex_config_types::ReasoningSummary;
pub use codex_config_types::SERVICE_TIER_DEFAULT_REQUEST_VALUE;
pub use codex_config_types::SandboxMode;
pub use codex_config_types::ServiceTier;
pub use codex_config_types::Settings;
pub use codex_config_types::ShellEnvironmentPolicy;
pub use codex_config_types::ShellEnvironmentPolicyInherit;
pub use codex_config_types::TUI_VISIBLE_COLLABORATION_MODES;
pub use codex_config_types::TrustLevel;
pub use codex_config_types::Verbosity;
pub use codex_config_types::WebSearchConfig;
pub use codex_config_types::WebSearchContextSize;
pub use codex_config_types::WebSearchFilters;
pub use codex_config_types::WebSearchLocation;
pub use codex_config_types::WebSearchMode;
pub use codex_config_types::WebSearchToolConfig;
pub use codex_config_types::WebSearchUserLocation;
pub use codex_config_types::WebSearchUserLocationType;
pub use codex_config_types::WindowsSandboxLevel;

// ---------------------------------------------------------------------------
// Protocol-only types — authoritative HERE (not present in, or differing from,
// `codex_config_types`). Keep these defined locally.
// ---------------------------------------------------------------------------

/// Validated plain profile-v2 name used to select `$CODEX_HOME/<name>.config.toml`.
///
/// Protocol-only: `codex_config_types` exposes `ProfileV2Name` as a plain
/// `String` alias; the validated newtype lives here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileV2Name(String);

impl ProfileV2Name {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProfileV2NameParseError {
    value: String,
}

impl fmt::Display for ProfileV2NameParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid --profile value `{}`; pass a plain name such as `work`",
            self.value
        )
    }
}

impl std::error::Error for ProfileV2NameParseError {}

impl FromStr for ProfileV2Name {
    type Err = ProfileV2NameParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(ProfileV2NameParseError {
                value: value.to_string(),
            });
        }

        Ok(Self(value.to_string()))
    }
}

impl Deref for ProfileV2Name {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for ProfileV2Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Controls whether the model should only spawn sub-agents after an explicit
/// user request or may delegate proactively when doing so would help.
///
/// Protocol-only: not present in `codex_config_types`.
#[derive(
    Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Display, JsonSchema, TS, Default,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum MultiAgentMode {
    #[default]
    ExplicitRequestOnly,
    Proactive,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn profile_v2_name_rejects_paths_and_empty_names() {
        assert_eq!(
            ProfileV2Name::from_str("../foo"),
            Err(ProfileV2NameParseError {
                value: "../foo".to_string(),
            }),
            "dots and slashes are disallowed to prevent reading arbitrary files"
        );
        assert_eq!(
            ProfileV2Name::from_str(""),
            Err(ProfileV2NameParseError {
                value: String::new(),
            }),
            "profile name cannot be empty"
        );
    }
}
