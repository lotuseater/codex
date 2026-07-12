use super::*;
use crate::StateDbHandle;
use crate::agents_md::LoadedAgentsMd;
use crate::agents_md_manager::AgentsMdManager;
use crate::config::ConstraintError;
use crate::environment_selection::ThreadEnvironments;
use crate::environment_selection::TurnEnvironmentSnapshot;
use crate::goals::GoalRuntimeState;
use crate::session::InputQueue;
use crate::session::blackboard::new_blackboard_session;
use crate::shell_snapshot::ShellSnapshot;
use crate::skills::SkillError;
use crate::state::ActiveTurn;
use crate::state::AutoCompactWindowIds;
use codex_extension_api::ExtensionDataInit;
use codex_login::auth::AgentIdentityAuthPolicy;
use codex_protocol::SessionId;
use codex_protocol::capabilities::SelectedCapabilityRoot;
use codex_protocol::config_types::ContextBudgetMode;
use codex_protocol::config_types::SERVICE_TIER_DEFAULT_REQUEST_VALUE;
use codex_protocol::config_types::ServiceTier;
use codex_protocol::permissions::FileSystemPath;
use codex_protocol::permissions::FileSystemSpecialPath;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_protocol::protocol::ThreadSource;
use codex_protocol::protocol::TurnEnvironmentSelections;
use std::sync::OnceLock;
use tokio::sync::Semaphore;

/// Context for an initialized model agent
///
/// A session has at most 1 running task at a time, and can be interrupted by user input.
pub(crate) struct Session {
    pub(crate) thread_id: ThreadId,
    pub(crate) installation_id: String,
    pub(super) tx_event: Sender<Event>,
    pub(super) agent_status: watch::Sender<AgentStatus>,
    // fork-local: retained after upstream removed the out-of-band elicitation
    // pause channel; still read by session/mod.rs accessors and constructed by
    // support_session.rs / zsh_fork_tests.rs.
    pub(super) out_of_band_elicitation_paused: watch::Sender<bool>,
    pub(super) state: Mutex<SessionState>,
    /// Serializes rebuild/apply cycles for the running proxy; each cycle
    /// rebuilds from the current SessionState while holding this lock.
    pub(super) managed_network_proxy_refresh_lock: Semaphore,
    /// The set of enabled features should be invariant for the lifetime of the
    /// session.
    pub(super) features: ManagedFeatures,
    pub(super) multi_agent_version: OnceLock<MultiAgentVersion>,
    pub(super) pending_mcp_server_refresh_config: Mutex<Option<McpServerRefreshConfig>>,
    pub(crate) conversation: Arc<RealtimeConversationManager>,
    pub(crate) active_turn: Mutex<Option<ActiveTurn>>,
    pub(super) mailbox: Mailbox,
    pub(super) mailbox_rx: Mutex<MailboxReceiver>,
    pub(super) idle_pending_input: Mutex<Vec<ResponseInputItem>>, // TODO (jif) merge with mailbox!
    pub(crate) input_queue: InputQueue,
    pub(crate) goal_runtime: GoalRuntimeState,
    pub(crate) guardian_review_session: GuardianReviewSessionManager,
    pub(crate) services: SessionServices,
    pub(super) next_internal_sub_id: AtomicU64,
}

struct LiveThreadInitGuard {
    live_thread: Option<Arc<dyn LiveThreadHandle>>,
}

impl LiveThreadInitGuard {
    fn new(live_thread: Option<Arc<dyn LiveThreadHandle>>) -> Self {
        Self { live_thread }
    }

    fn as_ref(&self) -> Option<&Arc<dyn LiveThreadHandle>> {
        self.live_thread.as_ref()
    }

    fn commit(&mut self) {
        self.live_thread.take();
    }

    async fn discard(&mut self) {
        let Some(live_thread) = self.live_thread.take() else {
            return;
        };
        if let Err(err) = live_thread.discard().await {
            warn!("failed to discard live thread during session init cleanup: {err}");
        }
    }
}

impl Drop for LiveThreadInitGuard {
    fn drop(&mut self) {
        if self.live_thread.is_some() {
            warn!("live thread init guard dropped before commit or discard");
        }
    }
}

#[derive(Clone)]
pub(crate) struct SessionConfiguration {
    /// Provider identifier ("openai", "openrouter", ...).
    pub(super) provider: ModelProviderInfo,

    pub(super) collaboration_mode: CollaborationMode,
    /// fork-local: per-thread multi-agent-mode override. `None` re-derives the
    /// mode every turn via `effective_multi_agent_mode` (upstream behavior);
    /// `Some` is honored verbatim when a thread explicitly selects a mode via
    /// `SessionSettingsUpdate::multi_agent_mode`.
    pub(super) multi_agent_mode_override: Option<codex_protocol::config_types::MultiAgentMode>,
    pub(super) model_reasoning_summary: Option<ReasoningSummaryConfig>,
    pub(super) service_tier: Option<String>,
    pub(super) context_budget_mode: ContextBudgetMode,

    /// Developer instructions that supplement the base instructions.
    pub(super) developer_instructions: Option<String>,

    /// Model instructions assembled from provider instructions and discovered
    /// AGENTS.md files.
    pub(super) loaded_agents_md: Option<LoadedAgentsMd>,

    /// fork-local: model instructions assembled from the caller-supplied user
    /// instructions and the files that supplied them. Distinct from the
    /// upstream `loaded_agents_md` (discovered AGENTS.md); read by
    /// `Codex::instruction_sources` and `Session::user_instructions`.
    pub(super) user_instructions: Option<LoadedAgentsMd>,

    /// Personality preference for the model.
    pub(super) personality: Option<Personality>,

    /// Base instructions for the session.
    pub(super) base_instructions: String,

    /// Compact prompt override.
    pub(super) compact_prompt: Option<String>,

    /// When to escalate for approval for execution
    pub(super) approval_policy: Constrained<AskForApproval>,
    pub(super) approvals_reviewer: ApprovalsReviewer,
    /// Canonical permission profile for the session.
    pub(super) permission_profile: Constrained<PermissionProfile>,
    /// Named or implicit built-in permissions profile selected from config, if
    /// any.
    pub(super) active_permission_profile: Option<ActivePermissionProfile>,
    pub(super) windows_sandbox_level: WindowsSandboxLevel,

    /// Sticky thread-level environment selections plus the legacy cwd used
    /// when a turn does not select an environment.
    pub(super) environments: TurnEnvironmentSelections,
    /// Thread-scoped runtime workspace roots for materializing symbolic
    /// workspace permissions at session runtime.
    pub(super) workspace_roots: Vec<AbsolutePathBuf>,
    /// Workspace roots contributed by the selected permission profile.
    pub(super) profile_workspace_roots: Vec<AbsolutePathBuf>,
    /// Directory containing all Codex state for this session.
    pub(super) codex_home: AbsolutePathBuf,
    /// Optional user-facing name for the thread, updated during the session.
    pub(super) thread_name: Option<String>,

    // TODO(pakrym): Remove config from here
    pub(super) original_config_do_not_use: Arc<Config>,
    /// Optional service name tag for session metrics.
    pub(super) metrics_service_name: Option<String>,
    pub(super) app_server_client_name: Option<String>,
    pub(super) app_server_client_version: Option<String>,
    /// Source of the session (cli, vscode, exec, mcp, ...)
    pub(super) session_source: SessionSource,
    /// Persisted thread history contract selected when this thread was created.
    pub(super) history_mode: ThreadHistoryMode,
    /// Immediate history source copied into this thread, when this thread was forked.
    pub(super) forked_from_thread_id: Option<ThreadId>,
    /// Immediate control/spawn parent for this thread, when it has one.
    pub(super) parent_thread_id: Option<ThreadId>,
    /// Optional analytics source classification for this thread.
    pub(super) thread_source: Option<ThreadSource>,
    /// Effective originator used for this thread's Responses requests and analytics events.
    pub(super) originator: String,
    pub(super) dynamic_tools: Vec<DynamicToolSpec>,
    pub(super) user_shell_override: Option<shell::Shell>,
}

impl SessionConfiguration {
    pub(super) fn cwd(&self) -> &AbsolutePathBuf {
        &self.environments.legacy_fallback_cwd
    }

    pub(super) fn environment_selections(&self) -> &[TurnEnvironmentSelection] {
        &self.environments.environments
    }

    pub(crate) fn codex_home(&self) -> &AbsolutePathBuf {
        &self.codex_home
    }

    pub(super) fn permission_profile(&self) -> PermissionProfile {
        self.permission_profile.get().clone()
    }

    pub(super) fn active_permission_profile(&self) -> Option<ActivePermissionProfile> {
        self.active_permission_profile.clone()
    }

    pub(super) fn sandbox_policy(&self) -> SandboxPolicy {
        let permission_profile = self.permission_profile();
        codex_sandboxing::compatibility_sandbox_policy_for_permission_profile(
            &permission_profile,
            self.cwd(),
        )
    }

    pub(super) fn file_system_sandbox_policy(&self) -> FileSystemSandboxPolicy {
        self.permission_profile.get().file_system_sandbox_policy()
    }

    pub(super) fn network_sandbox_policy(&self) -> NetworkSandboxPolicy {
        self.permission_profile.get().network_sandbox_policy()
    }

    pub(super) fn thread_config_snapshot(&self) -> ThreadConfigSnapshot {
        ThreadConfigSnapshot {
            model: self.collaboration_mode.model().to_string(),
            model_provider_id: self.original_config_do_not_use.model_provider_id.clone(),
            service_tier: self.service_tier.clone(),
            approval_policy: self.approval_policy.value(),
            approvals_reviewer: self.approvals_reviewer,
            permission_profile: self.permission_profile(),
            active_permission_profile: self.active_permission_profile(),
            environments: TurnEnvironmentSelections::new(self.cwd().clone(), Vec::new()),
            workspace_roots: self.workspace_roots.clone(),
            profile_workspace_roots: self.profile_workspace_roots.clone(),
            ephemeral: self.original_config_do_not_use.ephemeral,
            reasoning_effort: self.collaboration_mode.reasoning_effort(),
            reasoning_summary: self.model_reasoning_summary,
            personality: self.personality,
            collaboration_mode: self.collaboration_mode.clone(),
            // fork-local: reflect an explicit per-thread override when one was
            // selected via `SessionSettingsUpdate::multi_agent_mode`; otherwise
            // default (the live mode is re-derived by effective_multi_agent_mode
            // every turn, mirroring handlers.rs default()).
            multi_agent_mode: self.multi_agent_mode_override.clone().unwrap_or_default(),
            session_source: self.session_source.clone(),
            history_mode: self.history_mode,
            forked_from_thread_id: self.forked_from_thread_id,
            parent_thread_id: self.parent_thread_id,
            thread_source: self.thread_source.clone(),
            originator: self.originator.clone(),
        }
    }

    pub(crate) fn apply(&self, updates: &SessionSettingsUpdate) -> ConstraintResult<Self> {
        let mut next_configuration = self.clone();
        let current_sandbox_policy = self.sandbox_policy();
        let current_file_system_sandbox_policy = self.file_system_sandbox_policy();
        let current_network_sandbox_policy = self.network_sandbox_policy();
        let legacy_file_system_projection =
            FileSystemSandboxPolicy::from_legacy_sandbox_policy_preserving_deny_entries(
                &current_sandbox_policy,
                self.cwd(),
                &current_file_system_sandbox_policy,
            );
        let file_system_policy_matches_legacy = current_file_system_sandbox_policy
            .is_semantically_equivalent_to(&legacy_file_system_projection, self.cwd());
        let file_system_policy_has_rebindable_project_root_write =
            current_file_system_sandbox_policy
                .entries
                .iter()
                .any(|entry| {
                    entry.access.can_write()
                        && matches!(
                            &entry.path,
                            FileSystemPath::Special {
                                value: FileSystemSpecialPath::ProjectRoots { subpath: None },
                            }
                        )
                });
        if let Some(collaboration_mode) = updates.collaboration_mode.clone() {
            next_configuration.collaboration_mode = collaboration_mode;
        }
        if let Some(summary) = updates.reasoning_summary {
            next_configuration.model_reasoning_summary = Some(summary);
        }
        if let Some(service_tier) = updates.service_tier.clone() {
            // TODO(aibrahim): Remove once v2 clients no longer send the legacy
            // "fast" service tier value.
            next_configuration.service_tier = match service_tier {
                Some(service_tier) => Some(
                    ServiceTier::from_request_value(&service_tier)
                        .map_or(service_tier, |service_tier| {
                            service_tier.request_value().to_string()
                        }),
                ),
                None => Some(SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string()),
            };
        }
        if let Some(context_budget_mode) = updates.context_budget_mode {
            next_configuration.context_budget_mode = context_budget_mode;
        }
        if let Some(personality) = updates.personality {
            next_configuration.personality = Some(personality);
        }
        if let Some(multi_agent_mode) = updates.multi_agent_mode.clone() {
            // fork-local: persist an explicit per-thread multi-agent-mode
            // selection so effective_multi_agent_mode honors it instead of
            // re-deriving the effort/auto-coordinator default.
            next_configuration.multi_agent_mode_override = Some(multi_agent_mode);
        }
        if let Some(approval_policy) = updates.approval_policy {
            next_configuration.approval_policy.set(approval_policy)?;
        }
        if let Some(approvals_reviewer) = updates.approvals_reviewer {
            next_configuration.approvals_reviewer = approvals_reviewer;
        }
        if let Some(windows_sandbox_level) = updates.windows_sandbox_level {
            next_configuration.windows_sandbox_level = windows_sandbox_level;
        }

        let current_cwd = self.cwd().clone();
        let next_environments = updates
            .environments
            .clone()
            .unwrap_or_else(|| self.environments.clone());
        let cwd_changed = next_environments.legacy_fallback_cwd.as_path() != current_cwd.as_path();
        next_configuration.environments = next_environments;
        if let Some(workspace_roots) = updates.workspace_roots.clone() {
            next_configuration.workspace_roots = workspace_roots;
        } else if cwd_changed && self.workspace_roots.contains(&current_cwd) {
            let mut retargeted_workspace_roots =
                Vec::with_capacity(next_configuration.workspace_roots.len());
            for root in &self.workspace_roots {
                let root = if root == &current_cwd {
                    next_configuration.cwd().clone()
                } else {
                    root.clone()
                };
                if !retargeted_workspace_roots.contains(&root) {
                    retargeted_workspace_roots.push(root);
                }
            }
            next_configuration.workspace_roots = retargeted_workspace_roots;
        }
        // fork-local: fork permission model stores profile_workspace_roots as a
        // direct field; apply caller-supplied roots up front before the
        // permission-profile block reads `updates.profile_workspace_roots`.
        if let Some(profile_workspace_roots) = updates.profile_workspace_roots.clone() {
            next_configuration.profile_workspace_roots = profile_workspace_roots;
        }

        if let Some(permission_profile) = updates.permission_profile.clone() {
            let active_permission_profile =
                updates.active_permission_profile.clone().or_else(|| {
                    if permission_profile == self.permission_profile() {
                        self.active_permission_profile.clone()
                    } else {
                        None
                    }
                });
            next_configuration.set_permission_profile_projection(
                permission_profile,
                Some(&current_file_system_sandbox_policy),
            )?;
            next_configuration.active_permission_profile = active_permission_profile;
            if next_configuration.active_permission_profile.is_none()
                && updates.profile_workspace_roots.is_none()
            {
                next_configuration.profile_workspace_roots = Vec::new();
            }
            if let Some(active_permission_profile) = next_configuration.active_permission_profile()
            {
                let mut config = (*next_configuration.original_config_do_not_use).clone();
                let permission_profile = next_configuration.permission_profile();
                config.permissions.network = config
                    .network_proxy_spec_for_active_permission_profile(
                        &active_permission_profile,
                        &permission_profile,
                    )
                    .map_err(|err| ConstraintError::InvalidValue {
                        field_name: "default_permissions",
                        candidate: active_permission_profile.id.clone(),
                        allowed: format!(
                            "configured permission profile with valid network policy ({err})"
                        ),
                        requirement_source: codex_config::RequirementSource::Unknown,
                    })?;
                config
                    .permissions
                    .set_permission_profile_with_active_profile(
                        permission_profile,
                        Some(active_permission_profile),
                    )?;
                next_configuration.original_config_do_not_use = Arc::new(config);
            }
        } else if let Some(sandbox_policy) = updates.sandbox_policy.clone() {
            let file_system_sandbox_policy =
                FileSystemSandboxPolicy::from_legacy_sandbox_policy_preserving_deny_entries(
                    &sandbox_policy,
                    next_configuration.cwd(),
                    &current_file_system_sandbox_policy,
                );
            let network_sandbox_policy = NetworkSandboxPolicy::from(&sandbox_policy);
            next_configuration.permission_profile.set(
                PermissionProfile::from_runtime_permissions_with_enforcement(
                    SandboxEnforcement::from_legacy_sandbox_policy(&sandbox_policy),
                    &file_system_sandbox_policy,
                    network_sandbox_policy,
                ),
            )?;
            next_configuration.active_permission_profile = None;
            next_configuration.profile_workspace_roots = Vec::new();
        } else if cwd_changed
            && file_system_policy_matches_legacy
            && file_system_policy_has_rebindable_project_root_write
        {
            // Preserve richer split policies across cwd-only updates; only
            // rederive when the session is already using a structurally
            // cwd-bound legacy bridge.
            let file_system_sandbox_policy =
                FileSystemSandboxPolicy::from_legacy_sandbox_policy_preserving_deny_entries(
                    &current_sandbox_policy,
                    next_configuration.cwd(),
                    &current_file_system_sandbox_policy,
                );
            next_configuration.permission_profile.set(
                PermissionProfile::from_runtime_permissions_with_enforcement(
                    SandboxEnforcement::from_legacy_sandbox_policy(&current_sandbox_policy),
                    &file_system_sandbox_policy,
                    current_network_sandbox_policy,
                ),
            )?;
        }
        if let Some(app_server_client_name) = updates.app_server_client_name.clone() {
            next_configuration.app_server_client_name = Some(app_server_client_name);
        }
        if let Some(app_server_client_version) = updates.app_server_client_version.clone() {
            next_configuration.app_server_client_version = Some(app_server_client_version);
        }
        Ok(next_configuration)
    }

    fn set_permission_profile_projection(
        &mut self,
        permission_profile: PermissionProfile,
        preserve_deny_reads_from: Option<&FileSystemSandboxPolicy>,
    ) -> ConstraintResult<()> {
        let enforcement = permission_profile.enforcement();
        let (mut file_system_sandbox_policy, network_sandbox_policy) =
            permission_profile.to_runtime_permissions();
        if let Some(existing_file_system_policy) = preserve_deny_reads_from {
            file_system_sandbox_policy
                .preserve_deny_read_restrictions_from(existing_file_system_policy);
        }
        let effective_permission_profile =
            PermissionProfile::from_runtime_permissions_with_enforcement(
                enforcement,
                &file_system_sandbox_policy,
                network_sandbox_policy,
            );
        self.permission_profile.set(effective_permission_profile)?;
        Ok(())
    }
}

#[derive(Default, Clone)]
pub(crate) struct SessionSettingsUpdate {
    pub(crate) environments: Option<TurnEnvironmentSelections>,
    pub(crate) workspace_roots: Option<Vec<AbsolutePathBuf>>,
    pub(crate) profile_workspace_roots: Option<Vec<AbsolutePathBuf>>,
    pub(crate) approval_policy: Option<AskForApproval>,
    pub(crate) approvals_reviewer: Option<ApprovalsReviewer>,
    pub(crate) sandbox_policy: Option<SandboxPolicy>,
    pub(crate) permission_profile: Option<PermissionProfile>,
    pub(crate) active_permission_profile: Option<ActivePermissionProfile>,
    pub(crate) windows_sandbox_level: Option<WindowsSandboxLevel>,
    pub(crate) collaboration_mode: Option<CollaborationMode>,
    pub(crate) multi_agent_mode: Option<codex_protocol::config_types::MultiAgentMode>,
    pub(crate) reasoning_summary: Option<ReasoningSummaryConfig>,
    pub(crate) service_tier: Option<Option<String>>,
    pub(crate) context_budget_mode: Option<ContextBudgetMode>,
    pub(crate) final_output_json_schema: Option<Option<Value>>,
    pub(crate) personality: Option<Personality>,
    pub(crate) app_server_client_name: Option<String>,
    pub(crate) app_server_client_version: Option<String>,
}

pub(crate) struct AppServerClientMetadata {
    pub(crate) client_name: Option<String>,
    pub(crate) client_version: Option<String>,
}

async fn warm_plugins_and_skills_for_session_init(
    config: Arc<Config>,
    plugins_manager: Arc<PluginsManager>,
    skills_service: Arc<SkillsService>,
    turn_environments: &TurnEnvironmentSnapshot,
) -> Vec<SkillError> {
    let fs = turn_environments.primary_filesystem();
    let plugins_input = config.plugins_config_input();
    let plugin_outcome = plugins_manager.plugins_for_config(&plugins_input).await;
    let effective_skill_roots = plugin_outcome.effective_plugin_skill_roots();
    let plugin_skill_snapshots = plugins_manager.plugin_skill_snapshots_for_config(&plugins_input);
    let skills_input = skills_load_input_from_config(config.as_ref(), effective_skill_roots)
        .with_plugin_skill_snapshots(plugin_skill_snapshots);
    skills_service
        .snapshot_for_config(&skills_input, fs)
        .await
        .outcome()
        .errors
        .clone()
}

impl Session {
    /// Returns the concrete identity for this thread.
    pub(crate) fn thread_id(&self) -> ThreadId {
        self.thread_id
    }

    /// Returns the identity shared by the root thread and all descendant threads.
    pub(crate) fn session_id(&self) -> SessionId {
        self.services.agent_control.session_id()
    }

    // Restored from upstream `session/mod.rs`: dropped when the merge took the
    // fork's slim `mod.rs`. Callers (codex_delegate.rs, guardian/review_session.rs,
    // thread_manager.rs) read the discovered AGENTS.md user-instructions to seed
    // spawned/child sessions.
    pub(crate) async fn user_instructions(&self) -> Option<codex_extension_api::UserInstructions> {
        let state = self.state.lock().await;
        state
            .session_configuration
            .loaded_agents_md
            .as_ref()
            .and_then(LoadedAgentsMd::user_instructions)
            .cloned()
    }

    // Restored from upstream `session/mod.rs`: the auto-compact context-window
    // helpers were lost when the merge took the fork's slim `mod.rs`. Fork/upstream
    // callers in `session/turn.rs`, `tools/handlers/new_context_window.rs`, compact*
    // and prewarm paths still call these on `Session`.
    pub(crate) async fn current_window_id(&self) -> String {
        let state = self.state.lock().await;
        let thread_id = self.thread_id;
        let window_number = state.auto_compact_window_number();
        format!("{thread_id}:{window_number}")
    }

    pub(crate) async fn advance_auto_compact_window(&self) -> (u64, AutoCompactWindowIds) {
        let mut state = self.state.lock().await;
        state.advance_auto_compact_window()
    }

    pub(crate) async fn request_new_context_window(&self) {
        let mut state = self.state.lock().await;
        state.request_new_context_window();
    }

    /// Records an inter-agent message into history and the rollout.
    ///
    /// Restored from upstream `64bdeed9f7` (`session/mod.rs:2764`), dropped when the merge took the
    /// fork's split `session/*` modules. Adapted to the current tree's history helpers: the upstream
    /// `prepare_conversation_items_for_history` does not exist here, so this mirrors the sibling
    /// `record_conversation_items` (`session_history.rs`) — append to in-memory history, persist the
    /// `InterAgentCommunication` rollout item, then forward the raw response item.
    pub(crate) async fn record_inter_agent_communication(
        &self,
        turn_context: &TurnContext,
        communication: InterAgentCommunication,
    ) {
        let response_item = communication.to_model_input_item();
        let items = std::slice::from_ref(&response_item);
        self.record_into_history(items, turn_context).await;
        self.persist_rollout_items(&[RolloutItem::InterAgentCommunication(communication)])
            .await;
        self.send_raw_response_items(turn_context, items).await;
    }

    pub(crate) async fn maybe_start_new_context_window(
        &self,
        turn_context: &TurnContext,
    ) -> Option<u64> {
        let window = {
            let mut state = self.state.lock().await;
            if state.take_new_context_window_request() {
                Some(state.start_new_context_window())
            } else {
                None
            }
        };
        let (window_number, window_ids) = window?;
        let context_items = self.build_initial_context(turn_context).await;
        let turn_context_item = turn_context.to_turn_context_item();
        let replacement_history = context_items;
        {
            let mut state = self.state.lock().await;
            state.replace_history(replacement_history.clone(), Some(turn_context_item.clone()));
        };
        self.persist_rollout_items(&[
            RolloutItem::Compacted(CompactedItem {
                message: String::new(),
                replacement_history: Some(replacement_history),
                window_number: Some(window_number),
                first_window_id: Some(window_ids.first_window_id.to_string()),
                previous_window_id: window_ids.previous_window_id.map(|id| id.to_string()),
                window_id: Some(window_ids.window_id.to_string()),
            }),
            RolloutItem::TurnContext(turn_context_item),
        ])
        .await;
        {
            let mut state = self.state.lock().await;
            state.queue_pending_session_start_source(codex_hooks::SessionStartSource::Compact);
        }
        self.recompute_token_usage(turn_context).await;
        Some(window_number)
    }

    pub(crate) async fn originator(&self) -> String {
        let state = self.state.lock().await;
        state.session_configuration.originator.clone()
    }

    #[instrument(name = "session_init", level = "info", skip_all)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn new(
        mut session_configuration: SessionConfiguration,
        config: Arc<Config>,
        user_instructions: Option<codex_extension_api::UserInstructions>,
        installation_id: String,
        auth_manager: Arc<AuthManager>,
        models_manager: SharedModelsManager,
        exec_policy: Arc<ExecPolicyManager>,
        tx_event: Sender<Event>,
        agent_status: watch::Sender<AgentStatus>,
        initial_history: InitialHistory,
        session_source: SessionSource,
        skills_service: Arc<SkillsService>,
        plugins_manager: Arc<PluginsManager>,
        mcp_manager: Arc<McpManager>,
        code_mode_session_provider: Arc<dyn codex_code_mode::CodeModeSessionProvider>,
        extensions: Arc<codex_extension_api::ExtensionRegistry<crate::config::Config>>,
        mut thread_extension_init: ExtensionDataInit,
        supports_openai_form_elicitation: bool,
        agent_control: AgentControl,
        environment_manager: Arc<EnvironmentManager>,
        inherited_environments: Option<TurnEnvironmentSnapshot>,
        analytics_events_client: Option<AnalyticsEventsClient>,
        thread_store: Arc<dyn ThreadStore>,
        live_thread_factory: Arc<dyn LiveThreadFactory>,
        state_db: Option<StateDbHandle>,
        parent_rollout_thread_trace: ThreadTraceContext,
        attestation_provider: Option<Arc<dyn AttestationProvider>>,
        external_time_provider: Option<Arc<dyn TimeProvider>>,
        multi_agent_version: Option<MultiAgentVersion>,
    ) -> anyhow::Result<Arc<Self>> {
        debug!(
            "Configuring session: model={}; provider={:?}",
            session_configuration.collaboration_mode.model(),
            session_configuration.provider
        );
        let forked_from_id = session_configuration
            .forked_from_thread_id
            .or_else(|| initial_history.forked_from_id());
        session_configuration.forked_from_thread_id = forked_from_id;
        let parent_thread_id = session_configuration
            .parent_thread_id
            .or_else(|| initial_history.get_resumed_parent_thread_id());
        session_configuration.parent_thread_id = parent_thread_id;
        let multi_agent_version = multi_agent_version.map(OnceLock::from).unwrap_or_default();
        let initial_multi_agent_version = multi_agent_version.get().copied();

        let thread_id = match &initial_history {
            InitialHistory::New | InitialHistory::Cleared | InitialHistory::Forked(_) => {
                ThreadId::default()
            }
            InitialHistory::Resumed(resumed_history) => resumed_history.conversation_id,
        };
        let resumed_session_id = match &initial_history {
            InitialHistory::Resumed(resumed) => {
                resumed.history.iter().find_map(|item| match item {
                    RolloutItem::SessionMeta(meta_line) => Some(meta_line.meta.session_id),
                    _ => None,
                })
            }
            InitialHistory::New | InitialHistory::Cleared | InitialHistory::Forked(_) => None,
        };
        // Legacy subagent rollouts synthesize session_id from their own thread id.
        let resumed_session_id = resumed_session_id.filter(|session_id| {
            !session_configuration.session_source.is_non_root_agent()
                || *session_id != SessionId::from(thread_id)
        });
        let session_id = resumed_session_id.unwrap_or_else(|| {
            if session_configuration.session_source.is_non_root_agent() {
                agent_control.session_id()
            } else {
                SessionId::from(thread_id)
            }
        });
        let initial_auto_compact_window_ids = AutoCompactWindowIds::new_initial();
        let agent_control = agent_control.with_session_id(
            session_id,
            config
                .effective_agent_max_threads(MultiAgentVersion::V2)
                .ok()
                .flatten()
                .unwrap_or(usize::MAX),
        );
        let time_provider = crate::current_time::resolve_time_provider(
            config.current_time_reminder.as_ref(),
            external_time_provider,
        )?;
        let selected_capability_roots =
            match thread_extension_init.get::<Vec<SelectedCapabilityRoot>>() {
                Some(roots) => roots.as_ref().clone(),
                None => {
                    let roots = initial_history.get_selected_capability_roots();
                    if !roots.is_empty() {
                        thread_extension_init.insert(roots.clone());
                    }
                    roots
                }
            };
        let mcp_thread_init = thread_extension_init.clone();
        let thread_extension_data = codex_extension_api::ExtensionData::new_with_init(
            thread_id.to_string(),
            thread_extension_init,
        );
        // Kick off independent async setup tasks in parallel to reduce startup latency.
        //
        // - initialize thread persistence with new or resumed session info
        // - perform default shell discovery
        // - load history metadata (skipped for subagents)
        let thread_persistence_fut = async {
            if config.ephemeral {
                Ok::<_, anyhow::Error>(None)
            } else {
                let live_thread = match &initial_history {
                    InitialHistory::New | InitialHistory::Cleared | InitialHistory::Forked(_) => {
                        live_thread_factory
                            .create(
                                Arc::clone(&thread_store),
                                CreateThreadParams {
                                    session_id,
                                    thread_id,
                                    extra_config: config.extra_config.clone(),
                                    forked_from_id,
                                    parent_thread_id,
                                    source: session_source,
                                    thread_source: session_configuration.thread_source.clone(),
                                    originator: session_configuration.originator.clone(),
                                    base_instructions: BaseInstructions {
                                        text: session_configuration.base_instructions.clone(),
                                    },
                                    dynamic_tools: session_configuration.dynamic_tools.clone(),
                                    selected_capability_roots: selected_capability_roots.clone(),
                                    multi_agent_version: initial_multi_agent_version,
                                    history_mode: session_configuration.history_mode,
                                    initial_window_id: initial_auto_compact_window_ids
                                        .window_id
                                        .to_string(),
                                    metadata: ThreadPersistenceMetadata {
                                        cwd: Some(config.cwd.to_path_buf()),
                                        model_provider: config.model_provider_id.clone(),
                                        memory_mode: if config.memories.generate_memories {
                                            ThreadMemoryMode::Enabled
                                        } else {
                                            ThreadMemoryMode::Disabled
                                        },
                                    },
                                },
                            )
                            .await?
                    }
                    InitialHistory::Resumed(resumed_history) => {
                        live_thread_factory
                            .resume(
                                Arc::clone(&thread_store),
                                ResumeThreadParams {
                                    thread_id: resumed_history.conversation_id,
                                    rollout_path: resumed_history.rollout_path.clone(),
                                    history: Some(resumed_history.history.clone()),
                                    include_archived: true,
                                    metadata: ThreadPersistenceMetadata {
                                        cwd: Some(config.cwd.to_path_buf()),
                                        model_provider: config.model_provider_id.clone(),
                                        memory_mode: if config.memories.generate_memories {
                                            ThreadMemoryMode::Enabled
                                        } else {
                                            ThreadMemoryMode::Disabled
                                        },
                                    },
                                },
                            )
                            .await?
                    }
                };
                Ok(Some(live_thread))
            }
        }
        .instrument(info_span!(
            "session_init.thread_persistence",
            otel.name = "session_init.thread_persistence",
            session_init.ephemeral = config.ephemeral,
        ));
        let state_db_fut = async {
            if config.ephemeral {
                None
            } else {
                state_db.clone()
            }
        }
        .instrument(info_span!(
            "session_init.state_db",
            otel.name = "session_init.state_db",
            session_init.ephemeral = config.ephemeral,
        ));

        let auth_manager_clone = Arc::clone(&auth_manager);
        let config_for_mcp = Arc::clone(&config);
        let mcp_manager_for_mcp = Arc::clone(&mcp_manager);
        let mcp_thread_init_for_startup = &mcp_thread_init;
        let thread_extension_data_for_mcp = &thread_extension_data;
        let mcp_originator = session_configuration.originator.clone();
        let mcp_runtime_cwd = session_configuration
            .environment_selections()
            .first()
            .and_then(|environment| environment.cwd.to_abs_path().ok())
            .map(|cwd| cwd.to_path_buf())
            .unwrap_or_else(|| session_configuration.cwd().to_path_buf());
        let mcp_runtime_context =
            McpRuntimeContext::new(Arc::clone(&environment_manager), mcp_runtime_cwd);
        let mcp_runtime_context_for_auth = mcp_runtime_context.clone();
        let auth_and_mcp_fut = async move {
            let auth = auth_manager_clone.auth().await;
            let mcp_projection = mcp_manager_for_mcp
                .runtime_config_for_step(
                    &config_for_mcp,
                    mcp_thread_init_for_startup,
                    thread_extension_data_for_mcp,
                    &mcp_originator,
                    /*available_environment_ids*/ &[],
                )
                .await;
            let mcp_config = &mcp_projection.config;
            let mcp_servers = codex_mcp::effective_mcp_servers(mcp_config, auth.as_ref());
            let tool_plugin_provenance = codex_mcp::tool_plugin_provenance(mcp_config);
            let auth_statuses = compute_auth_statuses(
                mcp_servers.iter(),
                config_for_mcp.mcp_oauth_credentials_store_mode,
                config_for_mcp.auth_keyring_backend_kind(),
                auth.as_ref(),
                &mcp_runtime_context_for_auth,
            )
            .await;
            (
                auth,
                mcp_projection,
                mcp_servers,
                auth_statuses,
                tool_plugin_provenance,
            )
        }
        .instrument(info_span!(
            "session_init.auth_mcp",
            otel.name = "session_init.auth_mcp",
        ));

        // Join all independent futures.
        let (
            thread_persistence_result,
            state_db_ctx,
            (auth, mcp_projection, mcp_servers, auth_statuses, tool_plugin_provenance),
        ) = tokio::join!(thread_persistence_fut, state_db_fut, auth_and_mcp_fut);

        let mut live_thread_init =
            LiveThreadInitGuard::new(thread_persistence_result.map_err(|e| {
                error!("failed to initialize thread persistence: {e:#}");
                e
            })?);
        let session_result: anyhow::Result<Arc<Self>> = async {
            let rollout_path = if let Some(live_thread) = live_thread_init.as_ref() {
                live_thread.local_rollout_path().await?
            } else {
                None
            };
            let trace_agent_path = session_configuration
                .session_source
                .get_agent_path()
                .unwrap_or_else(codex_protocol::AgentPath::root);
            let trace_task_name =
                (!trace_agent_path.is_root()).then(|| trace_agent_path.name().to_string());
            let trace_metadata = ThreadStartedTraceMetadata {
                thread_id: thread_id.to_string(),
                agent_path: trace_agent_path.to_string(),
                task_name: trace_task_name,
                nickname: session_configuration.session_source.get_nickname(),
                agent_role: session_configuration.session_source.get_agent_role(),
                session_source: session_configuration.session_source.clone(),
                cwd: session_configuration.cwd().to_path_buf(),
                rollout_path: rollout_path.clone(),
                model: session_configuration.collaboration_mode.model().to_string(),
                provider_name: config.model_provider_id.clone(),
                approval_policy: session_configuration.approval_policy.value().to_string(),
                sandbox_policy: format!("{:?}", session_configuration.sandbox_policy()),
            };
            let rollout_thread_trace = if matches!(
                session_configuration.session_source,
                SessionSource::SubAgent(SubAgentSource::ThreadSpawn { .. })
            ) {
                // Spawned child threads are part of their root rollout tree. If the
                // parent had no trace bundle, do not create an orphan child bundle
                // that looks like an independent rollout.
                parent_rollout_thread_trace.start_child_thread_trace_or_disabled(trace_metadata)
            } else {
                ThreadTraceContext::start_root_or_disabled(trace_metadata)
            };

            let mut post_session_configured_events = Vec::<Event>::new();

            for usage in config.features.legacy_feature_usages() {
                post_session_configured_events.push(Event {
                    id: INITIAL_SUBMIT_ID.to_owned(),
                    msg: EventMsg::DeprecationNotice(DeprecationNoticeEvent {
                        summary: usage.summary.clone(),
                        details: usage.details.clone(),
                    }),
                });
            }
            if crate::config::uses_deprecated_instructions_file(&config.config_layer_stack) {
                post_session_configured_events.push(Event {
                    id: INITIAL_SUBMIT_ID.to_owned(),
                    msg: EventMsg::DeprecationNotice(DeprecationNoticeEvent {
                        summary: "`experimental_instructions_file` is deprecated and ignored. Use `model_instructions_file` instead."
                            .to_string(),
                        details: Some(
                            "Move the setting to `model_instructions_file` in config.toml (or under a profile) to load instructions from a file."
                                .to_string(),
                        ),
                    }),
                });
            }
            for message in &config.startup_warnings {
                post_session_configured_events.push(Event {
                    id: "".to_owned(),
                    msg: EventMsg::Warning(WarningEvent {
                        message: message.clone(),
                    }),
                });
            }
            let config_path = config.codex_home.join(CONFIG_TOML_FILE);
            if let Some(event) = unstable_features_warning_event(
                config
                    .config_layer_stack
                    .effective_config()
                    .get("features")
                    .and_then(TomlValue::as_table),
                config.suppress_unstable_features_warning,
                &config.features,
                &config_path.display().to_string(),
            ) {
                post_session_configured_events.push(event);
            }
            let auth = auth.as_ref();
            let auth_mode = auth.map(CodexAuth::auth_mode).map(TelemetryAuthMode::from);
            let account_id = auth.and_then(CodexAuth::get_account_id);
            let account_email = auth.and_then(CodexAuth::get_account_email);
            let originator = session_configuration.originator.clone();
            let terminal_type = user_agent();
            let session_model = session_configuration.collaboration_mode.model().to_string();
            let auth_env_telemetry = collect_auth_env_telemetry(
                &session_configuration.provider,
                auth_manager.codex_api_key_env_enabled(),
            );
            let mut session_telemetry = SessionTelemetry::new(
                thread_id,
                session_model.as_str(),
                session_model.as_str(),
                account_id.clone(),
                account_email.clone(),
                auth_mode,
                originator.clone(),
                config.otel.log_user_prompt,
                terminal_type.clone(),
                session_configuration.session_source.clone(),
            )
            .with_auth_env(auth_env_telemetry.to_otel_metadata());
            if let Some(service_name) = session_configuration.metrics_service_name.as_deref() {
                session_telemetry = session_telemetry.with_metrics_service_name(service_name);
            }
            let network_proxy_audit_metadata = NetworkProxyAuditMetadata {
                conversation_id: Some(thread_id.to_string()),
                app_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                user_account_id: account_id,
                auth_mode: auth_mode.map(|mode| mode.to_string()),
                originator: Some(originator),
                user_email: account_email,
                terminal_type: Some(terminal_type),
                model: Some(session_model.clone()),
                slug: Some(session_model),
            };
            for spec in FEATURES {
                let enabled = config.features.get().enabled(spec.id);
                session_telemetry.counter(
                    "codex.feature.state",
                    /*inc*/ 1,
                    &[("feature", spec.key), ("value", &enabled.to_string())],
                );
            }
            session_telemetry.counter(
                THREAD_STARTED_METRIC,
                /*inc*/ 1,
                &[(
                    "is_git",
                    if get_git_repo_root(session_configuration.cwd()).is_some() {
                        "true"
                    } else {
                        "false"
                    },
                )],
            );

            session_telemetry.conversation_starts(
                config.model_provider.name.as_str(),
                session_configuration.collaboration_mode.reasoning_effort(),
                config
                    .model_reasoning_summary
                    .unwrap_or(ReasoningSummaryConfig::Auto),
                config.model_context_window,
                config.model_auto_compact_token_limit,
                config.permissions.approval_policy.value(),
                config
                    .permissions
                    .legacy_sandbox_policy(session_configuration.cwd().as_path()),
                mcp_servers.keys().map(String::as_str).collect(),
            );

            let use_zsh_fork_shell = config.features.enabled(Feature::ShellZshFork);
            let default_shell = if let Some(user_shell_override) =
                session_configuration.user_shell_override.clone()
            {
                user_shell_override
            } else if use_zsh_fork_shell {
                let zsh_path = config.zsh_path.as_ref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "zsh fork feature enabled, but no packaged zsh fork is available for this install"
                    )
                })?;
                let zsh_path = zsh_path.to_path_buf();
                shell::get_shell(shell::ShellType::Zsh, Some(&zsh_path)).ok_or_else(|| {
                    anyhow::anyhow!(
                        "zsh fork feature enabled, but packaged zsh fork `{}` is not usable",
                        zsh_path.display()
                    )
                })?
            } else {
                shell::default_user_shell()
            };
            let shell_snapshot = if config.features.enabled(Feature::ShellSnapshot) {
                ShellSnapshot::new(
                    config.codex_home.clone(),
                    thread_id,
                    session_telemetry.clone(),
                    state_db_ctx.clone(),
                )
            } else {
                ShellSnapshot::disabled()
            };
            let turn_environments = Arc::new(ThreadEnvironments::new(
                // fork-local: keep an owned handle for the SessionServices.environment_manager
                // field (fork keeps it as a direct field); upstream moves it into ThreadEnvironments.
                Arc::clone(&environment_manager),
                default_shell.clone(),
                shell_snapshot,
                inherited_environments.unwrap_or_default(),
                config.features.enabled(Feature::DeferredExecutor),
            ));
            turn_environments.update_selections(session_configuration.environment_selections());
            let resolved_environments = turn_environments.snapshot().await;
            let agents_md_manager = Arc::new(AgentsMdManager::new(user_instructions));
            let plugin_skill_warmup = warm_plugins_and_skills_for_session_init(
                Arc::clone(&config),
                Arc::clone(&plugins_manager),
                Arc::clone(&skills_service),
                &resolved_environments,
            )
            .instrument(info_span!(
                "session_init.plugin_skill_warmup",
                otel.name = "session_init.plugin_skill_warmup",
            ));
            let ((), plugin_skill_errors) = tokio::join!(
                agents_md_manager.refresh(config.as_ref(), &resolved_environments),
                plugin_skill_warmup,
            );
            for err in &plugin_skill_errors {
                error!(
                    "failed to load skill {}: {}",
                    err.path.display(),
                    err.message
                );
            }
            let thread_name =
                thread_title_from_thread_store(live_thread_init.as_ref(), &thread_store, thread_id)
                    .instrument(info_span!(
                        "session_init.thread_name_lookup",
                        otel.name = "session_init.thread_name_lookup",
                    ))
                    .await;
            session_configuration.thread_name = thread_name.clone();
            validate_config_lock_if_configured(&session_configuration).await?;
            export_config_lock_if_configured(&session_configuration, thread_id).await?;
            let state = SessionState::new_with_auto_compact_window_ids(
                session_configuration.clone(),
                initial_auto_compact_window_ids,
            );
            let managed_network_requirements_configured = config
                .config_layer_stack
                .requirements_toml()
                .network
                .is_some();
            let managed_network_requirements_enabled = config.managed_network_requirements_enabled();
            let network_approval = Arc::new(NetworkApprovalService::default());
            // The managed proxy can call back into core for allowlist-miss decisions.
            let network_policy_decider_session = if managed_network_requirements_configured {
                config
                    .permissions
                    .network
                    .as_ref()
                    .map(|_| Arc::new(RwLock::new(std::sync::Weak::<Session>::new())))
            } else {
                None
            };
            let blocked_request_observer = if managed_network_requirements_configured {
                config
                    .permissions
                    .network
                    .as_ref()
                    .map(|_| build_blocked_request_observer(Arc::clone(&network_approval)))
            } else {
                None
            };
            let network_policy_decider =
                network_policy_decider_session
                    .as_ref()
                    .map(|network_policy_decider_session| {
                        build_network_policy_decider(
                            Arc::clone(&network_approval),
                            Arc::clone(network_policy_decider_session),
                        )
                    });
            let (network_proxy, session_network_proxy) =
                if let Some(spec) = config.permissions.network.as_ref() {
                    let current_exec_policy = exec_policy.current();
                    let (network_proxy, session_network_proxy) = Self::start_managed_network_proxy(
                        spec,
                        current_exec_policy.as_ref(),
                        config.permissions.permission_profile.get(),
                        network_policy_decider.as_ref().map(Arc::clone),
                        blocked_request_observer.as_ref().map(Arc::clone),
                        managed_network_requirements_configured,
                        network_proxy_audit_metadata.clone(),
                    )
                    .instrument(info_span!(
                        "session_init.network_proxy",
                        otel.name = "session_init.network_proxy",
                        session_init.managed_network_requirements_enabled =
                            managed_network_requirements_enabled,
                    ))
                    .await?;
                    (Some(network_proxy), Some(session_network_proxy))
                } else {
                    (None, None)
                };

            let hooks = build_hooks_for_config(
                &config,
                plugins_manager.as_ref(),
                &default_shell,
            )
            .await;
            for warning in hooks.startup_warnings() {
                post_session_configured_events.push(Event {
                    id: INITIAL_SUBMIT_ID.to_owned(),
                    msg: EventMsg::Warning(WarningEvent {
                        message: warning.clone(),
                    }),
                });
            }

            let analytics_events_client = analytics_events_client.unwrap_or_else(|| {
                AnalyticsEventsClient::new(
                    Arc::clone(&auth_manager),
                    config.chatgpt_base_url.trim_end_matches('/').to_string(),
                    config.analytics_enabled,
                    Box::new(codex_analytics::CustomFactReducer::default()),
                )
            });
            // Keep one stable manager handle for the session so extension resource clients
            // automatically observe the manager installed at startup and on later refreshes.
            let mcp_connection_manager = Arc::new(arc_swap::ArcSwap::from_pointee(
                McpConnectionManager::new_uninitialized_with_permission_profile(
                    &config.permissions.approval_policy,
                    &config.permissions.permission_profile(),
                    config.prefix_mcp_tool_names(),
                ),
            ));
            let session_extension_data =
                codex_extension_api::ExtensionData::new(session_id.to_string());
            session_extension_data.insert(McpResourceClient::new(Arc::clone(
                &mcp_connection_manager,
            )));
            // fork-local: blackboard session feature.
            let blackboard = new_blackboard_session(
                config.as_ref(),
                session_id.to_string(),
                thread_id.to_string(),
                &session_configuration.session_source,
            );
            for contributor in extensions.thread_lifecycle_contributors() {
                contributor.on_thread_start(codex_extension_api::ThreadStartInput {
                    config: config.as_ref(),
                    session_source: &session_configuration.session_source,
                    persistent_thread_state_available: state_db_ctx.is_some(),
                    environments: session_configuration.environment_selections(),
                    session_store: &session_extension_data,
                    thread_store: &thread_extension_data,
                }).await;
            }

            let services = SessionServices {
                // Initialize the MCP connection manager with an uninitialized
                // instance. It will be replaced with one created via
                // McpConnectionManager::new() once all its constructor args are
                // available. This also ensures `SessionConfigured` is emitted
                // before any MCP-related events. It is reasonable to consider
                // changing this to use Option or OnceCell, though the current
                // setup is straightforward enough and performs well.
                mcp_connection_manager,
                mcp_runtime: arc_swap::ArcSwapOption::empty(),
                mcp_projection_lock: Mutex::new(()),
                mcp_startup_cancellation_token: Mutex::new(CancellationToken::new()),
                unified_exec_manager: UnifiedExecProcessManager::new(
                    config.background_terminal_max_timeout,
                ),
                elicitations: crate::elicitation::ElicitationService::new(),
                shell_zsh_path: config.zsh_path.clone(),
                main_execve_wrapper_exe: config.main_execve_wrapper_exe.clone(),
                analytics_events_client,
                // Fork remnant: the snapshot watch channel has no live reader in the current
                // (upstream `ThreadEnvironments`-based) snapshot design. Provide a valid sender so
                // `maybe_refresh_shell_snapshot_for_cwd` -> `ShellSnapshot::refresh_snapshot` compiles;
                // the receiver is intentionally dropped.
                shell_snapshot_tx: watch::channel(None).0,
                hooks: arc_swap::ArcSwap::from_pointee(hooks),
                rollout_thread_trace,
                user_shell: Arc::new(default_shell),
                show_raw_agent_reasoning: config.show_raw_agent_reasoning,
                exec_policy,
                auth_manager: Arc::clone(&auth_manager),
                session_telemetry,
                models_manager: Arc::clone(&models_manager),
                tool_approvals: Mutex::new(ApprovalStore::default()),
                guardian_rejections: Mutex::new(HashMap::new()),
                guardian_rejection_circuit_breaker: Mutex::new(Default::default()),
                runtime_handle: tokio::runtime::Handle::current(),
                skills_service,
                agents_md_manager,
                plugins_manager: Arc::clone(&plugins_manager),
                mcp_manager: Arc::clone(&mcp_manager),
                extensions,
                // TODO(jif): extract session to share between sub-agents
                session_extension_data,
                thread_extension_data,
                selected_capability_roots,
                mcp_thread_init,
                supports_openai_form_elicitation: std::sync::atomic::AtomicBool::new(
                    supports_openai_form_elicitation,
                ),
                agent_control,
                network_proxy: arc_swap::ArcSwapOption::from(network_proxy.map(Arc::new)),
                network_proxy_audit_metadata,
                managed_network_requirements_configured,
                network_approval: Arc::clone(&network_approval),
                state_db: state_db_ctx.clone(),
                live_thread: live_thread_init.as_ref().cloned(),
                thread_store: Arc::clone(&thread_store),
                live_thread_factory: Arc::clone(&live_thread_factory),
                attestation_provider: attestation_provider.clone(),
                time_provider,
                model_client: ModelClient::new(
                    Some(Arc::clone(&auth_manager)),
                    if config.features.enabled(Feature::UseAgentIdentity) {
                        AgentIdentityAuthPolicy::ChatGptAuth
                    } else {
                        AgentIdentityAuthPolicy::JwtOnly
                    },
                    thread_id,
                    session_configuration.provider.clone(),
                    session_configuration.session_source.clone(),
                    session_configuration.originator.clone(),
                    config.model_verbosity,
                    config.features.enabled(Feature::EnableRequestCompression),
                    config.features.enabled(Feature::RuntimeMetrics),
                    Self::build_model_client_beta_features_header(config.as_ref()),
                    /*item_ids_enabled*/ config.features.enabled(Feature::ItemIds)
                        || matches!(
                            session_configuration.history_mode,
                            ThreadHistoryMode::Paginated
                        ),
                    /*concurrent_reasoning_summaries_enabled*/ config
                        .features
                        .enabled(Feature::ConcurrentReasoningSummaries),
                    attestation_provider,
                    config.http_client_factory(),
                )
                .with_prompt_cache_key_override(
                    crate::guardian::prompt_cache_key_override_for_review_session(
                        &session_configuration.session_source,
                        session_configuration.parent_thread_id,
                    ),
                ),
                code_mode_service: crate::tools::code_mode::CodeModeService::new(Arc::clone(
                    &code_mode_session_provider,
                )),
                blackboard,
                environment_manager,
                tool_search_handler_cache: Default::default(),
                turn_environments: Arc::clone(&turn_environments),
            };
            let (out_of_band_elicitation_paused, _out_of_band_elicitation_paused_rx) =
                watch::channel(false);

            let (mailbox, mailbox_rx) = Mailbox::new();
            let sess = Arc::new(Session {
                thread_id,
                installation_id,
                tx_event: tx_event.clone(),
                agent_status,
                out_of_band_elicitation_paused,
                state: Mutex::new(state),
                managed_network_proxy_refresh_lock: Semaphore::new(/*permits*/ 1),
                features: config.features.clone(),
                multi_agent_version,
                pending_mcp_server_refresh_config: Mutex::new(None),
                conversation: Arc::new(RealtimeConversationManager::new()),
                active_turn: Mutex::new(None),
                mailbox,
                mailbox_rx: Mutex::new(mailbox_rx),
                idle_pending_input: Mutex::new(Vec::new()),
                input_queue: InputQueue::new(),
                goal_runtime: GoalRuntimeState::new(),
                guardian_review_session: GuardianReviewSessionManager::default(),
                services,
                next_internal_sub_id: AtomicU64::new(0),
            });
            if let Some(network_policy_decider_session) = network_policy_decider_session {
                let mut guard = network_policy_decider_session.write().await;
                *guard = Arc::downgrade(&sess);
            }
            sess.services.blackboard.start();
            sess.services
                .blackboard
                .observe_path(config.cwd.as_ref())
                .await;
            sess.refresh_git_checkpoint_baseline(config.cwd.as_ref()).await;
            // Dispatch the SessionConfiguredEvent first and then report any errors.
            // If resuming, include converted initial messages in the payload so UIs can render them immediately.
            let initial_messages = initial_history.get_event_msgs();
            let events = std::iter::once(Event {
                id: INITIAL_SUBMIT_ID.to_owned(),
                msg: EventMsg::SessionConfigured(SessionConfiguredEvent {
                    session_id,
                    thread_id,
                    forked_from_id,
                    parent_thread_id,
                    thread_source: session_configuration.thread_source.clone(),
                    thread_name: session_configuration.thread_name.clone(),
                    model: session_configuration.collaboration_mode.model().to_string(),
                    model_provider_id: config.model_provider_id.clone(),
                    service_tier: session_configuration.service_tier.clone(),
                    approval_policy: session_configuration.approval_policy.value(),
                    approvals_reviewer: session_configuration.approvals_reviewer,
                    permission_profile: session_configuration.permission_profile(),
                    active_permission_profile: session_configuration.active_permission_profile(),
                    cwd: session_configuration.cwd().clone(),
                    reasoning_effort: session_configuration.collaboration_mode.reasoning_effort(),
                    initial_messages,
                    network_proxy: session_network_proxy.filter(|_| {
                        Self::managed_network_proxy_active_for_permission_profile(
                            session_configuration.permission_profile.get(),
                        )
                    }),
                    rollout_path,
                }),
            })
            .chain(post_session_configured_events.into_iter());
            for event in events {
                sess.send_event_raw(event).await;
            }

            let mcp_startup_cancellation_token = {
                let mut cancel_guard = sess.services.mcp_startup_cancellation_token.lock().await;
                cancel_guard.cancel();
                let cancel_token = CancellationToken::new();
                *cancel_guard = cancel_token.clone();
                cancel_token
            };
            let codex_apps_auth_manager =
                codex_mcp::host_owned_codex_apps_enabled(&mcp_projection.config, auth)
                    .then(|| Arc::clone(&sess.services.auth_manager));
            let mcp_connection_manager = McpConnectionManager::new(
                &mcp_servers,
                config.mcp_oauth_credentials_store_mode,
                config.auth_keyring_backend_kind(),
                auth_statuses,
                &session_configuration.approval_policy,
                INITIAL_SUBMIT_ID.to_owned(),
                tx_event.clone(),
                mcp_startup_cancellation_token,
                session_configuration.permission_profile(),
                mcp_runtime_context.clone(),
                config.codex_home.to_path_buf(),
                sess.services.mcp_manager.codex_apps_tools_cache(),
                codex_apps_tools_cache_key(auth),
                config.prefix_mcp_tool_names(),
                mcp_projection
                    .config
                    .client_elicitation_capability
                    .clone(),
                sess.services
                    .supports_openai_form_elicitation
                    .load(std::sync::atomic::Ordering::Relaxed),
                tool_plugin_provenance,
                auth,
                codex_apps_auth_manager,
                Some(sess.mcp_elicitation_reviewer()),
                Some(sess.mcp_elicitation_lifecycle()),
                codex_mcp::ElicitationRequestRouter::default(),
            )
            .instrument(info_span!(
                "session_init.mcp_manager_init",
                otel.name = "session_init.mcp_manager_init",
            ))
            .await;
            sess.services
                .install_mcp_connection_manager(
                    Arc::new(mcp_projection.config),
                    mcp_projection.plugins_available,
                    mcp_runtime_context,
                    /*available_environment_ids*/ Vec::new(),
                    mcp_connection_manager,
                )
                .await?;
            sess.schedule_startup_prewarm(session_configuration.base_instructions.clone())
                .await;
            let session_start_source = match &initial_history {
                InitialHistory::Resumed(_) => codex_hooks::SessionStartSource::Resume,
                InitialHistory::New | InitialHistory::Forked(_) => {
                    codex_hooks::SessionStartSource::Startup
                }
                InitialHistory::Cleared => codex_hooks::SessionStartSource::Clear,
            };

            // record_initial_history can emit events. We record only after the SessionConfiguredEvent is emitted.
            sess.record_initial_history(initial_history).await;
            {
                let mut state = sess.state.lock().await;
                state.queue_pending_session_start_source(session_start_source);
            }
            Ok(sess)
        }
        .await;
        match session_result {
            Ok(sess) => {
                live_thread_init.commit();
                Ok(sess)
            }
            Err(err) => {
                live_thread_init.discard().await;
                Err(err)
            }
        }
    }
}
