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
use codex_config::config_toml::ActionOptimizationInstructionsModeToml;
use codex_config::config_toml::ActionOptimizationInstructionsVariantToml;
use codex_config::config_toml::BatchMiniProgrammingInstructionsModeToml;
use codex_config::config_toml::BatchMiniProgrammingInstructionsVariantToml;
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
use codex_config::types::AuthKeyringBackendKind;
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
use codex_config::types::PromptReductionTuning;
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
use codex_features::CodeModeConfigToml;
use codex_features::CurrentTimeReminderConfigToml;
use codex_features::CurrentTimeSource;
use codex_features::Feature;
use codex_features::FeatureConfigSource;
use codex_features::FeatureOverrides;
use codex_features::FeatureToml;
use codex_features::Features;
use codex_features::FeaturesToml;
use codex_features::MultiAgentV2ConfigToml;
use codex_features::NetworkProxyConfigToml;
use codex_features::RolloutBudgetConfigToml;
use codex_features::TokenBudgetConfigToml;
use codex_first_moves::FirstMovesConfig;
use codex_first_moves::FirstMovesMode;
use codex_first_moves::FirstMovesPrewarm;
use codex_git_utils::resolve_root_git_project_for_trust;
use codex_install_context::InstallContext;
use codex_login::AuthManagerConfig;
use codex_login::AuthRouteConfig;
use codex_mcp::McpConfig;
use codex_mcp::McpPluginAttribution;
use codex_mcp::McpServerRegistration;
use codex_mcp::ResolvedMcpCatalog;
use codex_memories_read::memory_root;
use codex_model_provider_info::LEGACY_OLLAMA_CHAT_PROVIDER_ID;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::OLLAMA_CHAT_PROVIDER_REMOVED_ERROR;
use codex_model_provider_info::built_in_model_providers;
use codex_model_provider_info::merge_configured_model_providers;
use codex_models_manager::ModelsManagerConfig;
use codex_protocol::config_types::AltScreenMode;
use codex_protocol::config_types::AutoCompactTokenLimitScope;
use codex_protocol::config_types::ContextBudgetMode;
use codex_protocol::config_types::ForcedLoginMethod;
use codex_protocol::config_types::Personality;
use codex_protocol::config_types::ProfileV2Name;
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
use codex_utils_path_uri::PathUri;
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
use toml_edit::DocumentMut;

pub(crate) mod agent_roles;
mod auth_keyring;
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
mod permission_profile_catalog;
mod permissions;
mod permissions_config;
mod resolved;
pub(crate) mod resolved_permission_profile;
#[cfg(test)]
mod schema;
mod write_api;
pub use auth_keyring::resolve_bootstrap_auth_keyring_backend_kind;
pub use builder::ConfigBuilder;
pub use codex_config::ConfigLoadOptions;
pub use codex_config::Constrained;
pub use codex_config::ConstraintError;
pub use codex_config::ConstraintResult;
pub use codex_config::LoaderOverrides;
pub use codex_network_proxy::NetworkProxyAuditMetadata;
use codex_sandboxing::compatibility_sandbox_policy_for_permission_profile;
pub use codex_sandboxing::system_bwrap_warning;
pub use codex_thread_store_api::ExtraConfig;
pub use config_struct::ActionOptimizationInstructionsConfig;
pub use config_struct::ActionOptimizationInstructionsMode;
pub use config_struct::ActionOptimizationInstructionsVariant;
pub use config_struct::BatchMiniProgrammingInstructionsConfig;
pub use config_struct::BatchMiniProgrammingInstructionsMode;
pub use config_struct::BatchMiniProgrammingInstructionsVariant;
pub use config_struct::CodeModeConfig;
pub use config_struct::Config;
pub use config_types::AgentRoleConfig;
pub use config_types::ConfigOverrides;
pub use config_types::CurrentTimeReminderConfig;
pub use config_types::DesktopAutomationConfig;
pub use config_types::GhostSnapshotConfig;
pub use config_types::MultiAgentV2Config;
pub use config_types::RolloutBudgetConfig;
pub use config_types::TerminalResizeReflowConfig;
pub use config_types::TerminalResizeReflowMaxRows;
pub use config_types::ThreadStoreConfig;
pub use config_types::UsageHintCadence;
pub(crate) use context_budget::resolve_context_budget_mode;
pub use managed_features::ManagedFeatures;
pub use network_proxy_spec::NetworkProxySpec;
pub use network_proxy_spec::StartedNetworkProxy;
pub use permission_profile_catalog::PermissionProfileCatalogEntry;
pub use permission_profile_catalog::permission_profile_catalog;
use permission_profile_catalog::permission_profile_catalog_from_permissions;
use permission_profile_catalog::permission_profile_is_allowed;
use permission_profile_catalog::validate_permission_profile_for_deny_read;
pub(crate) use permissions::is_builtin_permission_profile_name;
pub(crate) use permissions::reject_unknown_builtin_permission_profile;
pub(crate) use permissions::resolve_permission_profile;
pub use permissions_config::Permissions;
pub use resolved::resolve_oss_provider;
pub(crate) use resolved::resolve_tool_suggest_config_from_layer_stack;
pub(crate) use resolved::resolve_web_search_mode_for_turn;
use resolved::*;
pub use resolved_permission_profile::PermissionProfileSnapshot;
pub use write_api::set_default_oss_provider;
pub use write_api::set_project_trust_level;

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
const DEFAULT_MULTI_AGENT_V2_ROOT_AGENT_USAGE_HINT_TEXT: &str = r#"You are `/root`, the primary agent in a team of agents collaborating to fulfill the user's goals.

At the start of your turn, you are the active agent.
You can spawn sub-agents to handle subtasks, and those sub-agents can spawn their own sub-agents.
All agents in the team, including the agents that you can assign tasks to, are equally intelligent and capable, and have access to the same set of tools.

You can use `spawn_agent` to create a new agent, `followup_task` to give an existing agent a new task and trigger a turn, and `send_message` to pass a message to a running agent without triggering a turn.
Child agents can also spawn their own sub-agents.
You can decide how much context you want to propagate to your sub-agents with the `fork_turns` parameter.

You will receive messages in the analysis channel in the form:
```
Message Type: MESSAGE | FINAL_ANSWER
Task name: <recipient>
Sender: <author>
Payload:
<payload text>
```
They may be addressed as to=/root
"#;
const DEFAULT_MULTI_AGENT_V2_SUBAGENT_USAGE_HINT_TEXT: &str = r#"You are an agent in a team of agents collaborating to complete a task.

You can spawn sub-agents to handle subtasks, and those sub-agents can spawn their own sub-agents. All agents in the team, including the agents that you can assign tasks to, are equally intelligent and capable, and have access to the same set of tools.

You can use `spawn_agent` to create a new agent, `followup_task` to give an existing agent a new task and trigger a turn, and `send_message` to pass a message to a running agent.
Child agents can also spawn their own sub-agents.

When you provide a response in the final channel, that content is immediately delivered back to your parent agent.

You will receive messages in the analysis channel in the form:
```
Message Type: NEW_TASK | MESSAGE | FINAL_ANSWER
Task name: <recipient>
Sender: <author>
Payload:
<payload text>
```
You may also see them addressed as to=/root/..., which indicates your identity is /root/...
"#;
const DEFAULT_MULTI_AGENT_V2_TOOL_NAMESPACE: &str = "collaboration";
const DEFAULT_MULTI_AGENT_V2_SHARED_USAGE_HINT_TEXT: &str = r#"Note that collaboration tools cannot be called from inside `functions.exec`. Call `spawn_agent`, `send_message`, `followup_task`, `wait_agent`, `interrupt_agent`, and `list_agents` only as direct tool calls using the recipient shown in their tool definitions, such as `to=functions.collaboration.spawn_agent`, since they are intentionally absent from the `functions.exec` `tools.*` namespace. Available tools in `functions.exec` are explicitly described with a `tools` namespace in the developer message.

All agents share the same directory. In detail:
- All agents have access to the same container and filesystem as you.
- All agents use the same current working directory.
- As a result, edits made by one agent are immediately visible to all other agents.
"#;
fn default_multi_agent_v2_usage_hint_text(usage_hint_text: &str, max_concurrency: usize) -> String {
    format!(
        "{usage_hint_text}\n{DEFAULT_MULTI_AGENT_V2_SHARED_USAGE_HINT_TEXT}\nThere are {max_concurrency} available concurrency slots, meaning that up to {max_concurrency} agents can be active at once, including you."
    )
}

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

pub(crate) const DEFAULT_TOKEN_BUDGET_REMINDER_MESSAGE_TEMPLATE: &str = concat!(
    "Your context window is nearly exhausted (only {n_remaining} tokens remaining) and will be automatically reset for you soon. ",
    "Once reset, message items in current context window will be cleared in the new window, but notes and history items will be persistent across windows."
);
const TOKEN_BUDGET_REMINDER_MESSAGE_TEMPLATE_MAX_BYTES: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenBudgetConfig {
    pub reminder_threshold_tokens: Option<i64>,
    pub reminder_message_template: String,
}

impl Default for TokenBudgetConfig {
    fn default() -> Self {
        Self {
            reminder_threshold_tokens: None,
            reminder_message_template: DEFAULT_TOKEN_BUDGET_REMINDER_MESSAGE_TEMPLATE.to_string(),
        }
    }
}

impl Config {
    pub(crate) fn validate_multi_agent_v2_config(&self) -> std::io::Result<()> {
        if self.features.enabled(Feature::MultiAgentV2) && self.agent_max_threads.is_some() {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "agents.max_threads cannot be set when features.multi_agent_v2 is enabled",
            ))
        } else {
            Ok(())
        }
    }

    pub fn auth_route_config(&self) -> Option<AuthRouteConfig> {
        self.features
            .enabled(Feature::RespectSystemProxy)
            .then(AuthRouteConfig::respect_system_proxy)
    }

    pub(crate) async fn to_mcp_config_with_plugin_registrations(
        &self,
        plugins_manager: &codex_core_plugins::PluginsManager,
        additional_plugin_registrations: impl IntoIterator<Item = McpServerRegistration>,
    ) -> McpConfig {
        let plugins_input = self.plugins_config_input();
        let loaded_plugins = plugins_manager.plugins_for_config(&plugins_input).await;
        let mut catalog = ResolvedMcpCatalog::builder();
        for (plugin_order, plugin) in loaded_plugins
            .plugins()
            .iter()
            .filter(|plugin| plugin.is_active())
            .enumerate()
        {
            let mut plugin_mcp_servers = plugin.mcp_servers.clone();
            self.apply_plugin_mcp_server_requirements(&plugin.config_name, &mut plugin_mcp_servers);
            let attribution = McpPluginAttribution::new(
                plugin.config_name.clone(),
                plugin.display_name().to_string(),
            );
            for (name, plugin_server) in plugin_mcp_servers {
                catalog.register(McpServerRegistration::from_plugin(
                    name,
                    attribution.clone(),
                    plugin_order,
                    plugin_server,
                ));
            }
        }
        for registration in additional_plugin_registrations {
            catalog.register(registration);
        }
        for (name, server) in self.mcp_servers.get() {
            catalog.register(McpServerRegistration::from_config(
                name.clone(),
                server.clone(),
            ));
        }

        McpConfig {
            chatgpt_base_url: self.chatgpt_base_url.clone(),
            apps_mcp_product_sku: self.apps_mcp_product_sku.clone(),
            codex_home: self.codex_home.to_path_buf(),
            mcp_oauth_credentials_store_mode: self.mcp_oauth_credentials_store_mode,
            auth_keyring_backend_kind: self.auth_keyring_backend_kind(),
            mcp_oauth_callback_port: self.mcp_oauth_callback_port,
            mcp_oauth_callback_url: self.mcp_oauth_callback_url.clone(),
            skill_mcp_dependency_install_enabled: self
                .features
                .enabled(Feature::SkillMcpDependencyInstall),
            approval_policy: self.permissions.approval_policy.clone(),
            codex_linux_sandbox_exe: self.codex_linux_sandbox_exe.clone(),
            use_legacy_landlock: self.features.use_legacy_landlock(),
            apps_enabled: self.features.enabled(Feature::Apps),
            prefix_mcp_tool_names: self.prefix_mcp_tool_names(),
            client_elicitation_capability: if self.features.enabled(Feature::AuthElicitation) {
                ElicitationCapability {
                    form: Some(FormElicitationCapability::default()),
                    url: Some(UrlElicitationCapability::default()),
                }
            } else {
                // https://modelcontextprotocol.io/specification/2025-06-18/client/elicitation#capabilities
                // indicates this should be an empty object.
                ElicitationCapability::default()
            },
            mcp_server_catalog: catalog.build(),
            plugin_capability_summaries: loaded_plugins.capability_summaries().to_vec(),
        }
    }

    /// This is a secondary way of creating [Config], which is appropriate when
    /// the harness is meant to be used with a specific configuration that
    /// ignores user settings. For example, the `codex exec` subcommand is
    /// designed to use [AskForApproval::Never] exclusively.
    ///
    /// Further, [ConfigOverrides] contains some options that are not supported
    /// in [ConfigToml], such as `cwd`, `codex_self_exe`, `codex_linux_sandbox_exe`, and
    /// `main_execve_wrapper_exe`.
    pub async fn load_with_cli_overrides_and_harness_overrides(
        cli_overrides: Vec<(String, TomlValue)>,
        harness_overrides: ConfigOverrides,
    ) -> std::io::Result<Self> {
        ConfigBuilder::default()
            .cli_overrides(cli_overrides)
            .harness_overrides(harness_overrides)
            .build()
            .await
    }
}

/// Filename suffix for per-profile config files (`<profile>.config.toml`).
/// Mirrors the local consts in `cli/src/dispatch.rs` and
/// `tui/src/session_archive_commands.rs`; the merge dropped the symbol the fork
/// referenced here, so it is re-declared locally to keep the helper self-contained.
const CONFIG_PROFILE_V2_SUFFIX: &str = ".config.toml";

pub fn resolve_profile_v2_config_path(
    codex_home: &Path,
    profile_name: &ProfileV2Name,
) -> AbsolutePathBuf {
    AbsolutePathBuf::resolve_path_against_base(
        format!("{profile_name}{CONFIG_PROFILE_V2_SUFFIX}"),
        codex_home,
    )
}

/// DEPRECATED: Use [Config::load_with_cli_overrides()] instead because working
/// with [ConfigToml] directly means that [ConfigRequirements] have not been
/// applied yet, which risks failing to enforce required constraints.
pub async fn load_config_as_toml_with_cli_overrides(
    codex_home: &Path,
    cwd: Option<&AbsolutePathBuf>,
    cli_overrides: Vec<(String, TomlValue)>,
    loader_overrides: LoaderOverrides,
) -> std::io::Result<ConfigToml> {
    load_config_as_toml_with_cli_and_loader_overrides(
        codex_home,
        cwd,
        cli_overrides,
        loader_overrides,
    )
    .await
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
    load_config_toml_with_layer_stack(codex_home, cwd, cli_overrides, options)
        .await
        .map(|result| result.config_toml)
}

/// Partially loaded config plus the layer stack used to derive it.
///
/// This is intended for startup paths that must inspect raw config before a
/// full [`Config`] can be constructed, but still need access to managed
/// requirements loaded with the config layers.
pub struct ConfigTomlLoadResult {
    pub config_toml: ConfigToml,
    pub config_layer_stack: ConfigLayerStack,
}

/// Loads the partially merged config together with the layer stack used to
/// derive it, before constructing a full [`Config`].
pub async fn load_config_toml_with_layer_stack(
    codex_home: &Path,
    cwd: Option<&AbsolutePathBuf>,
    cli_overrides: Vec<(String, TomlValue)>,
    options: impl Into<ConfigLoadOptions>,
) -> std::io::Result<ConfigTomlLoadResult> {
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

    Ok(ConfigTomlLoadResult {
        config_toml: cfg,
        config_layer_stack,
    })
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

pub(crate) fn set_project_trust_level_inner(
    doc: &mut DocumentMut,
    project_path: &Path,
    trust_level: TrustLevel,
) -> anyhow::Result<()> {
    // Ensure we render a human-friendly structure:
    //
    // [projects]
    // [projects."/path/to/project"]
    // trust_level = "trusted" or "untrusted"
    //
    // rather than inline tables like:
    //
    // [projects]
    // "/path/to/project" = { trust_level = "trusted" }
    let project_key = project_trust_key(project_path);

    // Ensure top-level `projects` exists as a non-inline, explicit table. If it
    // exists but was previously represented as a non-table (e.g., inline),
    // replace it with an explicit table.
    {
        let root = doc.as_table_mut();
        // If `projects` exists but isn't a standard table (e.g., it's an inline table),
        // convert it to an explicit table while preserving existing entries.
        let existing_projects = root.get("projects").cloned();
        if existing_projects.as_ref().is_none_or(|i| !i.is_table()) {
            let mut projects_tbl = toml_edit::Table::new();
            projects_tbl.set_implicit(true);

            // If there was an existing inline table, migrate its entries to explicit tables.
            if let Some(inline_tbl) = existing_projects.as_ref().and_then(|i| i.as_inline_table()) {
                for (k, v) in inline_tbl.iter() {
                    if let Some(inner_tbl) = v.as_inline_table() {
                        let new_tbl = inner_tbl.clone().into_table();
                        projects_tbl.insert(k, toml_edit::Item::Table(new_tbl));
                    }
                }
            }

            root.insert("projects", toml_edit::Item::Table(projects_tbl));
        }
    }
    let Some(projects_tbl) = doc["projects"].as_table_mut() else {
        return Err(anyhow::anyhow!(
            "projects table missing after initialization"
        ));
    };

    // Ensure the per-project entry is its own explicit table. If it exists but
    // is not a table (e.g., an inline table), replace it with an explicit table.
    let needs_proj_table = !projects_tbl.contains_key(project_key.as_str())
        || projects_tbl
            .get(project_key.as_str())
            .and_then(|i| i.as_table())
            .is_none();
    if needs_proj_table {
        projects_tbl.insert(project_key.as_str(), toml_edit::table());
    }
    let Some(proj_tbl) = projects_tbl
        .get_mut(project_key.as_str())
        .and_then(|i| i.as_table_mut())
    else {
        return Err(anyhow::anyhow!("project table missing for {project_key}"));
    };
    proj_tbl.set_implicit(false);
    proj_tbl["trust_level"] = toml_edit::value(trust_level.to_string());
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

fn resolve_experimental_request_user_input_enabled(config_toml: &ConfigToml) -> bool {
    config_toml
        .tools
        .as_ref()
        .and_then(|tools| tools.experimental_request_user_input.as_ref())
        .is_none_or(|config| config.enabled)
}

fn resolve_token_budget_config(
    config_toml: &ConfigToml,
    features: &ManagedFeatures,
) -> std::io::Result<Option<TokenBudgetConfig>> {
    if !features.enabled(Feature::TokenBudget) {
        return Ok(None);
    }

    let token_budget_config = token_budget_toml_config(config_toml.features.as_ref());
    let reminder_threshold_tokens =
        token_budget_config.and_then(|config| config.reminder_threshold_tokens);
    if reminder_threshold_tokens.is_some_and(|tokens| tokens <= 0) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "features.token_budget.reminder_threshold_tokens must be positive",
        ));
    }

    let reminder_message_template = token_budget_config
        .and_then(|config| config.reminder_message_template.clone())
        .unwrap_or_else(|| DEFAULT_TOKEN_BUDGET_REMINDER_MESSAGE_TEMPLATE.to_string());
    if reminder_message_template.trim().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "features.token_budget.reminder_message_template must not be empty",
        ));
    }
    if reminder_message_template.len() > TOKEN_BUDGET_REMINDER_MESSAGE_TEMPLATE_MAX_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "features.token_budget.reminder_message_template must not exceed {TOKEN_BUDGET_REMINDER_MESSAGE_TEMPLATE_MAX_BYTES} bytes"
            ),
        ));
    }

    Ok(Some(TokenBudgetConfig {
        reminder_threshold_tokens,
        reminder_message_template,
    }))
}

fn resolve_optional_prompt_text(
    configured: Option<&Option<String>>,
    default: Option<String>,
) -> Option<String> {
    match configured {
        Some(Some(value)) if value.is_empty() => None,
        Some(Some(value)) => Some(value.clone()),
        Some(None) | None => default,
    }
}

fn token_budget_toml_config(features: Option<&FeaturesToml>) -> Option<&TokenBudgetConfigToml> {
    match features?.token_budget.as_ref()? {
        FeatureToml::Enabled(_) => None,
        FeatureToml::Config(config) => Some(config),
    }
}

/// Bootstrap-only resolver for the cloud-config fetch.
///
/// Call before a cloud-config bundle is available. Final [`Config`] loading
/// resolves the effective feature value after all layers are available.
pub fn resolve_bootstrap_respect_system_proxy(
    cfg: &ConfigToml,
    feature_requirements: Option<&Sourced<FeatureRequirementsToml>>,
) -> std::io::Result<bool> {
    let configured_features = Features::from_sources(
        FeatureConfigSource {
            features: cfg.features.as_ref(),
            experimental_use_unified_exec_tool: cfg.experimental_use_unified_exec_tool,
        },
        FeatureConfigSource::default(),
        FeatureOverrides::default(),
    );
    let features =
        ManagedFeatures::from_configured(configured_features, feature_requirements.cloned())?;
    Ok(features.get().enabled(Feature::RespectSystemProxy))
}

/// Resolves auth route settings for the initial cloud-config bootstrap.
pub fn resolve_bootstrap_auth_route_config(
    cfg: &ConfigToml,
    feature_requirements: Option<&Sourced<FeatureRequirementsToml>>,
) -> std::io::Result<Option<AuthRouteConfig>> {
    resolve_bootstrap_respect_system_proxy(cfg, feature_requirements)
        .map(|enabled| enabled.then(AuthRouteConfig::respect_system_proxy))
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

fn validate_multi_agent_v2_tool_namespace(namespace: Option<&str>) -> std::io::Result<()> {
    const LABEL: &str = "features.multi_agent_v2.tool_namespace";
    const MAX_LEN: usize = 64;
    const RESERVED_RESPONSES_NAMESPACES: &[&str] = &[
        "api_tool",
        "browser",
        "computer",
        "container",
        "file_search",
        "functions",
        "image_gen",
        "multi_tool_use",
        "python",
        "python_user_visible",
        "submodel_delegator",
        "terminal",
        "tool_search",
        "web",
    ];

    let Some(namespace) = namespace else {
        return Ok(());
    };
    if namespace.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{LABEL} must not be empty"),
        ));
    }
    if namespace.trim() != namespace {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{LABEL} must not have leading or trailing whitespace"),
        ));
    }
    if !namespace
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{LABEL} must match ^[a-zA-Z0-9_-]+$"),
        ));
    }
    if namespace.chars().count() > MAX_LEN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{LABEL} must be at most {MAX_LEN} characters"),
        ));
    }
    if namespace == "mcp"
        || namespace.starts_with("mcp__")
        || RESERVED_RESPONSES_NAMESPACES.contains(&namespace)
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{LABEL} uses a reserved namespace: {namespace}"),
        ));
    }

    Ok(())
}

impl Config {
    /// Returns whether effective requirements allow selecting a concrete profile.
    pub fn is_permission_profile_allowed(
        &self,
        profile_id: &str,
        permission_profile: &PermissionProfile,
    ) -> bool {
        permission_profile_is_allowed(&self.config_layer_stack, profile_id, permission_profile)
    }
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
