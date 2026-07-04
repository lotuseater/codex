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

/// Cadence for re-surfacing the multi-agent delegation usage hint within a turn.
///
/// Mirrors the derive/`#[default]` style of [`ThreadStoreConfig`]. Note that the
/// resolved config enums in this module are serialize-only (the raw TOML enums
/// live in `codex_features`); this enum derives `Serialize` because its owner
/// [`MultiAgentV2Config`] derives `Serialize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageHintCadence {
    /// Today's behavior: surface the hint only via the initial-context /
    /// compaction / plan-entry paths; never re-inject per model request.
    #[default]
    InitialContext,
    /// Surface the hint on plan-entry paths (no per-request re-injection in the
    /// turn loop). Reserved for plan-scoped delivery.
    Plan,
    /// Re-inject the hint once every `usage_hint_reminder_interval` model
    /// requests within a turn.
    EveryN,
    /// Re-inject the hint on every model request within a turn.
    Always,
}

/// Whether the auto-coordinator framing is injected at the start of a fresh,
/// decomposable user turn.
///
/// Mirrors the derive/`#[default]` style of [`UsageHintCadence`]; serialize-only
/// (the raw TOML enum lives in `codex_features::AutoCoordinatorModeToml`). The
/// default [`AutoCoordinatorMode::Auto`] injects only when the local
/// decomposability heuristic fires, so a stray injection on small work is
/// harmless.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoCoordinatorMode {
    /// Never inject the coordinator framing.
    Off,
    /// Inject only when the local decomposability heuristic
    /// ([`codex_agent_policy::task_looks_decomposable`]) judges the task worth
    /// decomposing. Conservative: small or monolithic tasks are left untouched.
    #[default]
    Auto,
    /// Inject whenever multi-agent V2 is enabled, regardless of the heuristic.
    Always,
}

/// Role under which the delegation nudges (the auto-coordinator framing and the
/// multi-agent usage hint) are delivered to the model. Delivered as `user`-role
/// messages they are obeyed; delivered as `developer`-role messages (the legacy
/// path) they are discounted by the model.
///
/// Mirrors the derive/`#[default]` style of [`AutoCoordinatorMode`]; serialize-only
/// (the raw TOML enum lives in `codex_features::DelegationInjectionRoleToml`). The
/// default [`DelegationInjectionRole::User`] delivers the nudges as user-role so
/// the model obeys them; `Developer` preserves the prior behavior for
/// control/rollback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegationInjectionRole {
    /// Deliver the delegation nudges as `user`-role messages, which the model
    /// obeys. Default.
    #[default]
    User,
    /// Deliver the delegation nudges as `developer`-role messages (the prior
    /// behavior, discounted by the model). Retained for control/rollback.
    Developer,
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
    /// Cadence governing whether the multi-agent delegation usage hint may be
    /// re-injected within a single long turn (decoupled mechanism mirroring the
    /// current-time reminder). Default [`UsageHintCadence::InitialContext`]
    /// preserves today's behavior: the hint is surfaced only by the
    /// initial-context / compaction / plan-entry paths, never re-injected
    /// per-model-request.
    pub usage_hint_cadence: UsageHintCadence,
    /// When [`usage_hint_cadence`] is [`UsageHintCadence::EveryN`], the number of
    /// model requests between usage-hint re-injections. Ignored by the other
    /// cadences. Default: 5.
    pub usage_hint_reminder_interval: u64,
    /// Governs automatic injection of the coordinator framing at the start of a
    /// fresh, decomposable user turn (see [`AutoCoordinatorMode`]). Default
    /// [`AutoCoordinatorMode::Auto`] injects only when the local heuristic judges
    /// the task decomposable. Gated by the same multi-agent V2 enable as the
    /// usage hint.
    pub auto_coordinator: AutoCoordinatorMode,
    /// Role under which the delegation nudges (auto-coordinator framing and the
    /// usage hint) are injected to the model (see [`DelegationInjectionRole`]).
    /// Default [`DelegationInjectionRole::User`] delivers them as user-role
    /// messages so the model acts on them; `Developer` preserves the prior
    /// developer-role path for control/rollback.
    pub delegation_injection_role: DelegationInjectionRole,
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
            // Default cadence preserves today's behavior (no per-request
            // re-injection); flipping this is a separate, later decision.
            usage_hint_cadence: UsageHintCadence::InitialContext,
            usage_hint_reminder_interval: 5,
            // Default Auto: inject the coordinator framing only when the local
            // decomposability heuristic fires (conservative, beneficial-only).
            auto_coordinator: AutoCoordinatorMode::Auto,
            // Default User: deliver the delegation nudges as user-role messages
            // so the model acts on them (developer-role is the legacy path).
            delegation_injection_role: DelegationInjectionRole::User,
            hide_spawn_agent_metadata: false,
            non_code_mode_only: false,
            tool_namespace: None,
        }
    }
}

/// Pure gate over the resolved [`AutoCoordinatorMode`] plus the user prompt.
///
/// Exhaustive (no catch-all) so a newly added mode variant fails to compile here
/// until it is handled. `Auto` defers to the decomposability heuristic, which is
/// owned by `codex-agent-policy` so the policy stays independently testable.
fn auto_coordinator_should_inject(mode: AutoCoordinatorMode, prompt: &str) -> bool {
    match mode {
        AutoCoordinatorMode::Off => false,
        AutoCoordinatorMode::Always => true,
        AutoCoordinatorMode::Auto => codex_agent_policy::task_looks_decomposable(prompt),
    }
}

impl MultiAgentV2Config {
    /// Whether the auto-coordinator framing should be injected for `prompt`,
    /// given this config's [`AutoCoordinatorMode`]. The caller owns the
    /// multi-agent V2 enable gate (mirroring the usage-hint seam); this method
    /// only evaluates the mode plus heuristic.
    pub fn should_inject_auto_coordinator(&self, prompt: &str) -> bool {
        auto_coordinator_should_inject(self.auto_coordinator, prompt)
    }

    /// Whether auto-coordination is active (mode is not `Off`). A fresh root
    /// session consults this to start in `Proactive` multi-agent mode instead of
    /// the default `ExplicitRequestOnly` suppressor, so unprompted delegation is
    /// permitted whenever auto-coordination is enabled. Distinct from
    /// [`Self::should_inject_auto_coordinator`], which additionally consults the
    /// per-prompt decomposability heuristic under `Auto`.
    pub fn auto_coordinator_active(&self) -> bool {
        self.auto_coordinator != AutoCoordinatorMode::Off
    }

    /// Whether the delegation nudges should be delivered to the model as
    /// `user`-role messages (obeyed) rather than `developer`-role messages
    /// (discounted). This is the seam consumed by the injection path: `true`
    /// selects the user-role delivery, `false` keeps the prior developer-role
    /// delivery unchanged (control/rollback).
    pub fn inject_delegation_as_user(&self) -> bool {
        self.delegation_injection_role == DelegationInjectionRole::User
    }
}

#[cfg(test)]
mod auto_coordinator_tests {
    use super::*;

    #[test]
    fn auto_coordinator_mode_defaults_to_auto() {
        assert_eq!(AutoCoordinatorMode::default(), AutoCoordinatorMode::Auto);
        assert_eq!(
            MultiAgentV2Config::default().auto_coordinator,
            AutoCoordinatorMode::Auto
        );
    }

    #[test]
    fn auto_coordinator_gate_honors_mode_and_heuristic() {
        // Long, clearly multi-deliverable prompt: passes the agent-policy
        // decomposability heuristic (length floor + enumeration + build/plural
        // signals). Kept as one literal so rustfmt leaves it untouched.
        let decomposable = "Build a multi-module game engine. Implement several independent components, each owning its own files so the work can be split across multiple parallel workers:\n- a rendering module that draws sprites and text to the screen\n- an input module that maps keyboard and gamepad events to actions\n- a physics module that integrates velocities and resolves collisions\n- a level loader that parses map files and instantiates entities\n- a score system that tracks points, high scores, and persistence\n- an audio module that mixes sound effects and background music\nEach component is a separate file and can be developed and tested in isolation, then integrated together at the end. Add unit tests for every module and verify the whole engine runs.";
        let trivial = "fix this typo";

        // Off never injects, even for a decomposable task.
        assert!(!auto_coordinator_should_inject(
            AutoCoordinatorMode::Off,
            decomposable
        ));
        // Always injects, even for a trivial task.
        assert!(auto_coordinator_should_inject(
            AutoCoordinatorMode::Always,
            trivial
        ));
        // Auto defers to the decomposability heuristic.
        assert!(auto_coordinator_should_inject(
            AutoCoordinatorMode::Auto,
            decomposable
        ));
        assert!(!auto_coordinator_should_inject(
            AutoCoordinatorMode::Auto,
            trivial
        ));
    }
}

/// Resolved stream-resilience (extended auto-retry) configuration.
///
/// Fork-owned. Governs the bounded extended-retry that re-enters the turn loop
/// after the built-in per-stream retries (`stream_max_retries`, ~6s total) are
/// exhausted on a transient `CodexErr::Stream` disconnect, giving connectivity a
/// longer window to recover before the turn is surfaced as an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StreamResilienceConfig {
    /// Master switch (default `true`). When enabled, a transient stream
    /// disconnect that survives the built-in retries triggers up to
    /// `max_extended_waits` additional long-interval retries before the turn
    /// fails.
    pub auto_retry_enabled: bool,
    /// Number of extended waits attempted beyond the built-in stream retries.
    /// Default `3`.
    pub max_extended_waits: u64,
    /// Delay, in seconds, of each extended wait. Default `60`.
    pub extended_wait_secs: u64,
}

impl Default for StreamResilienceConfig {
    fn default() -> Self {
        Self {
            auto_retry_enabled: true,
            max_extended_waits: 3,
            extended_wait_secs: 60,
        }
    }
}

/// Resolved shared rollout token-budget configuration.
///
/// This is the resolved counterpart of [`codex_features::RolloutBudgetConfigToml`]:
/// the fork keeps resolved config types (`crate::config::*`) separate from the raw
/// TOML feature types (`codex_features::*Toml`). Resolution lives in
/// [`super::resolved::resolve_rollout_budget_config`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RolloutBudgetConfig {
    pub limit_tokens: i64,
    pub reminder_at_remaining_tokens: Vec<i64>,
    pub sampling_token_weight: f64,
    pub prefill_token_weight: f64,
}

/// Resolved current-time reminder configuration.
///
/// Resolved counterpart of [`codex_features::CurrentTimeReminderConfigToml`].
/// Resolution lives in [`super::resolved::resolve_current_time_reminder_config`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CurrentTimeReminderConfig {
    pub reminder_interval_seconds: u64,
    pub clock_source: CurrentTimeSource,
}

impl Default for CurrentTimeReminderConfig {
    fn default() -> Self {
        Self {
            reminder_interval_seconds: 1,
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
