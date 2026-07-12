use codex_permission_types::AskForApproval;
use codex_utils_absolute_path::AbsolutePathBuf;
use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::InstanceType;
use schemars::schema::Metadata;
use schemars::schema::Schema;
use schemars::schema::SchemaObject;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroU64;
use std::str::FromStr;
use std::time::Duration;
use strum_macros::Display;
use strum_macros::EnumIter;
use ts_rs::TS;
use wildmatch::WildMatchPattern;

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
pub enum ContextBudgetMode {
    Standard,
    #[default]
    Slow,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum AutoCompactTokenLimitScope {
    #[default]
    BodyAfterPrefix,
    Total,
}

impl fmt::Display for ContextBudgetMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Standard => "standard",
            Self::Slow => "slow",
        };
        f.write_str(value)
    }
}

impl FromStr for ContextBudgetMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .map_err(|_| format!("invalid context_budget_mode: {s}"))
    }
}

/// See https://platform.openai.com/docs/guides/reasoning?api-mode=responses#get-started-with-reasoning
#[derive(
    Debug,
    Serialize,
    Deserialize,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Display,
    JsonSchema,
    TS,
    EnumIter,
    Hash,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    #[default]
    Medium,
    High,
    XHigh,
    // Upstream reasoning-effort tiers above XHigh. Re-added after a union-merge
    // dropped them from the fork's relocated enum; unit variants keep `Copy`
    // (the fork maps Ultra -> Max for Bedrock rather than adopting Custom(String)).
    Max,
    Ultra,
}

impl FromStr for ReasoningEffort {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .map_err(|_| format!("invalid reasoning_effort: {s}"))
    }
}

impl ReasoningEffort {
    /// Lowercase wire/string representation, matching `#[serde(rename_all = "lowercase")]`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::XHigh => "xhigh",
            Self::Max => "max",
            Self::Ultra => "ultra",
        }
    }
}
/// A summary of the reasoning performed by the model. This can be useful for
/// debugging and understanding the model's reasoning process.
/// See https://platform.openai.com/docs/guides/reasoning?api-mode=responses#reasoning-summaries
#[derive(
    Debug, Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Display, JsonSchema, TS,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ReasoningSummary {
    #[default]
    Auto,
    Concise,
    Detailed,
    /// Option to disable reasoning summaries.
    None,
}

/// Controls output length/detail on GPT-5 models via the Responses API.
/// Serialized with lowercase values to match the OpenAI API.
#[derive(
    Hash,
    Debug,
    Serialize,
    Deserialize,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Display,
    JsonSchema,
    TS,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Verbosity {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(
    Deserialize, Debug, Clone, Copy, PartialEq, Default, Serialize, Display, JsonSchema, TS,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum SandboxMode {
    #[serde(rename = "read-only")]
    #[default]
    ReadOnly,

    #[serde(rename = "workspace-write")]
    WorkspaceWrite,

    #[serde(rename = "danger-full-access")]
    DangerFullAccess,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Display, TS)]
#[strum(serialize_all = "snake_case")]
#[ts(type = r#""user" | "auto_review" | "guardian_subagent""#)]
/// Configures who approval requests are routed to for review. Examples
/// include sandbox escapes, blocked network access, MCP approval prompts, and
/// ARC escalations. Defaults to `user`. `auto_review` uses a carefully
/// prompted subagent to gather relevant context and apply a risk-based
/// decision framework before approving or denying the request.
pub enum ApprovalsReviewer {
    #[default]
    #[serde(rename = "user")]
    User,
    #[serde(rename = "guardian_subagent", alias = "auto_review")]
    #[strum(serialize = "guardian_subagent")]
    AutoReview,
}

impl JsonSchema for ApprovalsReviewer {
    fn schema_name() -> String {
        "ApprovalsReviewer".to_string()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        string_enum_schema_with_description(
            &["user", "auto_review", "guardian_subagent"],
            "Configures who approval requests are routed to for review. Examples include sandbox escapes, blocked network access, MCP approval prompts, and ARC escalations. Defaults to `user`. `auto_review` uses a carefully prompted subagent to gather relevant context and apply a risk-based decision framework before approving or denying the request. The legacy value `guardian_subagent` is accepted for compatibility.",
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ShellEnvironmentPolicyInherit {
    /// "Core" environment variables for the platform. On UNIX, this would
    /// include HOME, LOGNAME, PATH, SHELL, and USER, among others.
    Core,

    /// Inherits the full environment from the parent process.
    #[default]
    All,

    /// Do not inherit any environment variables from the parent process.
    None,
}

pub type EnvironmentVariablePattern = WildMatchPattern<'*', '?'>;

/// Deriving the `env` based on this policy works as follows:
/// 1. Create an initial map based on the `inherit` policy.
/// 2. If `ignore_default_excludes` is false, filter the map using the default
///    exclude pattern(s), which are: `"*KEY*"`, `"*SECRET*"`, and `"*TOKEN*"`.
/// 3. If `exclude` is not empty, filter the map using the provided patterns.
/// 4. Insert any entries from `r#set` into the map.
/// 5. If non-empty, filter the map using the `include_only` patterns.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellEnvironmentPolicy {
    /// Starting point when building the environment.
    pub inherit: ShellEnvironmentPolicyInherit,

    /// True to skip the check to exclude default environment variables that
    /// contain "KEY", "SECRET", or "TOKEN" in their name. Defaults to true.
    pub ignore_default_excludes: bool,

    /// Environment variable names to exclude from the environment.
    pub exclude: Vec<EnvironmentVariablePattern>,

    /// (key, value) pairs to insert in the environment.
    pub r#set: HashMap<String, String>,

    /// Environment variable names to retain in the environment.
    pub include_only: Vec<EnvironmentVariablePattern>,

    /// If true, the shell profile will be used to run the command.
    pub use_profile: bool,
}

impl Default for ShellEnvironmentPolicy {
    fn default() -> Self {
        Self {
            inherit: ShellEnvironmentPolicyInherit::All,
            ignore_default_excludes: true,
            exclude: Vec::new(),
            r#set: HashMap::new(),
            include_only: Vec::new(),
            use_profile: false,
        }
    }
}

fn string_enum_schema_with_description(values: &[&str], description: &str) -> Schema {
    let mut schema = SchemaObject {
        instance_type: Some(InstanceType::String.into()),
        metadata: Some(Box::new(Metadata {
            description: Some(description.to_string()),
            ..Default::default()
        })),
        ..Default::default()
    };
    schema.enum_values = Some(
        values
            .iter()
            .map(|value| Value::String((*value).to_string()))
            .collect(),
    );
    Schema::Object(schema)
}

#[derive(
    Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Display, JsonSchema, TS,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum WindowsSandboxLevel {
    #[default]
    Disabled,
    RestrictedToken,
    Elevated,
}

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Display,
    JsonSchema,
    TS,
    PartialOrd,
    Ord,
    EnumIter,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Personality {
    None,
    Friendly,
    Pragmatic,
}

#[derive(
    Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Display, JsonSchema, TS, Default,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WebSearchMode {
    Disabled,
    #[default]
    Cached,
    Indexed,
    Live,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Display, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WebSearchContextSize {
    Low,
    Medium,
    High,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq, JsonSchema, TS)]
#[schemars(deny_unknown_fields)]
pub struct WebSearchLocation {
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub timezone: Option<String>,
}

impl WebSearchLocation {
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            country: other.country.clone().or_else(|| self.country.clone()),
            region: other.region.clone().or_else(|| self.region.clone()),
            city: other.city.clone().or_else(|| self.city.clone()),
            timezone: other.timezone.clone().or_else(|| self.timezone.clone()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq, JsonSchema, TS)]
#[schemars(deny_unknown_fields)]
pub struct WebSearchToolConfig {
    pub context_size: Option<WebSearchContextSize>,
    pub allowed_domains: Option<Vec<String>>,
    pub location: Option<WebSearchLocation>,
}

impl WebSearchToolConfig {
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            context_size: other.context_size.or(self.context_size),
            allowed_domains: other
                .allowed_domains
                .clone()
                .or_else(|| self.allowed_domains.clone()),
            location: match (&self.location, &other.location) {
                (Some(location), Some(other_location)) => Some(location.merge(other_location)),
                (Some(location), None) => Some(location.clone()),
                (None, Some(other_location)) => Some(other_location.clone()),
                (None, None) => None,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq, JsonSchema, TS)]
#[schemars(deny_unknown_fields)]
pub struct WebSearchFilters {
    pub allowed_domains: Option<Vec<String>>,
}

#[derive(
    Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq, Display, JsonSchema, TS,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WebSearchUserLocationType {
    #[default]
    Approximate,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq, JsonSchema, TS)]
#[schemars(deny_unknown_fields)]
pub struct WebSearchUserLocation {
    #[serde(default)]
    pub r#type: WebSearchUserLocationType,
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq, JsonSchema, TS)]
#[schemars(deny_unknown_fields)]
pub struct WebSearchConfig {
    pub filters: Option<WebSearchFilters>,
    pub user_location: Option<WebSearchUserLocation>,
    pub search_context_size: Option<WebSearchContextSize>,
}

impl From<WebSearchLocation> for WebSearchUserLocation {
    fn from(location: WebSearchLocation) -> Self {
        Self {
            r#type: WebSearchUserLocationType::Approximate,
            country: location.country,
            region: location.region,
            city: location.city,
            timezone: location.timezone,
        }
    }
}

impl From<WebSearchToolConfig> for WebSearchConfig {
    fn from(config: WebSearchToolConfig) -> Self {
        Self {
            filters: config
                .allowed_domains
                .map(|allowed_domains| WebSearchFilters {
                    allowed_domains: Some(allowed_domains),
                }),
            user_location: config.location.map(Into::into),
            search_context_size: config.context_size,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Display, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ServiceTier {
    Fast,
    Flex,
}

pub const SERVICE_TIER_DEFAULT_REQUEST_VALUE: &str = "default";

impl ServiceTier {
    pub const fn request_value(self) -> &'static str {
        match self {
            Self::Fast => "priority",
            Self::Flex => "flex",
        }
    }

    pub fn from_request_value(value: &str) -> Option<Self> {
        match value {
            "fast" | "priority" => Some(Self::Fast),
            "flex" => Some(Self::Flex),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Display, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ForcedLoginMethod {
    Chatgpt,
    Api,
}

const DEFAULT_PROVIDER_AUTH_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_PROVIDER_AUTH_REFRESH_INTERVAL_MS: u64 = 300_000;

/// Configuration for obtaining a provider bearer token from a command.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ModelProviderAuthInfo {
    /// Command to execute. Bare names are resolved via `PATH`; paths are resolved against `cwd`.
    pub command: String,

    /// Command arguments.
    #[serde(default)]
    pub args: Vec<String>,

    /// Maximum time to wait for the token command to exit successfully.
    #[serde(default = "default_provider_auth_timeout_ms")]
    pub timeout_ms: NonZeroU64,

    /// Maximum age for the cached token before rerunning the command.
    /// Set to `0` to disable proactive refresh and only rerun after a 401 retry path.
    #[serde(default = "default_provider_auth_refresh_interval_ms")]
    pub refresh_interval_ms: u64,

    /// Working directory used when running the token command.
    #[serde(default = "default_provider_auth_cwd")]
    #[schemars(skip_serializing_if = "is_default_provider_auth_cwd")]
    pub cwd: AbsolutePathBuf,
}

impl ModelProviderAuthInfo {
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms.get())
    }

    pub fn refresh_interval(&self) -> Option<Duration> {
        NonZeroU64::new(self.refresh_interval_ms).map(|value| Duration::from_millis(value.get()))
    }
}

fn default_provider_auth_timeout_ms() -> NonZeroU64 {
    non_zero_u64(
        DEFAULT_PROVIDER_AUTH_TIMEOUT_MS,
        "model_providers.<id>.auth.timeout_ms",
    )
}

fn default_provider_auth_refresh_interval_ms() -> u64 {
    DEFAULT_PROVIDER_AUTH_REFRESH_INTERVAL_MS
}

fn non_zero_u64(value: u64, field_name: &str) -> NonZeroU64 {
    match NonZeroU64::new(value) {
        Some(value) => value,
        None => panic!("{field_name} must be non-zero"),
    }
}

fn default_provider_auth_cwd() -> AbsolutePathBuf {
    let deserializer = serde::de::value::StrDeserializer::<serde::de::value::Error>::new(".");
    if let Ok(cwd) = AbsolutePathBuf::deserialize(deserializer) {
        return cwd;
    }

    match AbsolutePathBuf::current_dir() {
        Ok(cwd) => cwd,
        Err(err) => panic!("provider auth cwd must resolve: {err}"),
    }
}

fn is_default_provider_auth_cwd(path: &AbsolutePathBuf) -> bool {
    path == &default_provider_auth_cwd()
}

#[derive(Deserialize, Debug, Clone, PartialEq, Serialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct UserSavedConfig {
    pub approval_policy: Option<AskForApproval>,
    pub sandbox_mode: Option<SandboxMode>,
    pub sandbox_settings: Option<SandboxSettings>,
    pub forced_chatgpt_workspace_id: Option<String>,
    pub forced_login_method: Option<ForcedLoginMethod>,
    pub model: Option<String>,
    pub model_reasoning_effort: Option<ReasoningEffort>,
    pub model_reasoning_summary: Option<ReasoningSummary>,
    pub model_verbosity: Option<Verbosity>,
    pub tools: Option<Tools>,
    pub profile: Option<String>,
    pub profiles: HashMap<String, Profile>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Serialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub approval_policy: Option<AskForApproval>,
    pub model_reasoning_effort: Option<ReasoningEffort>,
    pub model_reasoning_summary: Option<ReasoningSummary>,
    pub model_verbosity: Option<Verbosity>,
    pub chatgpt_base_url: Option<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Serialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct Tools {
    pub web_search: Option<bool>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Serialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct SandboxSettings {
    #[serde(default)]
    pub writable_roots: Vec<AbsolutePathBuf>,
    pub network_access: Option<bool>,
    pub exclude_tmpdir_env_var: Option<bool>,
    pub exclude_slash_tmp: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type")]
#[ts(export_to = "v2/")]
pub enum ConfigLayerSource {
    /// Managed preferences layer delivered by MDM (macOS only).
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Mdm { domain: String, key: String },

    /// Managed config layer from a file (usually `managed_config.toml`).
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    System {
        /// This is the path to the system config.toml file, though it is not
        /// guaranteed to exist.
        file: AbsolutePathBuf,
    },

    /// Enterprise-managed config layer delivered via the cloud config bundle.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    EnterpriseManaged {
        /// Stable identifier for the delivered layer.
        id: String,

        /// Admin-facing name for the delivered layer. This is surfaced in
        /// diagnostics so users know which cloud layer needs administrator
        /// attention.
        name: String,
    },

    /// User config layer from $CODEX_HOME/config.toml. This layer is special
    /// in that it is expected to be:
    /// - writable by the user
    /// - generally outside the workspace directory
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    User {
        /// This is the path to the user's config.toml file, though it is not
        /// guaranteed to exist.
        file: AbsolutePathBuf,
        #[serde(default)]
        profile: Option<ProfileV2Name>,
    },

    /// Path to a .codex/ folder within a project. There could be multiple of
    /// these between `cwd` and the project/repo root.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Project { dot_codex_folder: AbsolutePathBuf },

    /// Session-layer overrides supplied via `-c`/`--config`.
    SessionFlags,

    /// `managed_config.toml` was designed to be a config that was loaded
    /// as the last layer on top of everything else. This scheme did not quite
    /// work out as intended, but we keep this variant as a "best effort" while
    /// we phase out `managed_config.toml` in favor of `requirements.toml`.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    LegacyManagedConfigTomlFromFile { file: AbsolutePathBuf },

    /// Legacy managed config loaded from MDM.
    LegacyManagedConfigTomlFromMdm,
}

impl ConfigLayerSource {
    /// Higher values indicate higher precedence.
    pub fn precedence(&self) -> u8 {
        match self {
            ConfigLayerSource::Mdm { .. } => 0,
            ConfigLayerSource::System { .. } => 10,
            ConfigLayerSource::EnterpriseManaged { .. } => 15,
            ConfigLayerSource::User { .. } => 20,
            ConfigLayerSource::Project { .. } => 25,
            ConfigLayerSource::SessionFlags => 30,
            ConfigLayerSource::LegacyManagedConfigTomlFromFile { .. } => 40,
            ConfigLayerSource::LegacyManagedConfigTomlFromMdm => 50,
        }
    }
}

/// Compares [ConfigLayerSource] by precedence, so `A < B` means settings from
/// B override settings from A.
impl PartialOrd for ConfigLayerSource {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ConfigLayerSource {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.precedence().cmp(&other.precedence())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigLayerMetadata {
    pub name: ConfigLayerSource,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigLayer {
    pub name: ConfigLayerSource,
    pub version: String,
    pub config: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,
}

#[derive(
    Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash, JsonSchema, TS, Ord, PartialOrd,
)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum RealtimeVoice {
    Alloy,
    Arbor,
    Ash,
    Ballad,
    Breeze,
    Cedar,
    Coral,
    Cove,
    Echo,
    Ember,
    Juniper,
    Maple,
    Marin,
    Sage,
    Shimmer,
    Sol,
    Spruce,
    Vale,
    Verse,
}

impl RealtimeVoice {
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Alloy => "alloy",
            Self::Arbor => "arbor",
            Self::Ash => "ash",
            Self::Ballad => "ballad",
            Self::Breeze => "breeze",
            Self::Cedar => "cedar",
            Self::Coral => "coral",
            Self::Cove => "cove",
            Self::Echo => "echo",
            Self::Ember => "ember",
            Self::Juniper => "juniper",
            Self::Maple => "maple",
            Self::Marin => "marin",
            Self::Sage => "sage",
            Self::Shimmer => "shimmer",
            Self::Sol => "sol",
            Self::Spruce => "spruce",
            Self::Vale => "vale",
            Self::Verse => "verse",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RealtimeVoicesList {
    pub v1: Vec<RealtimeVoice>,
    pub v2: Vec<RealtimeVoice>,
    pub default_v1: RealtimeVoice,
    pub default_v2: RealtimeVoice,
}

impl RealtimeVoicesList {
    pub fn builtin() -> Self {
        Self {
            v1: vec![
                RealtimeVoice::Juniper,
                RealtimeVoice::Maple,
                RealtimeVoice::Spruce,
                RealtimeVoice::Ember,
                RealtimeVoice::Vale,
                RealtimeVoice::Breeze,
                RealtimeVoice::Arbor,
                RealtimeVoice::Sol,
                RealtimeVoice::Cove,
            ],
            v2: vec![
                RealtimeVoice::Alloy,
                RealtimeVoice::Ash,
                RealtimeVoice::Ballad,
                RealtimeVoice::Coral,
                RealtimeVoice::Echo,
                RealtimeVoice::Sage,
                RealtimeVoice::Shimmer,
                RealtimeVoice::Verse,
                RealtimeVoice::Marin,
                RealtimeVoice::Cedar,
            ],
            default_v1: RealtimeVoice::Cove,
            default_v2: RealtimeVoice::Marin,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS, EnumIter)]
#[serde(rename_all = "snake_case")]
pub enum HookEventName {
    PreToolUse,
    PermissionRequest,
    PostToolUse,
    PreCompact,
    PostCompact,
    SessionStart,
    UserPromptSubmit,
    SubagentStart,
    SubagentStop,
    Stop,
}

pub type ProfileV2Name = String;

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeConversationVersion {
    V1,
    #[default]
    V2,
}

/// Represents the trust level for a project directory.
/// This determines the approval policy and sandbox mode applied.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Display, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum TrustLevel {
    Trusted,
    Untrusted,
}

/// Controls whether the TUI uses the terminal's alternate screen buffer.
///
/// - `auto` (default): Use alternate screen mode.
/// - `always`: Always use alternate screen mode.
/// - `never`: Never use alternate screen mode. Runs in inline mode, preserving scrollback.
///
/// The CLI flag `--no-alt-screen` can override this setting at runtime.
#[derive(
    Debug, Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Display, JsonSchema, TS,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum AltScreenMode {
    /// Use alternate screen mode.
    #[default]
    Auto,
    /// Always use alternate screen mode.
    Always,
    /// Never use alternate screen (inline mode only).
    Never,
}

/// Initial collaboration mode to use when the TUI starts.
#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema, TS, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum ModeKind {
    Plan,
    #[default]
    #[serde(
        alias = "code",
        alias = "pair_programming",
        alias = "execute",
        alias = "custom"
    )]
    Default,
    #[doc(hidden)]
    #[serde(skip_serializing, skip_deserializing)]
    #[schemars(skip)]
    #[ts(skip)]
    PairProgramming,
    #[doc(hidden)]
    #[serde(skip_serializing, skip_deserializing)]
    #[schemars(skip)]
    #[ts(skip)]
    Execute,
}

pub const TUI_VISIBLE_COLLABORATION_MODES: [ModeKind; 2] = [ModeKind::Default, ModeKind::Plan];

impl ModeKind {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Plan => "Plan",
            Self::Default => "Default",
            Self::PairProgramming => "Pair Programming",
            Self::Execute => "Execute",
        }
    }

    pub const fn is_tui_visible(self) -> bool {
        matches!(self, Self::Plan | Self::Default)
    }

    pub const fn allows_request_user_input(self) -> bool {
        matches!(self, Self::Plan)
    }
}

/// Collaboration mode for a Codex session.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
pub struct CollaborationMode {
    pub mode: ModeKind,
    pub settings: Settings,
}

impl CollaborationMode {
    /// Returns a reference to the settings.
    fn settings_ref(&self) -> &Settings {
        &self.settings
    }

    pub fn model(&self) -> &str {
        self.settings_ref().model.as_str()
    }

    pub fn reasoning_effort(&self) -> Option<ReasoningEffort> {
        self.settings_ref().reasoning_effort
    }

    /// Updates the collaboration mode with new model and/or effort values.
    ///
    /// - `model`: `Some(s)` to update the model, `None` to keep the current model
    /// - `effort`: `Some(Some(e))` to set effort to `e`, `Some(None)` to clear effort, `None` to keep current effort
    /// - `developer_instructions`: `Some(Some(s))` to set instructions, `Some(None)` to clear them, `None` to keep current
    ///
    /// Returns a new `CollaborationMode` with updated values, preserving the mode.
    pub fn with_updates(
        &self,
        model: Option<String>,
        effort: Option<Option<ReasoningEffort>>,
        developer_instructions: Option<Option<String>>,
    ) -> Self {
        let settings = self.settings_ref();
        let updated_settings = Settings {
            model: model.unwrap_or_else(|| settings.model.clone()),
            reasoning_effort: effort.unwrap_or(settings.reasoning_effort),
            developer_instructions: developer_instructions
                .unwrap_or_else(|| settings.developer_instructions.clone()),
        };

        CollaborationMode {
            mode: self.mode,
            settings: updated_settings,
        }
    }

    /// Applies a mask to this collaboration mode, returning a new collaboration mode
    /// with the mask values applied. Fields in the mask that are `Some` will override
    /// the corresponding fields, while `None` values will preserve the original values.
    ///
    /// The `name` field in the mask is ignored as it's metadata for the mask itself.
    pub fn apply_mask(&self, mask: &CollaborationModeMask) -> Self {
        let settings = self.settings_ref();
        CollaborationMode {
            mode: mask.mode.unwrap_or(self.mode),
            settings: Settings {
                model: mask.model.clone().unwrap_or_else(|| settings.model.clone()),
                reasoning_effort: mask.reasoning_effort.unwrap_or(settings.reasoning_effort),
                developer_instructions: mask
                    .developer_instructions
                    .clone()
                    .unwrap_or_else(|| settings.developer_instructions.clone()),
            },
        }
    }
}

/// Settings for a collaboration mode.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema, TS)]
pub struct Settings {
    pub model: String,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub developer_instructions: Option<String>,
}

/// A mask for collaboration mode settings, allowing partial updates.
/// All fields except `name` are optional, enabling selective updates.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema, TS)]
pub struct CollaborationModeMask {
    pub name: String,
    pub mode: Option<ModeKind>,
    pub model: Option<String>,
    pub reasoning_effort: Option<Option<ReasoningEffort>>,
    pub developer_instructions: Option<Option<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn context_budget_mode_from_str_accepts_lowercase_modes() {
        assert_eq!(Ok(ContextBudgetMode::Standard), "standard".parse());
        assert_eq!(Ok(ContextBudgetMode::Slow), "slow".parse());
        assert_eq!(
            "invalid context_budget_mode: fast",
            "fast".parse::<ContextBudgetMode>().unwrap_err()
        );
    }

    #[test]
    fn apply_mask_can_clear_optional_fields() {
        let mode = CollaborationMode {
            mode: ModeKind::Default,
            settings: Settings {
                model: "gpt-5.2-codex".to_string(),
                reasoning_effort: Some(ReasoningEffort::High),
                developer_instructions: Some("stay focused".to_string()),
            },
        };
        let mask = CollaborationModeMask {
            name: "Clear".to_string(),
            mode: None,
            model: None,
            reasoning_effort: Some(None),
            developer_instructions: Some(None),
        };

        let expected = CollaborationMode {
            mode: ModeKind::Default,
            settings: Settings {
                model: "gpt-5.2-codex".to_string(),
                reasoning_effort: None,
                developer_instructions: None,
            },
        };
        assert_eq!(expected, mode.apply_mask(&mask));
    }

    #[test]
    fn mode_kind_deserializes_alias_values_to_default() {
        for alias in ["code", "pair_programming", "execute", "custom"] {
            let json = format!("\"{alias}\"");
            let mode: ModeKind = serde_json::from_str(&json).expect("deserialize mode");
            assert_eq!(ModeKind::Default, mode);
        }
    }

    #[test]
    fn approvals_reviewer_serializes_auto_review_and_accepts_legacy_guardian_subagent() {
        assert_eq!(ApprovalsReviewer::User.to_string(), "user");
        assert_eq!(
            serde_json::to_string(&ApprovalsReviewer::User).expect("serialize reviewer"),
            "\"user\""
        );
        assert_eq!(
            serde_json::to_string(&ApprovalsReviewer::AutoReview).expect("serialize reviewer"),
            "\"guardian_subagent\""
        );

        for value in ["user", "auto_review", "guardian_subagent"] {
            let json = format!("\"{value}\"");
            let reviewer: ApprovalsReviewer =
                serde_json::from_str(&json).expect("deserialize reviewer");
            let expected = if value == "user" {
                ApprovalsReviewer::User
            } else {
                ApprovalsReviewer::AutoReview
            };
            assert_eq!(expected, reviewer);
        }
    }

    #[test]
    fn tui_visible_collaboration_modes_match_mode_kind_visibility() {
        let expected = [ModeKind::Default, ModeKind::Plan];
        assert_eq!(expected, TUI_VISIBLE_COLLABORATION_MODES);

        for mode in TUI_VISIBLE_COLLABORATION_MODES {
            assert!(mode.is_tui_visible());
        }

        assert!(!ModeKind::PairProgramming.is_tui_visible());
        assert!(!ModeKind::Execute.is_tui_visible());
    }

    #[test]
    fn web_search_location_merge_prefers_overlay_values() {
        let base = WebSearchLocation {
            country: Some("US".to_string()),
            region: Some("CA".to_string()),
            city: None,
            timezone: Some("America/Los_Angeles".to_string()),
        };
        let overlay = WebSearchLocation {
            country: None,
            region: Some("WA".to_string()),
            city: Some("Seattle".to_string()),
            timezone: None,
        };

        let expected = WebSearchLocation {
            country: Some("US".to_string()),
            region: Some("WA".to_string()),
            city: Some("Seattle".to_string()),
            timezone: Some("America/Los_Angeles".to_string()),
        };

        assert_eq!(expected, base.merge(&overlay));
    }

    #[test]
    fn web_search_tool_config_merge_prefers_overlay_values() {
        let base = WebSearchToolConfig {
            context_size: Some(WebSearchContextSize::Low),
            allowed_domains: Some(vec!["openai.com".to_string()]),
            location: Some(WebSearchLocation {
                country: Some("US".to_string()),
                region: Some("CA".to_string()),
                city: None,
                timezone: Some("America/Los_Angeles".to_string()),
            }),
        };
        let overlay = WebSearchToolConfig {
            context_size: Some(WebSearchContextSize::High),
            allowed_domains: None,
            location: Some(WebSearchLocation {
                country: None,
                region: Some("WA".to_string()),
                city: Some("Seattle".to_string()),
                timezone: None,
            }),
        };

        let expected = WebSearchToolConfig {
            context_size: Some(WebSearchContextSize::High),
            allowed_domains: Some(vec!["openai.com".to_string()]),
            location: Some(WebSearchLocation {
                country: Some("US".to_string()),
                region: Some("WA".to_string()),
                city: Some("Seattle".to_string()),
                timezone: Some("America/Los_Angeles".to_string()),
            }),
        };

        assert_eq!(expected, base.merge(&overlay));
    }
}
