use crate::FeatureConfig;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodeModeConfigToml {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Exact tool namespaces to omit from the code-mode nested tool surface.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_tool_namespaces: Option<Vec<String>>,
    /// Exact tool namespaces to expose only as direct model tools.
    /// These tools bypass deferral, remain top-level in code-mode-only sessions, and are omitted
    /// from the nested code-mode tool surface.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direct_only_tool_namespaces: Option<Vec<String>>,
}

impl FeatureConfig for CodeModeConfigToml {
    fn enabled(&self) -> Option<bool> {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = Some(enabled);
    }
}

/// Raw TOML mirror of the resolved `UsageHintCadence`
/// (`codex_core::config::UsageHintCadence`, which is serialize-only). This is the
/// deserialize-able counterpart read from `features.multi_agent_v2`; it is mapped
/// onto the resolved enum in `resolve_multi_agent_v2_config`. Mirrors the
/// derive/`rename_all` style of [`CurrentTimeSource`].
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UsageHintCadenceToml {
    /// Today's behavior: surface the hint only via the initial-context /
    /// compaction / plan-entry paths; never re-inject per model request.
    #[default]
    InitialContext,
    /// Plan-entry paths only (no per-request re-injection within the turn loop).
    Plan,
    /// Re-inject once every `usage_hint_reminder_interval` model requests.
    EveryN,
    /// Re-inject on every model request within a turn.
    Always,
}

/// Raw TOML mirror of the resolved `AutoCoordinatorMode`
/// (`codex_core::config::AutoCoordinatorMode`, which is serialize-only). The
/// deserialize-able counterpart read from `features.multi_agent_v2.auto_coordinator`;
/// it is mapped onto the resolved enum in `resolve_multi_agent_v2_config`. Mirrors
/// the derive/`rename_all` style of [`UsageHintCadenceToml`]; parses
/// `off` / `auto` / `always`.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AutoCoordinatorModeToml {
    /// Never inject the auto-coordinator framing.
    Off,
    /// Inject only when the local decomposability heuristic judges the task
    /// worth decomposing.
    #[default]
    Auto,
    /// Inject whenever multi-agent V2 is enabled.
    Always,
}

/// Raw TOML mirror of the resolved `DelegationInjectionRole`
/// (`codex_core::config::DelegationInjectionRole`, which is serialize-only). The
/// deserialize-able counterpart read from
/// `features.multi_agent_v2.delegation_injection_role`; it is mapped onto the
/// resolved enum in `resolve_multi_agent_v2_config`. Mirrors the
/// derive/`rename_all` style of [`AutoCoordinatorModeToml`]; parses
/// `user` / `developer`.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DelegationInjectionRoleToml {
    /// Deliver the delegation nudges as `user`-role messages (obeyed by the
    /// model).
    #[default]
    User,
    /// Deliver the delegation nudges as `developer`-role messages (the prior
    /// behavior, discounted by the model).
    Developer,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MultiAgentV2ConfigToml {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1))]
    pub max_concurrent_threads_per_session: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0, max = 3600000))]
    pub min_wait_timeout_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0, max = 3600000))]
    pub max_wait_timeout_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0, max = 3600000))]
    pub default_wait_timeout_ms: Option<i64>,
    /// Deprecated compatibility field. Its value is ignored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_hint_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_hint_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_agent_usage_hint_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_usage_hint_text: Option<String>,
    /// Minimum estimated token cost for a subtask before delegation is allowed.
    /// Used by the plan-token-economy prompt injection. Defaults to 26 000.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_token_economy_delegation_k: Option<usize>,
    /// Cadence governing whether the multi-agent delegation usage hint may be
    /// re-injected within a single long turn. Mirrors the resolved
    /// `UsageHintCadence`. Absent (or `initial_context`) preserves today's
    /// behavior: no per-model-request re-injection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_hint_cadence: Option<UsageHintCadenceToml>,
    /// When `usage_hint_cadence` is `every_n`, the number of model requests
    /// between usage-hint re-injections. Ignored by the other cadences.
    /// Defaults to 5.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_hint_reminder_interval: Option<u64>,
    /// Governs automatic injection of the coordinator framing at the start of a
    /// fresh, decomposable user turn. Mirrors the resolved `AutoCoordinatorMode`:
    /// `off` never injects, `auto` (default) injects only when the local
    /// decomposability heuristic fires, `always` injects whenever multi-agent V2
    /// is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_coordinator: Option<AutoCoordinatorModeToml>,
    /// Role under which the delegation nudges (auto-coordinator framing and the
    /// usage hint) are injected to the model. Mirrors the resolved
    /// `DelegationInjectionRole`: `user` (default) delivers them as user-role
    /// messages so the model acts on them; `developer` preserves the prior
    /// developer-role path for control/rollback.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegation_injection_role: Option<DelegationInjectionRoleToml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 64), regex(pattern = r"^[a-zA-Z0-9_-]+$"))]
    pub tool_namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_spawn_agent_metadata: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_code_mode_only: Option<bool>,
}

impl FeatureConfig for MultiAgentV2ConfigToml {
    fn enabled(&self) -> Option<bool> {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = Some(enabled);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TokenBudgetConfigToml {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Number of tokens remaining before auto-compaction when the wrap-up reminder is emitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1))]
    pub reminder_threshold_tokens: Option<i64>,
    /// Reminder template. `{n_remaining}` is replaced with the tokens remaining before
    /// auto-compaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 1000))]
    pub reminder_message_template: Option<String>,
}

impl FeatureConfig for TokenBudgetConfigToml {
    fn enabled(&self) -> Option<bool> {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = Some(enabled);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RolloutBudgetConfigToml {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1))]
    pub limit_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Remaining weighted-token values that trigger reminders when crossed.
    pub reminder_at_remaining_tokens: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0.0))]
    pub sampling_token_weight: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0.0))]
    pub prefill_token_weight: Option<f64>,
}

impl FeatureConfig for RolloutBudgetConfigToml {
    fn enabled(&self) -> Option<bool> {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = Some(enabled);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CurrentTimeSource {
    #[default]
    System,
    External,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CurrentTimeReminderConfigToml {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1))]
    pub reminder_interval_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clock_source: Option<CurrentTimeSource>,
}

impl FeatureConfig for CurrentTimeReminderConfigToml {
    fn enabled(&self) -> Option<bool> {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = Some(enabled);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RemovedAppsMcpPathOverrideConfigToml {
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NetworkProxyConfigToml {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_socks5: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socks_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_socks5_udp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_upstream_proxy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dangerously_allow_non_loopback_proxy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dangerously_allow_all_unix_sockets: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<NetworkProxyModeToml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domains: Option<BTreeMap<String, NetworkProxyDomainPermissionToml>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unix_sockets: Option<BTreeMap<String, NetworkProxyUnixSocketPermissionToml>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_local_binding: Option<bool>,
}

impl FeatureConfig for NetworkProxyConfigToml {
    fn enabled(&self) -> Option<bool> {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = Some(enabled);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum NetworkProxyModeToml {
    Limited,
    Full,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum NetworkProxyDomainPermissionToml {
    Allow,
    Deny,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum NetworkProxyUnixSocketPermissionToml {
    Allow,
    Deny,
}
