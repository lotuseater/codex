//! Serializable session state DTOs.
//!
//! This crate owns persisted or transferred state shapes without depending on
//! any concrete runtime, store, transport, or UI implementation.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

use std::collections::BTreeMap;
use std::path::PathBuf;

pub use codex_session_api::PreviousTurnSettings;
use codex_session_api::SessionIdentity;
use codex_session_api::SessionLifecycleState;
use serde::Deserialize;
use serde::Serialize;

/// Workspace roots active for a session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionWorkspaceRoots {
    pub cwd: PathBuf,
    pub runtime_workspace_roots: Vec<PathBuf>,
    pub profile_workspace_roots: Vec<PathBuf>,
}

impl SessionWorkspaceRoots {
    /// Creates workspace roots from the current directory and root lists.
    pub fn new(
        cwd: PathBuf,
        runtime_workspace_roots: Vec<PathBuf>,
        profile_workspace_roots: Vec<PathBuf>,
    ) -> Self {
        Self {
            cwd,
            runtime_workspace_roots,
            profile_workspace_roots,
        }
    }

    /// Applies a partial workspace-root update, preserving omitted fields.
    pub fn apply_update(&mut self, update: SessionWorkspaceRootsUpdate) {
        if let Some(cwd) = update.cwd {
            self.cwd = cwd;
        }
        if let Some(runtime_workspace_roots) = update.runtime_workspace_roots {
            self.runtime_workspace_roots = runtime_workspace_roots;
        }
        if let Some(profile_workspace_roots) = update.profile_workspace_roots {
            self.profile_workspace_roots = profile_workspace_roots;
        }
    }
}

/// Partial update for session workspace roots.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionWorkspaceRootsUpdate {
    pub cwd: Option<PathBuf>,
    pub runtime_workspace_roots: Option<Vec<PathBuf>>,
    pub profile_workspace_roots: Option<Vec<PathBuf>>,
}

/// Update to the service tier selected for a session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum SessionServiceTierUpdate {
    Preserve,
    Clear,
    Set(String),
}

impl Default for SessionServiceTierUpdate {
    fn default() -> Self {
        Self::Preserve
    }
}

/// Environment selection active for a session.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionEnvironmentState {
    pub selected_environments: Vec<String>,
}

impl SessionEnvironmentState {
    /// Creates an environment state from the selected environment names.
    pub fn new(selected_environments: Vec<String>) -> Self {
        Self {
            selected_environments,
        }
    }

    /// Applies an environment update, preserving the current state when asked.
    pub fn apply_update(&mut self, update: SessionEnvironmentUpdate) {
        match update {
            SessionEnvironmentUpdate::Preserve => {}
            SessionEnvironmentUpdate::Disable => self.selected_environments.clear(),
            SessionEnvironmentUpdate::Replace(selected_environments) => {
                self.selected_environments = selected_environments;
            }
        }
    }
}

/// Update to session environment selection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum SessionEnvironmentUpdate {
    Preserve,
    Disable,
    Replace(Vec<String>),
}

impl Default for SessionEnvironmentUpdate {
    fn default() -> Self {
        Self::Preserve
    }
}

/// Session settings that can be persisted, transferred, or patched without
/// depending on a concrete runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionSettingsSnapshot {
    pub workspace_roots: SessionWorkspaceRoots,
    pub service_tier: Option<String>,
    pub environment: SessionEnvironmentState,
    pub config_metadata: BTreeMap<String, String>,
}

impl SessionSettingsSnapshot {
    /// Creates settings state with no selected service tier, environment, or
    /// metadata.
    pub fn new(workspace_roots: SessionWorkspaceRoots) -> Self {
        Self {
            workspace_roots,
            service_tier: None,
            environment: SessionEnvironmentState::default(),
            config_metadata: BTreeMap::new(),
        }
    }

    /// Applies a partial settings update, preserving omitted settings.
    pub fn apply_update(&mut self, update: SessionSettingsUpdate) {
        if let Some(workspace_roots) = update.workspace_roots {
            self.workspace_roots.apply_update(workspace_roots);
        }

        match update.service_tier {
            SessionServiceTierUpdate::Preserve => {}
            SessionServiceTierUpdate::Clear => self.service_tier = None,
            SessionServiceTierUpdate::Set(service_tier) => self.service_tier = Some(service_tier),
        }

        self.environment.apply_update(update.environment);

        if let Some(config_metadata) = update.config_metadata {
            self.config_metadata = config_metadata;
        }
    }
}

/// Partial update for session settings.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionSettingsUpdate {
    pub workspace_roots: Option<SessionWorkspaceRootsUpdate>,
    pub service_tier: SessionServiceTierUpdate,
    pub environment: SessionEnvironmentUpdate,
    pub config_metadata: Option<BTreeMap<String, String>>,
}

/// Compact snapshot of session state for storage or handoff.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionStateSnapshot {
    pub identity: SessionIdentity,
    pub lifecycle: SessionLifecycleState,
    pub metadata: BTreeMap<String, String>,
    pub pending_input_count: usize,
}

impl SessionStateSnapshot {
    /// Creates a snapshot with empty metadata and no pending inputs.
    pub fn new(identity: SessionIdentity, lifecycle: SessionLifecycleState) -> Self {
        Self {
            identity,
            lifecycle,
            metadata: BTreeMap::new(),
            pending_input_count: 0,
        }
    }
}
