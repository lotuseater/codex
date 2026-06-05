use crate::agents_md::AgentsMdManager;
pub use crate::agents_md::LoadedAgentsMd;
use crate::config::edit::ConfigEdit;
use crate::config::edit::ConfigEditsBuilder;
use crate::path_utils::normalize_for_native_workdir;
use crate::unified_exec::DEFAULT_MAX_BACKGROUND_TERMINAL_TIMEOUT_MS;
use crate::unified_exec::MIN_EMPTY_YIELD_TIME_MS;
use crate::windows_sandbox::WindowsSandboxLevelExt;
use crate::windows_sandbox::resolve_windows_sandbox_mode;
use crate::windows_sandbox::resolve_windows_sandbox_private_desktop;
use codex_compaction_policy::DEFAULT_TRIGGER_CONTEXT_PERCENT;
use codex_config::CloudConfigBundleLoader;
use codex_config::CloudRequirementsLoader;
use codex_config::ConfigLayerSource;
use codex_config::ConfigLayerStack;
use codex_config::ConfigLayerStackOrdering;
use codex_config::ConfigRequirements;
use codex_config::ConfigRequirementsToml;
use codex_config::ConstrainedWithSource;
use codex_config::FeatureRequirementsToml;
use codex_config::McpServerIdentity;
use codex_config::McpServerRequirement;
use codex_config::PluginRequirementsToml;
use codex_config::ResidencyRequirement;
use codex_config::SandboxModeRequirement;
use codex_config::Sourced;
use codex_config::ThreadConfigLoader;
use codex_config::config_toml::ConfigLockfileToml;
use codex_config::config_toml::ConfigToml;
use codex_config::config_toml::DEFAULT_PROJECT_DOC_MAX_BYTES;
use codex_config::config_toml::DesktopAutomationToml;
use codex_config::config_toml::FirstMovesModeToml;
use codex_config::config_toml::FirstMovesPrewarmToml;
use codex_config::config_toml::FirstMovesToml;
use codex_config::config_toml::ProjectConfig;
use codex_config::config_toml::RealtimeAudioConfig;
use codex_config::config_toml::RealtimeConfig;
use codex_config::config_toml::RepoContextScoutModeToml;
use codex_config::config_toml::RepoContextScoutToml;
use codex_config::config_toml::ThreadStoreToml;
use codex_config::config_toml::validate_model_providers;
use codex_config::loader::load_config_layers_state;
use codex_config::loader::project_trust_key;
use codex_config::permissions_toml::PermissionsToml;
use codex_config::profile_toml::ConfigProfile;
use codex_config::sandbox_mode_requirement_for_permission_profile;
use codex_config::types::ApprovalsReviewer;
use codex_config::types::AuthCredentialsStoreMode;
use codex_config::types::BlackboardConfig;
use codex_config::types::History;
use codex_config::types::McpServerConfig;
use codex_config::types::McpServerDisabledReason;
use codex_config::types::McpServerTransportConfig;
use codex_config::types::MemoriesConfig;
use codex_config::types::ModelAvailabilityNuxConfig;
use codex_config::types::Notice;
use codex_config::types::OAuthCredentialsStoreMode;
use codex_config::types::PromptReductionModeToml;
use codex_config::types::SessionPickerViewMode;
use codex_config::types::ToolSuggestConfig;
use codex_config::types::ToolSuggestDisabledTool;
use codex_config::types::ToolSuggestDiscoverable;
use codex_config::types::TuiKeymap;
use codex_config::types::TuiNotificationSettings;
use codex_config::types::TuiPetAnchor;
use codex_config::types::UriBasedFileOpener;
use codex_config::types::WindowsSandboxModeToml;
use codex_core_plugins::PluginsConfigInput;
use codex_exec_server::ExecutorFileSystem;
use codex_exec_server::LOCAL_FS;
use codex_features::AppsMcpPathOverrideConfigToml;
use codex_features::CodeModeConfigToml;
use codex_features::Feature;
use codex_features::FeatureConfigSource;
use codex_features::FeatureOverrides;
use codex_features::FeatureToml;
use codex_features::Features;
use codex_features::FeaturesToml;
use codex_features::MultiAgentV2ConfigToml;
use codex_features::NetworkProxyConfigToml;
use codex_first_moves::FirstMovesConfig;
use codex_first_moves::FirstMovesMode;
use codex_first_moves::FirstMovesPrewarm;
use codex_git_utils::resolve_root_git_project_for_trust;
use codex_install_context::InstallContext;
use codex_login::AuthManagerConfig;
use codex_mcp::McpConfig;
use codex_memories_read::memory_root;
use codex_model_provider_info::LEGACY_OLLAMA_CHAT_PROVIDER_ID;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::OLLAMA_CHAT_PROVIDER_REMOVED_ERROR;
use codex_model_provider_info::built_in_model_providers;
use codex_model_provider_info::merge_configured_model_providers;
use codex_models_manager::ModelsManagerConfig;
use codex_protocol::config_types::AltScreenMode;
use codex_protocol::config_types::ContextBudgetMode;
use codex_protocol::config_types::ForcedLoginMethod;
use codex_protocol::config_types::Personality;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::config_types::SERVICE_TIER_DEFAULT_REQUEST_VALUE;
use codex_protocol::config_types::SandboxMode;
use codex_protocol::config_types::ServiceTier;
use codex_protocol::config_types::ShellEnvironmentPolicy;
use codex_protocol::config_types::TrustLevel;
use codex_protocol::config_types::Verbosity;
use codex_protocol::config_types::WebSearchConfig;
use codex_protocol::config_types::WebSearchMode;
use codex_protocol::config_types::WindowsSandboxLevel;
use codex_protocol::models::ActivePermissionProfile;
use codex_protocol::models::PermissionProfile;
use codex_protocol::models::SandboxEnforcement;
use codex_protocol::openai_models::ModelsResponse;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::permissions::FileSystemSandboxPolicy;
use codex_protocol::permissions::NetworkSandboxPolicy;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::SandboxPolicy;
use codex_repo_context_scout::RepoContextScoutConfig;
use codex_repo_context_scout::RepoContextScoutMode;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_absolute_path::AbsolutePathBufGuard;
use rmcp::model::ElicitationCapability;
use rmcp::model::FormElicitationCapability;
use rmcp::model::UrlElicitationCapability;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::permissions::BUILT_IN_WORKSPACE_PROFILE;
use crate::config::permissions::apply_network_proxy_feature_config;
use crate::config::permissions::builtin_permission_profile;
use crate::config::permissions::compile_permission_profile_selection;
use crate::config::permissions::compile_permission_profile_workspace_roots;
use crate::config::permissions::default_builtin_permission_profile_name;
use crate::config::permissions::get_readable_roots_required_for_codex_runtime;
use crate::config::permissions::network_proxy_config_for_profile_selection;
use crate::config::permissions::validate_user_permission_profile_names;
use crate::config_lock::config_without_lock_controls;
use crate::config_lock::lock_layer_from_config;
use crate::config_lock::read_config_lock_from_path;
use codex_network_proxy::NetworkProxyConfig;
use toml::Value as TomlValue;

pub(crate) mod agent_roles;
mod builder;
mod config_accessors;
mod config_loaders;
mod config_struct;
mod config_transforms;
mod config_types;
mod context_budget;
pub mod edit;
mod managed_features;
mod network_proxy_spec;
mod otel;
mod permissions;
mod permissions_config;
mod resolved;
pub(crate) mod resolved_permission_profile;
#[cfg(test)]
mod schema;
mod write_api;
use resolved::*;
pub(crate) use context_budget::resolve_context_budget_mode;
pub use builder::ConfigBuilder;
pub use config_struct::Config;
pub use config_types::AgentRoleConfig;
pub use config_types::ConfigOverrides;
pub use config_types::DEFAULT_MULTI_AGENT_V2_ROOT_USAGE_HINT_TEXT;
pub use config_types::DEFAULT_MULTI_AGENT_V2_SUBAGENT_USAGE_HINT_TEXT;
pub use config_types::DesktopAutomationConfig;
pub use config_types::GhostSnapshotConfig;
pub use config_types::MultiAgentV2Config;
pub use config_types::TerminalResizeReflowConfig;
pub use config_types::TerminalResizeReflowMaxRows;
pub use config_types::ThreadStoreConfig;
pub use permissions_config::Permissions;
pub use resolved::resolve_oss_provider;
pub(crate) use resolved::resolve_tool_suggest_config_from_layer_stack;
pub(crate) use resolved::resolve_web_search_mode_for_turn;
pub use write_api::set_default_oss_provider;
pub use write_api::set_project_trust_level;
pub use resolved_permission_profile::PermissionProfileSnapshot;
pub use codex_config::ConfigLoadOptions;
pub use codex_config::Constrained;
pub use codex_config::ConstraintError;
pub use codex_config::ConstraintResult;
pub use codex_config::LoaderOverrides;
pub use codex_network_proxy::NetworkProxyAuditMetadata;
use codex_sandboxing::compatibility_sandbox_policy_for_permission_profile;
pub use codex_sandboxing::system_bwrap_warning;
pub use managed_features::ManagedFeatures;
pub use network_proxy_spec::NetworkProxySpec;
pub use network_proxy_spec::StartedNetworkProxy;
pub(crate) use permissions::is_builtin_permission_profile_name;
pub(crate) use permissions::reject_unknown_builtin_permission_profile;
pub(crate) use permissions::resolve_permission_profile;

const DEFAULT_IGNORE_LARGE_UNTRACKED_DIRS: i64 = 200;
const DEFAULT_IGNORE_LARGE_UNTRACKED_FILES: i64 = 10 * 1024 * 1024;

/// Maximum number of bytes of the documentation that will be embedded. Larger
/// files are *silently truncated* to this size so we do not take up too much of
/// the context window.
pub(crate) const AGENTS_MD_MAX_BYTES: usize = DEFAULT_PROJECT_DOC_MAX_BYTES; // 32 KiB
pub(crate) const DEFAULT_AGENT_MAX_THREADS: Option<usize> = Some(6);
pub(crate) const DEFAULT_MULTI_AGENT_V2_MAX_CONCURRENT_THREADS_PER_SESSION: usize = 4;
pub(crate) const DEFAULT_MULTI_AGENT_V2_MIN_WAIT_TIMEOUT_MS: i64 = 10_000;
pub(crate) const DEFAULT_MULTI_AGENT_V2_MAX_WAIT_TIMEOUT_MS: i64 = 3600 * 1000;
pub(crate) const DEFAULT_MULTI_AGENT_V2_DEFAULT_WAIT_TIMEOUT_MS: i64 = 30_000;
pub(crate) const HARD_MIN_MULTI_AGENT_V2_TIMEOUT_MS: i64 = 0;
pub(crate) const HARD_MAX_MULTI_AGENT_V2_TIMEOUT_MS: i64 =
    DEFAULT_MULTI_AGENT_V2_MAX_WAIT_TIMEOUT_MS;
pub(crate) const DEFAULT_AGENT_MAX_DEPTH: i32 = 1;
pub(crate) const DEFAULT_AGENT_JOB_MAX_RUNTIME_SECONDS: Option<u64> = None;
const LOCAL_DEV_BUILD_VERSION: &str = "0.0.0";

pub const CONFIG_TOML_FILE: &str = "config.toml";

fn resolve_sqlite_home_env(resolved_cwd: &Path) -> Option<PathBuf> {
    let raw = std::env::var(codex_state::SQLITE_HOME_ENV).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        Some(path)
    } else {
        Some(resolved_cwd.join(path))
    }
}

fn resolve_cli_auth_credentials_store_mode(
    configured: AuthCredentialsStoreMode,
    package_version: &str,
) -> AuthCredentialsStoreMode {
    match (package_version, configured) {
        (
            LOCAL_DEV_BUILD_VERSION,
            AuthCredentialsStoreMode::Keyring | AuthCredentialsStoreMode::Auto,
        ) => AuthCredentialsStoreMode::File,
        (_, mode) => mode,
    }
}

fn resolve_mcp_oauth_credentials_store_mode(
    configured: OAuthCredentialsStoreMode,
    package_version: &str,
) -> OAuthCredentialsStoreMode {
    match (package_version, configured) {
        (
            LOCAL_DEV_BUILD_VERSION,
            OAuthCredentialsStoreMode::Keyring | OAuthCredentialsStoreMode::Auto,
        ) => OAuthCredentialsStoreMode::File,
        (_, mode) => mode,
    }
}

fn resolve_model_compact_percentage(
    configured: Option<i64>,
    startup_warnings: &mut Vec<String>,
) -> u8 {
    match configured {
        None => DEFAULT_TRIGGER_CONTEXT_PERCENT,
        Some(value) if (0..=100).contains(&value) => value as u8,
        Some(_value) => {
            startup_warnings.push(format!(
                "configured value for `model_compact_percentage` must be between 0 and 100; \
                 using default {DEFAULT_TRIGGER_CONTEXT_PERCENT}"
            ));
            DEFAULT_TRIGGER_CONTEXT_PERCENT
        }
    }
}

#[cfg(test)]
pub(crate) async fn test_config() -> Config {
    let codex_home = tempfile::tempdir().expect("create temp dir");
    Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides::default(),
        AbsolutePathBuf::from_absolute_path(codex_home.path()).expect("temp dir should resolve"),
    )
    .await
    .expect("load default test config")
}

// A profile override only inherits the selected profile's proxy/allowlist config
// when Codex is still responsible for the network policy. `Disabled` means no
// outer sandbox, so starting the managed proxy would narrow the override.
fn profile_allows_configured_network_proxy(permission_profile: &PermissionProfile) -> bool {
    match permission_profile {
        PermissionProfile::Managed { network, .. } | PermissionProfile::External { network } => {
            network.is_enabled()
        }
        PermissionProfile::Disabled => false,
    }
}

fn build_network_proxy_spec(
    configured_network_proxy_config: NetworkProxyConfig,
    network_requirements: Option<Sourced<codex_config::NetworkConstraints>>,
    permission_profile: &PermissionProfile,
) -> std::io::Result<Option<NetworkProxySpec>> {
    let (network_requirements, network_requirements_source) = match network_requirements {
        Some(Sourced { value, source }) => (Some(value), Some(source)),
        None => (None, None),
    };
    let has_network_requirements = network_requirements.is_some();
    let network = NetworkProxySpec::from_config_and_constraints(
        configured_network_proxy_config,
        network_requirements,
        permission_profile,
    )
    .map_err(|err| {
        if let Some(source) = network_requirements_source.as_ref() {
            std::io::Error::new(
                err.kind(),
                format!("failed to build managed network proxy from {source}: {err}"),
            )
        } else {
            err
        }
    })?;

    Ok(if has_network_requirements {
        Some(network)
    } else {
        network.enabled().then_some(network)
    })
}

/// DEPRECATED for most callers: prefer [Config::load_with_cli_overrides()] or
/// [ConfigBuilder] because working with [ConfigToml] directly means
/// [ConfigRequirements] have not been applied yet, which risks skipping
/// required constraints.
pub async fn load_config_as_toml_with_cli_and_loader_overrides(
    codex_home: &Path,
    cwd: Option<&AbsolutePathBuf>,
    cli_overrides: Vec<(String, TomlValue)>,
    loader_overrides: LoaderOverrides,
) -> std::io::Result<ConfigToml> {
    load_config_as_toml_with_cli_and_load_options(codex_home, cwd, cli_overrides, loader_overrides)
        .await
}

/// DEPRECATED for most callers: prefer [Config::load_with_cli_overrides()] or
/// [ConfigBuilder] because working with [ConfigToml] directly means
/// [ConfigRequirements] have not been applied yet, which risks skipping
/// required constraints.
pub async fn load_config_as_toml_with_cli_and_load_options(
    codex_home: &Path,
    cwd: Option<&AbsolutePathBuf>,
    cli_overrides: Vec<(String, TomlValue)>,
    options: impl Into<ConfigLoadOptions>,
) -> std::io::Result<ConfigToml> {
    let config_layer_stack = load_config_layers_state(
        LOCAL_FS.as_ref(),
        codex_home,
        cwd.cloned(),
        &cli_overrides,
        options,
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    let merged_toml = config_layer_stack.effective_config();
    let cfg = deserialize_config_toml_with_base(merged_toml, codex_home).map_err(|e| {
        tracing::error!("Failed to deserialize overridden config: {e}");
        e
    })?;

    Ok(cfg)
}

pub fn deserialize_config_toml_with_base(
    root_value: TomlValue,
    config_base_dir: &Path,
) -> std::io::Result<ConfigToml> {
    // This guard ensures that any relative paths that is deserialized into an
    // [AbsolutePathBuf] is resolved against `config_base_dir`.
    let _guard = AbsolutePathBufGuard::new(config_base_dir);
    root_value
        .try_into()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Validate user-visible feature settings against managed feature requirements.
pub fn validate_feature_requirements_for_config_toml(
    cfg: &ConfigToml,
    feature_requirements: Option<&Sourced<FeatureRequirementsToml>>,
) -> std::io::Result<()> {
    managed_features::validate_explicit_feature_settings_in_config_toml(cfg, feature_requirements)?;
    managed_features::validate_feature_requirements_in_config_toml(cfg, feature_requirements)
}

fn load_catalog_json(path: &AbsolutePathBuf) -> std::io::Result<ModelsResponse> {
    let file_contents = std::fs::read_to_string(path)?;
    let catalog = serde_json::from_str::<ModelsResponse>(&file_contents).map_err(|err| {
        std::io::Error::new(
            ErrorKind::InvalidData,
            format!(
                "failed to parse model_catalog_json path `{}` as JSON: {err}",
                path.display()
            ),
        )
    })?;
    if catalog.models.is_empty() {
        return Err(std::io::Error::new(
            ErrorKind::InvalidData,
            format!(
                "model_catalog_json path `{}` must contain at least one model",
                path.display()
            ),
        ));
    }
    Ok(catalog)
}

fn load_model_catalog(
    model_catalog_json: Option<AbsolutePathBuf>,
) -> std::io::Result<Option<ModelsResponse>> {
    model_catalog_json
        .map(|path| load_catalog_json(&path))
        .transpose()
}

fn filter_mcp_servers_by_requirements(
    mcp_servers: &mut HashMap<String, McpServerConfig>,
    mcp_requirements: Option<&Sourced<BTreeMap<String, McpServerRequirement>>>,
) {
    let Some(allowlist) = mcp_requirements else {
        return;
    };

    let source = allowlist.source.clone();
    for (name, server) in mcp_servers.iter_mut() {
        let allowed = allowlist
            .value
            .get(name)
            .is_some_and(|requirement| mcp_server_matches_requirement(requirement, server));
        if allowed {
            server.disabled_reason = None;
        } else {
            server.enabled = false;
            server.disabled_reason = Some(McpServerDisabledReason::Requirements {
                source: source.clone(),
            });
        }
    }
}

fn filter_plugin_mcp_servers_by_requirements(
    plugin_config_name: &str,
    mcp_servers: &mut HashMap<String, McpServerConfig>,
    plugin_requirements: Option<&Sourced<BTreeMap<String, PluginRequirementsToml>>>,
) {
    let Some(requirements) = plugin_requirements else {
        return;
    };
    let source = requirements.source.clone();
    let plugin_mcp_requirements = requirements
        .value
        .get(plugin_config_name)
        .and_then(|plugin| plugin.mcp_servers.as_ref());

    for (name, server) in mcp_servers.iter_mut() {
        let allowed = plugin_mcp_requirements
            .and_then(|mcp_requirements| mcp_requirements.get(name))
            .is_some_and(|requirement| mcp_server_matches_requirement(requirement, server));
        if allowed {
            server.disabled_reason = None;
        } else {
            server.enabled = false;
            server.disabled_reason = Some(McpServerDisabledReason::Requirements {
                source: source.clone(),
            });
        }
    }
}

fn constrain_mcp_servers(
    mcp_servers: HashMap<String, McpServerConfig>,
    mcp_requirements: Option<&Sourced<BTreeMap<String, McpServerRequirement>>>,
) -> ConstraintResult<Constrained<HashMap<String, McpServerConfig>>> {
    if mcp_requirements.is_none() {
        return Ok(Constrained::allow_any(mcp_servers));
    }

    let mcp_requirements = mcp_requirements.cloned();
    Constrained::normalized(mcp_servers, move |mut servers| {
        filter_mcp_servers_by_requirements(&mut servers, mcp_requirements.as_ref());
        servers
    })
}

fn apply_requirement_constrained_value<T>(
    field_name: &'static str,
    configured_value: T,
    constrained_value: &mut ConstrainedWithSource<T>,
    startup_warnings: &mut Vec<String>,
) -> std::io::Result<bool>
where
    T: Clone + std::fmt::Debug + Send + Sync,
{
    if let Err(err) = constrained_value.set(configured_value) {
        let fallback_value = constrained_value.get().clone();
        tracing::warn!(
            error = %err,
            ?fallback_value,
            requirement_source = ?constrained_value.source,
            "configured value is disallowed by requirements; falling back to required value for {field_name}"
        );
        let message = format!(
            "Configured value for `{field_name}` is disallowed by requirements; falling back to required value {fallback_value:?}. Details: {err}"
        );
        startup_warnings.push(message);

        constrained_value.set(fallback_value).map_err(|fallback_err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "configured value for `{field_name}` is disallowed by requirements ({err}); fallback to a requirement-compliant value also failed ({fallback_err})"
                ),
            )
        })?;
        return Ok(true);
    }

    Ok(false)
}

fn mcp_server_matches_requirement(
    requirement: &McpServerRequirement,
    server: &McpServerConfig,
) -> bool {
    match &requirement.identity {
        McpServerIdentity::Command {
            command: want_command,
        } => matches!(
            &server.transport,
            McpServerTransportConfig::Stdio { command: got_command, .. }
                if got_command == want_command
        ),
        McpServerIdentity::Url { url: want_url } => matches!(
            &server.transport,
            McpServerTransportConfig::StreamableHttp { url: got_url, .. }
                if got_url == want_url
        ),
    }
}

pub async fn load_global_mcp_servers(
    codex_home: &Path,
) -> std::io::Result<BTreeMap<String, McpServerConfig>> {
    // In general, Config::load_with_cli_overrides() should be used to load the
    // full config with requirements.toml applied, but in this case, we need
    // access to the raw TOML in order to warn the user about deprecated fields.
    //
    // Note that a more precise way to do this would be to audit the individual
    // config layers for deprecated fields rather than reporting on the merged
    // result.
    let cli_overrides = Vec::<(String, TomlValue)>::new();
    // There is no cwd/project context for this query, so this will not include
    // MCP servers defined in in-repo .codex/ folders.
    let cwd: Option<AbsolutePathBuf> = None;
    let config_layer_stack = load_config_layers_state(
        LOCAL_FS.as_ref(),
        codex_home,
        cwd,
        &cli_overrides,
        LoaderOverrides::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;
    let merged_toml = config_layer_stack.effective_config();
    let Some(servers_value) = merged_toml.get("mcp_servers") else {
        return Ok(BTreeMap::new());
    };

    ensure_no_inline_bearer_tokens(servers_value)?;

    servers_value
        .clone()
        .try_into()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// We briefly allowed plain text bearer_token fields in MCP server configs.
/// We want to warn people who recently added these fields but can remove this after a few months.
fn ensure_no_inline_bearer_tokens(value: &TomlValue) -> std::io::Result<()> {
    let Some(servers_table) = value.as_table() else {
        return Ok(());
    };

    for (server_name, server_value) in servers_table {
        if let Some(server_table) = server_value.as_table()
            && server_table.contains_key("bearer_token")
        {
            let message = format!(
                "mcp_servers.{server_name} uses unsupported `bearer_token`; set `bearer_token_env_var`."
            );
            return Err(std::io::Error::new(ErrorKind::InvalidData, message));
        }
    }

    Ok(())
}

fn is_session_layer(source: &ConfigLayerSource) -> bool {
    matches!(source, ConfigLayerSource::SessionFlags)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PermissionConfigSyntax {
    Legacy,
    Profiles,
}

#[derive(Debug, Deserialize, Default)]
struct PermissionSelectionToml {
    default_permissions: Option<String>,
    sandbox_mode: Option<SandboxMode>,
}

// Resolve the named-profile catalog and selected profile id together. Runtime
// profile constraints are applied later after this selection compiles into a
// concrete `PermissionProfile`.
#[derive(Debug)]
struct EffectivePermissionSelection<'a> {
    profiles: Option<PermissionsToml>,
    selected_profile_id: Option<&'a str>,
    requirements_force_profile_selection: bool,
}

impl EffectivePermissionSelection<'_> {
    fn has_profiles(&self) -> bool {
        self.profiles
            .as_ref()
            .is_some_and(|profiles| !profiles.is_empty())
    }

    fn profiles_are_active(
        &self,
        default_permissions_override: Option<&str>,
        permission_config_syntax: Option<PermissionConfigSyntax>,
    ) -> bool {
        self.requirements_force_profile_selection
            || default_permissions_override.is_some()
            || matches!(
                permission_config_syntax,
                Some(PermissionConfigSyntax::Profiles)
            )
            || permission_config_syntax.is_none()
    }
}

fn dedupe_absolute_paths(paths: &mut Vec<AbsolutePathBuf>) {
    let mut seen = HashSet::new();
    paths.retain(|path| seen.insert(path.clone()));
}

fn resolve_permission_config_syntax(
    config_layer_stack: &ConfigLayerStack,
    cfg: &ConfigToml,
    sandbox_mode_override: Option<SandboxMode>,
) -> Option<PermissionConfigSyntax> {
    if sandbox_mode_override.is_some() {
        return Some(PermissionConfigSyntax::Legacy);
    }

    let session_flags_select_profiles = config_layer_stack
        .get_layers(
            ConfigLayerStackOrdering::HighestPrecedenceFirst,
            /*include_disabled*/ false,
        )
        .into_iter()
        .find(|layer| matches!(layer.name, ConfigLayerSource::SessionFlags))
        .and_then(|layer| {
            layer
                .config
                .clone()
                .try_into::<PermissionSelectionToml>()
                .ok()
        })
        .is_some_and(|selection| selection.default_permissions.is_some());
    if session_flags_select_profiles {
        return Some(PermissionConfigSyntax::Profiles);
    }

    let mut selection = None;
    for layer in config_layer_stack.get_layers(
        ConfigLayerStackOrdering::LowestPrecedenceFirst,
        /*include_disabled*/ false,
    ) {
        let Ok(layer_selection) = layer.config.clone().try_into::<PermissionSelectionToml>() else {
            continue;
        };

        if layer_selection.sandbox_mode.is_some() {
            selection = Some(PermissionConfigSyntax::Legacy);
        }
        if layer_selection.default_permissions.is_some() {
            selection = Some(PermissionConfigSyntax::Profiles);
        }
    }

    selection.or_else(|| {
        if cfg.default_permissions.is_some() {
            Some(PermissionConfigSyntax::Profiles)
        } else if cfg.sandbox_mode.is_some() {
            Some(PermissionConfigSyntax::Legacy)
        } else {
            None
        }
    })
}

fn apply_managed_filesystem_constraints(
    file_system_sandbox_policy: &mut FileSystemSandboxPolicy,
    filesystem_constraints: &codex_config::FilesystemConstraints,
) {
    for deny_read in &filesystem_constraints.deny_read {
        let deny_entry = if deny_read.contains_glob() {
            codex_protocol::permissions::FileSystemSandboxEntry {
                path: codex_protocol::permissions::FileSystemPath::GlobPattern {
                    pattern: deny_read.as_str().to_string(),
                },
                access: codex_protocol::permissions::FileSystemAccessMode::None,
            }
        } else {
            let Ok(path) = AbsolutePathBuf::try_from(deny_read.as_str()) else {
                continue;
            };
            codex_protocol::permissions::FileSystemSandboxEntry {
                path: codex_protocol::permissions::FileSystemPath::Path { path },
                access: codex_protocol::permissions::FileSystemAccessMode::None,
            }
        };
        if !file_system_sandbox_policy
            .entries
            .iter()
            .any(|existing| existing == &deny_entry)
        {
            file_system_sandbox_policy.entries.push(deny_entry);
        }
    }
}

fn validate_multi_agent_v2_wait_timeout(label: &str, value: i64) -> std::io::Result<()> {
    if value < HARD_MIN_MULTI_AGENT_V2_TIMEOUT_MS {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{label} must be at least {HARD_MIN_MULTI_AGENT_V2_TIMEOUT_MS}"),
        ));
    }
    if value > HARD_MAX_MULTI_AGENT_V2_TIMEOUT_MS {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{label} must be at most {HARD_MAX_MULTI_AGENT_V2_TIMEOUT_MS}"),
        ));
    }
    Ok(())
}

pub(crate) fn uses_deprecated_instructions_file(config_layer_stack: &ConfigLayerStack) -> bool {
    config_layer_stack
        .layers_high_to_low()
        .into_iter()
        .any(|layer| toml_uses_deprecated_instructions_file(&layer.config))
}

fn guardian_policy_config_from_requirements(
    requirements_toml: &ConfigRequirementsToml,
) -> Option<String> {
    normalize_guardian_policy_config(requirements_toml.guardian_policy_config.as_deref())
}

fn merge_managed_permission_profiles(
    configured_permissions: Option<&PermissionsToml>,
    _requirements_toml: &ConfigRequirementsToml,
) -> std::io::Result<Option<PermissionsToml>> {
    Ok(configured_permissions.cloned())
}

fn resolve_effective_permission_selection<'a>(
    configured_permissions: Option<&PermissionsToml>,
    default_permissions_override: Option<&'a str>,
    configured_default_permissions: Option<&'a str>,
    requirements_toml: &'a ConfigRequirementsToml,
    startup_warnings: &mut Vec<String>,
) -> std::io::Result<EffectivePermissionSelection<'a>> {
    let profiles = merge_managed_permission_profiles(configured_permissions, requirements_toml)?;
    validate_user_permission_profile_names(profiles.as_ref())?;
    validate_required_permission_profile_catalog(requirements_toml, profiles.as_ref())?;
    let selected_profile_id = resolve_default_permissions(
        default_permissions_override,
        configured_default_permissions,
        requirements_toml,
        startup_warnings,
    )?;

    Ok(EffectivePermissionSelection {
        profiles,
        selected_profile_id,
        requirements_force_profile_selection: false,
    })
}

fn resolve_default_permissions<'a>(
    default_permissions_override: Option<&'a str>,
    configured_default_permissions: Option<&'a str>,
    _requirements_toml: &'a ConfigRequirementsToml,
    _startup_warnings: &mut Vec<String>,
) -> std::io::Result<Option<&'a str>> {
    Ok(default_permissions_override.or(configured_default_permissions))
}

fn validate_required_permission_profile_catalog(
    _requirements_toml: &ConfigRequirementsToml,
    _available_permissions: Option<&PermissionsToml>,
) -> std::io::Result<()> {
    Ok(())
}

fn normalize_guardian_policy_config(value: Option<&str>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn toml_uses_deprecated_instructions_file(value: &TomlValue) -> bool {
    let Some(table) = value.as_table() else {
        return false;
    };
    if table.contains_key("experimental_instructions_file") {
        return true;
    }
    let Some(profiles) = table.get("profiles").and_then(TomlValue::as_table) else {
        return false;
    };
    profiles.values().any(|profile| {
        profile.as_table().is_some_and(|profile_table| {
            profile_table.contains_key("experimental_instructions_file")
        })
    })
}

/// Returns the path to the Codex configuration directory, which can be
/// specified by the `CODEX_HOME` environment variable. If not set, defaults to
/// `~/.codex`.
///
/// - If `CODEX_HOME` is set, the value must exist and be a directory. The
///   value will be canonicalized and this function will Err otherwise.
/// - If `CODEX_HOME` is not set, this function does not verify that the
///   directory exists.
pub fn find_codex_home() -> std::io::Result<AbsolutePathBuf> {
    codex_utils_home_dir::find_codex_home()
}

/// Returns the path to the folder where Codex logs are stored. Does not verify
/// that the directory exists.
pub fn log_dir(cfg: &Config) -> std::io::Result<PathBuf> {
    Ok(cfg.log_dir.clone())
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "config_loader_tests.rs"]
mod config_loader_tests;
