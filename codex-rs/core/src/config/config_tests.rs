use crate::agents_md::DEFAULT_AGENTS_MD_FILENAME;
use crate::agents_md::LOCAL_AGENTS_MD_FILENAME;
use crate::config::edit::ConfigEdit;
use crate::config::edit::ConfigEditsBuilder;
use crate::config::edit::apply_blocking;
use assert_matches::assert_matches;
use codex_config::CONFIG_TOML_FILE;
use codex_config::ConfigLayerEntry;
use codex_config::ProfileV2Name;
use codex_config::RequirementSource;
use codex_config::config_toml::AgentRoleToml;
use codex_config::config_toml::AgentsToml;
use codex_config::config_toml::AutoReviewToml;
use codex_config::config_toml::ConfigToml;
use codex_config::config_toml::FirstMovesModeToml;
use codex_config::config_toml::FirstMovesPrewarmToml;
use codex_config::config_toml::ProjectConfig;
use codex_config::config_toml::RealtimeConfig;
use codex_config::config_toml::RealtimeToml;
use codex_config::config_toml::RealtimeTransport;
use codex_config::config_toml::RealtimeWsMode;
use codex_config::config_toml::RealtimeWsVersion;
use codex_config::config_toml::RepoContextScoutModeToml;
use codex_config::config_toml::ToolsToml;
use codex_config::permissions_toml::FilesystemPermissionToml;
use codex_config::permissions_toml::FilesystemPermissionsToml;
use codex_config::permissions_toml::NetworkDomainPermissionToml;
use codex_config::permissions_toml::NetworkDomainPermissionsToml;
use codex_config::permissions_toml::NetworkMitmActionToml;
use codex_config::permissions_toml::NetworkMitmHookToml;
use codex_config::permissions_toml::NetworkMitmToml;
use codex_config::permissions_toml::NetworkToml;
use codex_config::permissions_toml::PermissionProfileToml;
use codex_config::permissions_toml::PermissionsToml;
use codex_config::permissions_toml::WorkspaceRootsToml;
use codex_config::profile_toml::ConfigProfile;
use codex_config::types::AppToolApproval;
use codex_config::types::ApprovalsReviewer;
use codex_config::types::BlackboardConfig;
use codex_config::types::BundledSkillsConfig;
use codex_config::types::FeedbackConfigToml;
use codex_config::types::HistoryPersistence;
use codex_config::types::McpServerEnvVar;
use codex_config::types::McpServerOAuthConfig;
use codex_config::types::McpServerToolConfig;
use codex_config::types::McpServerTransportConfig;
use codex_config::types::MemoriesConfig;
use codex_config::types::MemoriesToml;
use codex_config::types::ModelAvailabilityNuxConfig;
use codex_config::types::Notice;
use codex_config::types::NotificationCondition;
use codex_config::types::NotificationMethod;
use codex_config::types::Notifications;
use codex_config::types::OtelConfigToml;
use codex_config::types::OtelExporterKind;
use codex_config::types::SandboxWorkspaceWrite;
use codex_config::types::SessionPickerViewMode;
use codex_config::types::SkillsConfig;
use codex_config::types::ToolSuggestDisabledTool;
use codex_config::types::ToolSuggestDiscoverableType;
use codex_config::types::Tui;
use codex_config::types::TuiKeymap;
use codex_config::types::TuiNotificationSettings;
use codex_config::types::TuiPetAnchor;
use codex_config::types::WindowsSandboxModeToml;
use codex_config::types::WindowsToml;
use codex_context_reduction::DEFAULT_TRIGGER_CONTEXT_PERCENT;
use codex_core_plugins::PluginsManager;
use codex_exec_server::LOCAL_FS;
use codex_features::Feature;
use codex_features::FeaturesToml;
use codex_model_provider_info::LMSTUDIO_OSS_PROVIDER_ID;
use codex_model_provider_info::OLLAMA_OSS_PROVIDER_ID;
use codex_model_provider_info::WireApi;
use codex_models_manager::bundled_models_response;
use codex_network_proxy::NetworkMode;
use codex_protocol::config_types::ContextBudgetMode;
use codex_protocol::config_types::SERVICE_TIER_DEFAULT_REQUEST_VALUE;
use codex_protocol::config_types::ServiceTier;
use codex_protocol::models::ActivePermissionProfile;
use codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS;
use codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_READ_ONLY;
use codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_WORKSPACE;
use codex_protocol::models::ManagedFileSystemPermissions;
use codex_protocol::models::PermissionProfile;
use codex_protocol::models::SandboxEnforcement;
use codex_protocol::permissions::FileSystemAccessMode;
use codex_protocol::permissions::FileSystemPath;
use codex_protocol::permissions::FileSystemSandboxEntry;
use codex_protocol::permissions::FileSystemSandboxPolicy;
use codex_protocol::permissions::FileSystemSpecialPath;
use codex_protocol::permissions::NetworkSandboxPolicy;
use codex_protocol::protocol::NetworkAccess;
use codex_protocol::protocol::RealtimeVoice;
use codex_protocol::protocol::SandboxPolicy;
use codex_repo_context_scout::RepoContextScoutConfig;
use serde::Deserialize;
use tempfile::tempdir;

use super::*;
use codex_test_support_lightweight::PathBufExt;
use codex_test_support_lightweight::PathExt;
use codex_test_support_lightweight::TempDirExt;
use codex_test_support_lightweight::test_absolute_path;
use indexmap::IndexMap;
use pretty_assertions::assert_eq;
use rmcp::model::ElicitationCapability;
use rmcp::model::FormElicitationCapability;
use rmcp::model::UrlElicitationCapability;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tempfile::TempDir;

// Test submodules (split from the original monolith). Shared fixtures/helpers
// live in `common`; each child opens with `use super::*;`.
#[path = "config_tests/common.rs"]
mod common;

#[path = "config_tests/loading_and_parsing.rs"]
mod loading_and_parsing;
#[path = "config_tests/provider_and_tui_basics.rs"]
mod provider_and_tui_basics;
#[path = "config_tests/permission_profiles_a.rs"]
mod permission_profiles_a;
#[path = "config_tests/network_proxy_feature.rs"]
mod network_proxy_feature;
#[path = "config_tests/permission_profiles_b.rs"]
mod permission_profiles_b;
#[path = "config_tests/permission_profiles_c.rs"]
mod permission_profiles_c;
#[path = "config_tests/permission_profiles_d.rs"]
mod permission_profiles_d;
#[path = "config_tests/tui_misc_and_sandbox.rs"]
mod tui_misc_and_sandbox;
#[path = "config_tests/mcp_filter.rs"]
mod mcp_filter;
#[path = "config_tests/mcp_rebuild_and_to_config.rs"]
mod mcp_rebuild_and_to_config;
#[path = "config_tests/runtime_auth_and_websearch.rs"]
mod runtime_auth_and_websearch;
#[path = "config_tests/features_and_mcp_loading.rs"]
mod features_and_mcp_loading;
#[path = "config_tests/mcp_to_config_serialization.rs"]
mod mcp_to_config_serialization;
#[path = "config_tests/mcp_replace_serialization.rs"]
mod mcp_replace_serialization;
#[path = "config_tests/set_model_and_guardian.rs"]
mod set_model_and_guardian;
#[path = "config_tests/agent_role_files.rs"]
mod agent_role_files;
#[path = "config_tests/agent_role_discovery.rs"]
mod agent_role_discovery;
#[path = "config_tests/precedence_otel_service_tier.rs"]
mod precedence_otel_service_tier;
#[path = "config_tests/precedence_fixtures.rs"]
mod precedence_fixtures;
#[path = "config_tests/oss_and_sandbox_derive.rs"]
mod oss_and_sandbox_derive;
#[path = "config_tests/requirements_fallbacks.rs"]
mod requirements_fallbacks;
#[path = "config_tests/features_and_approvals.rs"]
mod features_and_approvals;
#[path = "config_tests/multi_agent_v2.rs"]
mod multi_agent_v2;
#[path = "config_tests/tool_suggest_and_realtime.rs"]
mod tool_suggest_and_realtime;
#[path = "config_tests/tui_notifications.rs"]
mod tui_notifications;
