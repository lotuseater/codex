use super::*;

/// Compatibility-only config retained so legacy `ghost_snapshot` settings
/// continue to load even though snapshots are no longer produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostSnapshotConfig {
    pub ignore_large_untracked_files: Option<i64>,
    pub ignore_large_untracked_dirs: Option<i64>,
    pub disable_warnings: bool,
}

impl Default for GhostSnapshotConfig {
    fn default() -> Self {
        Self {
            ignore_large_untracked_files: Some(DEFAULT_IGNORE_LARGE_UNTRACKED_FILES),
            ignore_large_untracked_dirs: Some(DEFAULT_IGNORE_LARGE_UNTRACKED_DIRS),
            disable_warnings: false,
        }
    }
}

/// Configured thread persistence backend.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ThreadStoreConfig {
    /// Persist threads locally using rollout JSONL files and sqlite metadata.
    #[default]
    Local,
    /// In-memory thread store for test and debug configurations.
    InMemory { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MultiAgentV2Config {
    pub max_concurrent_threads_per_session: usize,
    pub min_wait_timeout_ms: i64,
    pub max_wait_timeout_ms: i64,
    pub default_wait_timeout_ms: i64,
    pub usage_hint_enabled: bool,
    pub usage_hint_text: Option<String>,
    /// Root-agent hint text override. When `None`, the hint is generated at
    /// runtime from [`plan_token_economy_delegation_k`] so that a
    /// `/delegate-prompt k <n>` change takes effect next turn without
    /// persisting frozen text.
    pub root_agent_usage_hint_text: Option<String>,
    /// Subagent hint text override. Same deferred-generation semantics as
    /// [`root_agent_usage_hint_text`].
    pub subagent_usage_hint_text: Option<String>,
    /// Minimum estimated token cost for a subtask before delegation is allowed.
    /// Used by the plan-token-economy prompt injection. Default: 26 000.
    pub plan_token_economy_delegation_k: usize,
    pub hide_spawn_agent_metadata: bool,
    pub non_code_mode_only: bool,
    /// Optional namespace under which multi-agent v2 spawn tools are exposed
    /// when namespace tools are enabled. `None` exposes them un-namespaced.
    pub tool_namespace: Option<String>,
}

pub fn default_multi_agent_v2_root_usage_hint_text() -> String {
    crate::agent::policy::default_multi_agent_v2_root_usage_hint_text()
}

// Keep the full multi-agent planning and worker guidance in codex-agent-policy so
// config stays a thin adapter while the prompt policy remains independently tested.
// The moved root prompt still includes the useful planning tips that used to live
// here: planner/overseer ownership, Agent ROI / Delegation / Work Split, reusable
// high-capability helpers, first_moves/context-scout routing, bounded context
// contracts, compact/clear guidance, root-owned finalization, and active helper
// oversight. The moved subagent prompt keeps worker boundaries, short handoffs,
// threshold-gated recursive delegation, and no-revert/no-finalization rules.
pub fn default_multi_agent_v2_subagent_usage_hint_text() -> String {
    crate::agent::policy::default_multi_agent_v2_subagent_usage_hint_text()
}

impl Default for MultiAgentV2Config {
    fn default() -> Self {
        Self {
            max_concurrent_threads_per_session:
                DEFAULT_MULTI_AGENT_V2_MAX_CONCURRENT_THREADS_PER_SESSION,
            min_wait_timeout_ms: DEFAULT_MULTI_AGENT_V2_MIN_WAIT_TIMEOUT_MS,
            max_wait_timeout_ms: DEFAULT_MULTI_AGENT_V2_MAX_WAIT_TIMEOUT_MS,
            default_wait_timeout_ms: DEFAULT_MULTI_AGENT_V2_DEFAULT_WAIT_TIMEOUT_MS,
            usage_hint_enabled: true,
            usage_hint_text: None,
            // Default to None so the hint is generated at runtime from
            // plan_token_economy_delegation_k, enabling /delegate-prompt k <n>
            // to take effect next turn without persisting frozen text.
            root_agent_usage_hint_text: None,
            subagent_usage_hint_text: None,
            plan_token_economy_delegation_k:
                crate::agent::policy::DEFAULT_PLAN_TOKEN_ECONOMY_DELEGATION_K,
            hide_spawn_agent_metadata: false,
            non_code_mode_only: false,
            tool_namespace: None,
        }
    }
}

/// Resolved shared rollout token-budget configuration.
///
/// This is the resolved counterpart of [`codex_features::RolloutBudgetConfigToml`]:
/// the fork keeps resolved config types (`crate::config::*`) separate from the raw
/// TOML feature types (`codex_features::*Toml`). Resolution lives in
/// [`super::resolved::resolve_rollout_budget_config`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RolloutBudgetConfig {
    pub limit_tokens: i64,
    pub reminder_interval_tokens: i64,
    pub sampling_token_weight: f64,
    pub prefill_token_weight: f64,
}

/// Resolved current-time reminder configuration.
///
/// Resolved counterpart of [`codex_features::CurrentTimeReminderConfigToml`].
/// Resolution lives in [`super::resolved::resolve_current_time_reminder_config`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CurrentTimeReminderConfig {
    pub reminder_interval_model_requests: u64,
    pub clock_source: CurrentTimeSource,
}

impl Default for CurrentTimeReminderConfig {
    fn default() -> Self {
        Self {
            reminder_interval_model_requests: 1,
            clock_source: CurrentTimeSource::System,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DesktopAutomationConfig {
    pub enabled: bool,
    pub proactive: bool,
    pub allow_input: bool,
    pub prefer_app_harness: bool,
}

impl Default for DesktopAutomationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            proactive: true,
            allow_input: true,
            prefer_app_harness: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalResizeReflowMaxRows {
    /// Use the runtime terminal detector to choose a scrollback-sized cap.
    #[default]
    Auto,
    /// Keep all rendered transcript rows during resize reflow.
    Disabled,
    /// Keep at most this many rendered transcript rows during resize reflow.
    Limit(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerminalResizeReflowConfig {
    pub max_rows: TerminalResizeReflowMaxRows,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentRoleConfig {
    /// Human-facing role documentation used in spawn tool guidance.
    /// Required for loaded user-defined roles after deprecated/new metadata precedence resolves.
    pub description: Option<String>,
    /// Path to a role-specific config layer.
    pub config_file: Option<PathBuf>,
    /// Candidate nicknames for agents spawned with this role.
    pub nickname_candidates: Option<Vec<String>>,
}

/// Optional overrides for user configuration (e.g., from CLI flags).
#[derive(Default, Debug, Clone)]
pub struct ConfigOverrides {
    pub model: Option<String>,
    pub review_model: Option<String>,
    pub cwd: Option<PathBuf>,
    pub approval_policy: Option<AskForApproval>,
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    pub sandbox_mode: Option<SandboxMode>,
    pub permission_profile: Option<PermissionProfile>,
    pub default_permissions: Option<String>,
    pub model_provider: Option<String>,
    pub service_tier: Option<Option<String>>,
    pub context_budget_mode: Option<ContextBudgetMode>,
    pub config_profile: Option<String>,
    pub codex_self_exe: Option<PathBuf>,
    pub codex_linux_sandbox_exe: Option<PathBuf>,
    pub main_execve_wrapper_exe: Option<PathBuf>,
    pub default_zsh_path: Option<AbsolutePathBuf>,
    pub base_instructions: Option<String>,
    pub developer_instructions: Option<String>,
    pub personality: Option<Personality>,
    pub compact_prompt: Option<String>,
    pub show_raw_agent_reasoning: Option<bool>,
    pub tools_web_search_request: Option<bool>,
    pub ephemeral: Option<bool>,
    pub bypass_hook_trust: Option<bool>,
    /// Additional directories that should be treated as writable roots for this session.
    pub additional_writable_roots: Vec<PathBuf>,
    /// Explicit workspace roots for this session. When set, these replace the
    /// default cwd-derived workspace roots.
    pub workspace_roots: Option<Vec<PathBuf>>,
}
