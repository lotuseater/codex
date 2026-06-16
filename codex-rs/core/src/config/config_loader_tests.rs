use crate::config::ConfigBuilder;
use crate::config::ConfigOverrides;
use crate::config::ConstraintError;
use codex_config::CONFIG_TOML_FILE;
use codex_config::CloudRequirementsLoadError;
use codex_config::CloudRequirementsLoader;
use codex_config::ConfigError;
use codex_config::ConfigLayerEntry;
use codex_config::ConfigLayerSource;
use codex_config::ConfigLayerStackOrdering;
use codex_config::ConfigLoadError;
use codex_config::ConfigRequirements;
use codex_config::ConfigRequirementsToml;
use codex_config::ConfigRequirementsWithSources;
use codex_config::FilesystemDenyReadPattern;
use codex_config::LoaderOverrides;
use codex_config::RequirementSource;
use codex_config::SessionThreadConfig;
use codex_config::StaticThreadConfigLoader;
use codex_config::ThreadConfigSource;
use codex_config::config_error_from_ignored_toml_fields;
use codex_config::config_error_from_toml;
use codex_config::config_toml::ConfigToml;
use codex_config::config_toml::ProjectConfig;
use codex_config::loader::load_config_layers_state;
use codex_config::loader::load_requirements_toml;
use codex_exec_server::LOCAL_FS;
use codex_protocol::config_types::TrustLevel;
use codex_protocol::config_types::WebSearchMode;
use codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_READ_ONLY;
use codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_WORKSPACE;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use codex_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;
use tempfile::tempdir;
use toml::Value as TomlValue;

#[path = "config_loader_tests/common.rs"]
mod common;
#[path = "config_loader_tests/errors_and_schema.rs"]
mod errors_and_schema;
#[path = "config_loader_tests/layers.rs"]
mod layers;
#[path = "config_loader_tests/macos_managed_preferences.rs"]
mod macos_managed_preferences;
#[path = "config_loader_tests/paths_and_instructions.rs"]
mod paths_and_instructions;
#[path = "config_loader_tests/permissions.rs"]
mod permissions;
#[path = "config_loader_tests/project_layers.rs"]
mod project_layers;
#[path = "config_loader_tests/project_trust.rs"]
mod project_trust;
#[path = "config_loader_tests/project_worktree_hooks.rs"]
mod project_worktree_hooks;
#[path = "config_loader_tests/requirements_exec_policy_tests.rs"]
mod requirements_exec_policy_tests;
#[path = "config_loader_tests/requirements_toml.rs"]
mod requirements_toml;
