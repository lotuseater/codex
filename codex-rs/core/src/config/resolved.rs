use super::*;

use super::config_types::AutoCoordinatorMode;
use codex_features::AutoCoordinatorModeToml;

pub(crate) fn resolve_tool_suggest_config(
    config_toml: &ConfigToml,
    config_layer_stack: &ConfigLayerStack,
) -> ToolSuggestConfig {
    resolve_tool_suggest_config_from_config(config_toml.tool_suggest.as_ref(), config_layer_stack)
}

pub(crate) fn resolve_tool_suggest_config_from_layer_stack(
    config_layer_stack: &ConfigLayerStack,
) -> ToolSuggestConfig {
    let tool_suggest = config_layer_stack
        .effective_config()
        .get("tool_suggest")
        .cloned()
        .and_then(|value| value.try_into::<ToolSuggestConfig>().ok());
    resolve_tool_suggest_config_from_config(tool_suggest.as_ref(), config_layer_stack)
}

pub(crate) fn resolve_tool_suggest_config_from_config(
    tool_suggest: Option<&ToolSuggestConfig>,
    config_layer_stack: &ConfigLayerStack,
) -> ToolSuggestConfig {
    let discoverables = tool_suggest
        .into_iter()
        .flat_map(|tool_suggest| tool_suggest.discoverables.iter())
        .filter_map(|discoverable| {
            let trimmed = discoverable.id.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(ToolSuggestDiscoverable {
                    kind: discoverable.kind,
                    id: trimmed.to_string(),
                })
            }
        })
        .collect();
    let mut seen_disabled_tools = HashSet::new();
    let mut disabled_tools = Vec::new();
    let mut add_disabled_tool = |disabled_tool: ToolSuggestDisabledTool| {
        if let Some(disabled_tool) = disabled_tool.normalized()
            && seen_disabled_tools.insert(disabled_tool.clone())
        {
            disabled_tools.push(disabled_tool);
        }
    };

    let layers = config_layer_stack.get_layers(
        ConfigLayerStackOrdering::LowestPrecedenceFirst,
        /*include_disabled*/ false,
    );
    if layers.is_empty() {
        for disabled_tool in tool_suggest
            .into_iter()
            .flat_map(|tool_suggest| tool_suggest.disabled_tools.iter().cloned())
        {
            add_disabled_tool(disabled_tool);
        }
    } else {
        for layer in layers {
            let Some(tool_suggest) = layer
                .config
                .get("tool_suggest")
                .cloned()
                .and_then(|value| value.try_into::<ToolSuggestConfig>().ok())
            else {
                continue;
            };
            for disabled_tool in tool_suggest.disabled_tools {
                add_disabled_tool(disabled_tool);
            }
        }
    }

    ToolSuggestConfig {
        discoverables,
        disabled_tools,
    }
}

pub(crate) fn thread_store_config(thread_store: Option<ThreadStoreToml>) -> ThreadStoreConfig {
    match thread_store {
        Some(ThreadStoreToml::Local {}) => ThreadStoreConfig::Local,
        Some(ThreadStoreToml::InMemory { id }) => ThreadStoreConfig::InMemory { id },
        None => ThreadStoreConfig::Local,
    }
}

/// Resolves the OSS provider from CLI override or global config.
/// Returns `None` if no provider is configured at any level.
pub fn resolve_oss_provider(
    explicit_provider: Option<&str>,
    config_toml: &ConfigToml,
) -> Option<String> {
    if let Some(provider) = explicit_provider {
        // Explicit provider specified (e.g., via --local-provider)
        Some(provider.to_string())
    } else {
        config_toml.oss_provider.clone()
    }
}

/// Resolve the web search mode from explicit config and feature flags.
pub(crate) fn resolve_web_search_mode(
    config_toml: &ConfigToml,
    features: &Features,
) -> Option<WebSearchMode> {
    if let Some(mode) = config_toml.web_search {
        return Some(mode);
    }
    if features.enabled(Feature::WebSearchCached) {
        return Some(WebSearchMode::Cached);
    }
    if features.enabled(Feature::WebSearchRequest) {
        return Some(WebSearchMode::Live);
    }
    None
}

pub(crate) fn resolve_web_search_config(config_toml: &ConfigToml) -> Option<WebSearchConfig> {
    config_toml
        .tools
        .as_ref()
        .and_then(|tools| tools.web_search.as_ref())
        .cloned()
        .map(Into::into)
}

/// Map the raw TOML cadence enum onto the resolved [`UsageHintCadence`].
/// Exhaustive (no catch-all) so a newly added cadence variant fails to compile
/// here until it is mapped, rather than being silently dropped.
fn resolve_usage_hint_cadence(raw: UsageHintCadenceToml) -> UsageHintCadence {
    match raw {
        UsageHintCadenceToml::InitialContext => UsageHintCadence::InitialContext,
        UsageHintCadenceToml::Plan => UsageHintCadence::Plan,
        UsageHintCadenceToml::EveryN => UsageHintCadence::EveryN,
        UsageHintCadenceToml::Always => UsageHintCadence::Always,
    }
}

/// Map the raw TOML auto-coordinator mode onto the resolved
/// [`AutoCoordinatorMode`]. Exhaustive (no catch-all) so a newly added mode
/// variant fails to compile here until it is mapped, mirroring
/// [`resolve_usage_hint_cadence`].
fn resolve_auto_coordinator(raw: AutoCoordinatorModeToml) -> AutoCoordinatorMode {
    match raw {
        AutoCoordinatorModeToml::Off => AutoCoordinatorMode::Off,
        AutoCoordinatorModeToml::Auto => AutoCoordinatorMode::Auto,
        AutoCoordinatorModeToml::Always => AutoCoordinatorMode::Always,
    }
}

pub(crate) fn resolve_multi_agent_v2_config(
    config_toml: &ConfigToml,
    config_profile: &ConfigProfile,
) -> MultiAgentV2Config {
    let base = multi_agent_v2_toml_config(config_toml.features.as_ref());
    let profile = multi_agent_v2_toml_config(config_profile.features.as_ref());
    let default = MultiAgentV2Config::default();

    let max_concurrent_threads_per_session = profile
        .and_then(|config| config.max_concurrent_threads_per_session)
        .or_else(|| base.and_then(|config| config.max_concurrent_threads_per_session))
        .unwrap_or(default.max_concurrent_threads_per_session);
    let min_wait_timeout_ms = profile
        .and_then(|config| config.min_wait_timeout_ms)
        .or_else(|| base.and_then(|config| config.min_wait_timeout_ms))
        .unwrap_or(default.min_wait_timeout_ms);
    let max_wait_timeout_ms = profile
        .and_then(|config| config.max_wait_timeout_ms)
        .or_else(|| base.and_then(|config| config.max_wait_timeout_ms))
        .unwrap_or(default.max_wait_timeout_ms);
    let default_wait_timeout_ms = profile
        .and_then(|config| config.default_wait_timeout_ms)
        .or_else(|| base.and_then(|config| config.default_wait_timeout_ms))
        .unwrap_or(default.default_wait_timeout_ms);
    let usage_hint_enabled = profile
        .and_then(|config| config.usage_hint_enabled)
        .or_else(|| base.and_then(|config| config.usage_hint_enabled))
        .unwrap_or(default.usage_hint_enabled);
    let usage_hint_text = profile
        .and_then(|config| config.usage_hint_text.as_ref())
        .or_else(|| base.and_then(|config| config.usage_hint_text.as_ref()))
        .cloned()
        .or(default.usage_hint_text);
    let root_agent_usage_hint_text = profile
        .and_then(|config| config.root_agent_usage_hint_text.as_ref())
        .or_else(|| base.and_then(|config| config.root_agent_usage_hint_text.as_ref()))
        .cloned()
        .or(default.root_agent_usage_hint_text);
    let subagent_usage_hint_text = profile
        .and_then(|config| config.subagent_usage_hint_text.as_ref())
        .or_else(|| base.and_then(|config| config.subagent_usage_hint_text.as_ref()))
        .cloned()
        .or(default.subagent_usage_hint_text);
    let plan_token_economy_delegation_k = profile
        .and_then(|config| config.plan_token_economy_delegation_k)
        .or_else(|| base.and_then(|config| config.plan_token_economy_delegation_k))
        .unwrap_or(default.plan_token_economy_delegation_k);
    let usage_hint_cadence = profile
        .and_then(|config| config.usage_hint_cadence)
        .or_else(|| base.and_then(|config| config.usage_hint_cadence))
        .map(resolve_usage_hint_cadence)
        .unwrap_or(default.usage_hint_cadence);
    let usage_hint_reminder_interval = profile
        .and_then(|config| config.usage_hint_reminder_interval)
        .or_else(|| base.and_then(|config| config.usage_hint_reminder_interval))
        .unwrap_or(default.usage_hint_reminder_interval);
    let auto_coordinator = profile
        .and_then(|config| config.auto_coordinator)
        .or_else(|| base.and_then(|config| config.auto_coordinator))
        .map(resolve_auto_coordinator)
        .unwrap_or(default.auto_coordinator);
    let hide_spawn_agent_metadata = profile
        .and_then(|config| config.hide_spawn_agent_metadata)
        .or_else(|| base.and_then(|config| config.hide_spawn_agent_metadata))
        .unwrap_or(default.hide_spawn_agent_metadata);
    let non_code_mode_only = profile
        .and_then(|config| config.non_code_mode_only)
        .or_else(|| base.and_then(|config| config.non_code_mode_only))
        .unwrap_or(default.non_code_mode_only);

    MultiAgentV2Config {
        max_concurrent_threads_per_session,
        min_wait_timeout_ms,
        max_wait_timeout_ms,
        default_wait_timeout_ms,
        usage_hint_enabled,
        usage_hint_text,
        root_agent_usage_hint_text,
        subagent_usage_hint_text,
        plan_token_economy_delegation_k,
        // Cadence + interval are resolved from `features.multi_agent_v2` TOML
        // (profile over base over struct default). When the keys are absent the
        // default cadence is InitialContext (a per-request no-op) and the default
        // interval is 5, preserving today's behavior.
        usage_hint_cadence,
        usage_hint_reminder_interval,
        auto_coordinator,
        hide_spawn_agent_metadata,
        non_code_mode_only,
        tool_namespace: default.tool_namespace.clone(),
    }
}

pub(crate) fn resolve_rollout_budget_config(
    config_toml: &ConfigToml,
    features: &ManagedFeatures,
) -> std::io::Result<Option<RolloutBudgetConfig>> {
    if !features.enabled(Feature::RolloutBudget) {
        return Ok(None);
    }
    let missing_limit_error = || {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "features.rollout_budget.limit_tokens is required when rollout_budget is enabled",
        )
    };
    let Some(FeatureToml::Config(config)) = config_toml
        .features
        .as_ref()
        .and_then(|features| features.rollout_budget.as_ref())
    else {
        return Err(missing_limit_error());
    };
    let Some(limit_tokens) = config.limit_tokens else {
        return Err(missing_limit_error());
    };
    if limit_tokens <= 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "features.rollout_budget.limit_tokens must be positive",
        ));
    }
    let reminder_at_remaining_tokens = config.reminder_at_remaining_tokens.clone().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "features.rollout_budget.reminder_at_remaining_tokens is required when rollout_budget is enabled",
        )
    })?;
    if reminder_at_remaining_tokens
        .iter()
        .any(|&tokens| tokens <= 0 || tokens >= limit_tokens)
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "features.rollout_budget.reminder_at_remaining_tokens must contain only positive values below limit_tokens",
        ));
    }
    let sampling_token_weight = config.sampling_token_weight.unwrap_or(1.0);
    let prefill_token_weight = config.prefill_token_weight.unwrap_or(1.0);
    for (field, weight) in [
        ("sampling_token_weight", sampling_token_weight),
        ("prefill_token_weight", prefill_token_weight),
    ] {
        if !weight.is_finite() || weight < 0.0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("features.rollout_budget.{field} must be finite and non-negative"),
            ));
        }
    }
    Ok(Some(RolloutBudgetConfig {
        limit_tokens,
        reminder_at_remaining_tokens,
        sampling_token_weight,
        prefill_token_weight,
    }))
}

pub(crate) fn resolve_current_time_reminder_config(
    config_toml: &ConfigToml,
    features: &ManagedFeatures,
) -> std::io::Result<Option<CurrentTimeReminderConfig>> {
    if !features.enabled(Feature::CurrentTimeReminder) {
        return Ok(None);
    }

    let base = current_time_reminder_toml_config(config_toml.features.as_ref());
    let default = CurrentTimeReminderConfig::default();
    let reminder_interval_seconds = base
        .and_then(|config| config.reminder_interval_seconds)
        .unwrap_or(default.reminder_interval_seconds);
    if reminder_interval_seconds == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "features.current_time_reminder.reminder_interval_seconds must be positive",
        ));
    }

    Ok(Some(CurrentTimeReminderConfig {
        reminder_interval_seconds,
        clock_source: base
            .and_then(|config| config.clock_source)
            .unwrap_or(default.clock_source),
    }))
}

pub(crate) fn current_time_reminder_toml_config(
    features: Option<&FeaturesToml>,
) -> Option<&CurrentTimeReminderConfigToml> {
    match features?.current_time_reminder.as_ref()? {
        FeatureToml::Enabled(_) => None,
        FeatureToml::Config(config) => Some(config),
    }
}

pub(crate) fn resolve_terminal_resize_reflow_config(
    config_toml: &ConfigToml,
) -> TerminalResizeReflowConfig {
    let Some(tui) = config_toml.tui.as_ref() else {
        return TerminalResizeReflowConfig::default();
    };

    TerminalResizeReflowConfig {
        max_rows: match tui.terminal_resize_reflow_max_rows {
            Some(0) => TerminalResizeReflowMaxRows::Disabled,
            Some(rows) => TerminalResizeReflowMaxRows::Limit(rows),
            None => TerminalResizeReflowMaxRows::Auto,
        },
    }
}

pub(crate) fn multi_agent_v2_toml_config(
    features: Option<&FeaturesToml>,
) -> Option<&MultiAgentV2ConfigToml> {
    match features?.multi_agent_v2.as_ref()? {
        FeatureToml::Enabled(_) => None,
        FeatureToml::Config(config) => Some(config),
    }
}

pub(crate) fn resolve_desktop_automation_config(
    base: Option<&DesktopAutomationToml>,
    profile: Option<&DesktopAutomationToml>,
) -> DesktopAutomationConfig {
    DesktopAutomationConfig {
        enabled: profile
            .and_then(|config| config.enabled)
            .or_else(|| base.and_then(|config| config.enabled))
            .unwrap_or(true),
        proactive: profile
            .and_then(|config| config.proactive)
            .or_else(|| base.and_then(|config| config.proactive))
            .unwrap_or(true),
        allow_input: profile
            .and_then(|config| config.allow_input)
            .or_else(|| base.and_then(|config| config.allow_input))
            .unwrap_or(true),
        prefer_app_harness: profile
            .and_then(|config| config.prefer_app_harness)
            .or_else(|| base.and_then(|config| config.prefer_app_harness))
            .unwrap_or(true),
    }
}

pub(crate) fn resolve_first_moves_config(
    base: Option<&FirstMovesToml>,
    profile: Option<&FirstMovesToml>,
) -> FirstMovesConfig {
    let defaults = FirstMovesConfig::default();
    let enabled = profile
        .and_then(|config| config.enabled)
        .or_else(|| base.and_then(|config| config.enabled))
        .unwrap_or(true);
    let mut mode = profile
        .and_then(|config| config.mode)
        .or_else(|| base.and_then(|config| config.mode))
        .map(first_moves_mode_from_toml)
        .unwrap_or(defaults.mode);
    if !enabled {
        mode = FirstMovesMode::Off;
    }

    FirstMovesConfig {
        mode,
        inject_context: profile
            .and_then(|config| config.inject_context)
            .or_else(|| base.and_then(|config| config.inject_context))
            .unwrap_or(defaults.inject_context),
        prewarm: profile
            .and_then(|config| config.prewarm)
            .or_else(|| base.and_then(|config| config.prewarm))
            .map(first_moves_prewarm_from_toml)
            .unwrap_or(defaults.prewarm),
        max_candidates: profile
            .and_then(|config| config.max_candidates)
            .or_else(|| base.and_then(|config| config.max_candidates))
            .unwrap_or(defaults.max_candidates),
        max_context_moves: profile
            .and_then(|config| config.max_context_moves)
            .or_else(|| base.and_then(|config| config.max_context_moves))
            .unwrap_or(defaults.max_context_moves),
        max_prewarm_files: profile
            .and_then(|config| config.max_prewarm_files)
            .or_else(|| base.and_then(|config| config.max_prewarm_files))
            .unwrap_or(defaults.max_prewarm_files),
        min_context_score: profile
            .and_then(|config| config.min_context_score)
            .or_else(|| base.and_then(|config| config.min_context_score))
            .unwrap_or(defaults.min_context_score),
        min_prewarm_score: profile
            .and_then(|config| config.min_prewarm_score)
            .or_else(|| base.and_then(|config| config.min_prewarm_score))
            .unwrap_or(defaults.min_prewarm_score),
        max_scan_files: profile
            .and_then(|config| config.max_scan_files)
            .or_else(|| base.and_then(|config| config.max_scan_files))
            .unwrap_or(defaults.max_scan_files),
        max_scan_depth: profile
            .and_then(|config| config.max_scan_depth)
            .or_else(|| base.and_then(|config| config.max_scan_depth))
            .unwrap_or(defaults.max_scan_depth),
        max_read_bytes: profile
            .and_then(|config| config.max_read_bytes)
            .or_else(|| base.and_then(|config| config.max_read_bytes))
            .unwrap_or(defaults.max_read_bytes),
    }
}

pub(crate) fn first_moves_mode_from_toml(mode: FirstMovesModeToml) -> FirstMovesMode {
    match mode {
        FirstMovesModeToml::Auto => FirstMovesMode::Auto,
        FirstMovesModeToml::SuggestOnly => FirstMovesMode::SuggestOnly,
        FirstMovesModeToml::Prewarm => FirstMovesMode::Prewarm,
        FirstMovesModeToml::Off => FirstMovesMode::Off,
    }
}

pub(crate) fn first_moves_prewarm_from_toml(prewarm: FirstMovesPrewarmToml) -> FirstMovesPrewarm {
    match prewarm {
        FirstMovesPrewarmToml::Off => FirstMovesPrewarm::Off,
        FirstMovesPrewarmToml::HighConfidenceOnly => FirstMovesPrewarm::HighConfidenceOnly,
    }
}

pub(crate) fn resolve_repo_context_scout_config(
    base: Option<&RepoContextScoutToml>,
    profile: Option<&RepoContextScoutToml>,
) -> RepoContextScoutConfig {
    let defaults = RepoContextScoutConfig::default();
    RepoContextScoutConfig {
        mode: profile
            .and_then(|config| config.mode)
            .or_else(|| base.and_then(|config| config.mode))
            .map(repo_context_scout_mode_from_toml)
            .unwrap_or(defaults.mode),
        max_files: profile
            .and_then(|config| config.max_files)
            .or_else(|| base.and_then(|config| config.max_files))
            .unwrap_or(defaults.max_files),
        max_file_bytes: profile
            .and_then(|config| config.max_file_bytes)
            .or_else(|| base.and_then(|config| config.max_file_bytes))
            .unwrap_or(defaults.max_file_bytes),
        max_anchors_per_file: profile
            .and_then(|config| config.max_anchors_per_file)
            .or_else(|| base.and_then(|config| config.max_anchors_per_file))
            .unwrap_or(defaults.max_anchors_per_file),
        max_output_tokens: profile
            .and_then(|config| config.max_output_tokens)
            .or_else(|| base.and_then(|config| config.max_output_tokens))
            .unwrap_or(defaults.max_output_tokens),
        max_candidates: profile
            .and_then(|config| config.max_candidates)
            .or_else(|| base.and_then(|config| config.max_candidates))
            .unwrap_or(defaults.max_candidates),
    }
}

pub(crate) fn repo_context_scout_mode_from_toml(
    mode: RepoContextScoutModeToml,
) -> RepoContextScoutMode {
    match mode {
        RepoContextScoutModeToml::Off => RepoContextScoutMode::Off,
        RepoContextScoutModeToml::Shadow => RepoContextScoutMode::Shadow,
        RepoContextScoutModeToml::Tool => RepoContextScoutMode::Tool,
    }
}

// apps_mcp_path_override config was removed from FeaturesToml in upstream; the path is no longer
// configurable via TOML. Always return None so callers fall back to the hard-coded default.
pub(crate) fn apps_mcp_path_override_toml_config(
    _features: Option<&FeaturesToml>,
) -> Option<String> {
    None
}

pub(crate) fn network_proxy_toml_config(
    features: Option<&FeaturesToml>,
) -> Option<&NetworkProxyConfigToml> {
    match features?.network_proxy.as_ref()? {
        FeatureToml::Enabled(_) => None,
        FeatureToml::Config(config) => Some(config),
    }
}

pub(crate) fn resolve_web_search_mode_for_turn(
    web_search_mode: &Constrained<WebSearchMode>,
    permission_profile: &PermissionProfile,
) -> WebSearchMode {
    let preferred = web_search_mode.value();

    if matches!(permission_profile, PermissionProfile::Disabled)
        && preferred != WebSearchMode::Disabled
    {
        for mode in [
            WebSearchMode::Live,
            WebSearchMode::Cached,
            WebSearchMode::Disabled,
        ] {
            if web_search_mode.can_set(&mode).is_ok() {
                return mode;
            }
        }
    } else {
        if web_search_mode.can_set(&preferred).is_ok() {
            return preferred;
        }
        for mode in [
            WebSearchMode::Cached,
            WebSearchMode::Live,
            WebSearchMode::Disabled,
        ] {
            if web_search_mode.can_set(&mode).is_ok() {
                return mode;
            }
        }
    }

    WebSearchMode::Disabled
}
