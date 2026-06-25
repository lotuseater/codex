use super::*;
use codex_protocol::protocol::MultiAgentVersion;
use codex_utils_path_uri::PathUri;

#[derive(Debug, PartialEq)]
pub enum SteerInputError {
    NoActiveTurn(Vec<UserInput>),
    ExpectedTurnMismatch { expected: String, actual: String },
    ActiveTurnNotSteerable { turn_kind: NonSteerableTurnKind },
    EmptyInput,
}

impl SteerInputError {
    pub(crate) fn to_error_event(&self) -> ErrorEvent {
        match self {
            Self::NoActiveTurn(_) => ErrorEvent {
                message: "no active turn to steer".to_string(),
                codex_error_info: Some(CodexErrorInfo::BadRequest),
            },
            Self::ExpectedTurnMismatch { expected, actual } => ErrorEvent {
                message: format!("expected active turn id `{expected}` but found `{actual}`"),
                codex_error_info: Some(CodexErrorInfo::BadRequest),
            },
            Self::ActiveTurnNotSteerable { turn_kind } => {
                let turn_kind_label = match turn_kind {
                    NonSteerableTurnKind::Review => "review",
                    NonSteerableTurnKind::Compact => "compact",
                };
                ErrorEvent {
                    message: format!("cannot steer a {turn_kind_label} turn"),
                    codex_error_info: Some(CodexErrorInfo::ActiveTurnNotSteerable {
                        turn_kind: *turn_kind,
                    }),
                }
            }
            Self::EmptyInput => ErrorEvent {
                message: "input must not be empty".to_string(),
                codex_error_info: Some(CodexErrorInfo::BadRequest),
            },
        }
    }
}

/// The high-level interface to the Codex system.
/// It operates as a queue pair where you send submissions and receive events.
pub struct Codex {
    pub(crate) tx_sub: Sender<Submission>,
    pub(crate) rx_event: Receiver<Event>,
    // Last known status of the agent.
    pub(crate) agent_status: watch::Receiver<AgentStatus>,
    pub(crate) session: Arc<Session>,
    // Shared future for the background submission loop completion so multiple
    // callers can wait for shutdown.
    pub(crate) session_loop_termination: SessionLoopTermination,
}

pub(crate) type SessionLoopTermination = Shared<BoxFuture<'static, ()>>;

/// Wrapper returned by [`Codex::spawn`] containing the spawned [`Codex`] and
/// the unique session id.
pub struct CodexSpawnOk {
    pub codex: Codex,
    pub thread_id: ThreadId,
}

pub(crate) struct CodexSpawnArgs {
    pub(crate) config: Config,
    pub(crate) user_instructions: LoadedUserInstructions,
    pub(crate) installation_id: String,
    pub(crate) auth_manager: Arc<AuthManager>,
    pub(crate) models_manager: SharedModelsManager,
    pub(crate) environment_manager: Arc<EnvironmentManager>,
    pub(crate) skills_service: Arc<SkillsService>,
    pub(crate) plugins_manager: Arc<PluginsManager>,
    pub(crate) mcp_manager: Arc<McpManager>,
    pub(crate) extensions: Arc<codex_extension_api::ExtensionRegistry<crate::config::Config>>,
    pub(crate) conversation_history: InitialHistory,
    pub(crate) session_source: SessionSource,
    pub(crate) forked_from_thread_id: Option<ThreadId>,
    pub(crate) thread_source: Option<ThreadSource>,
    pub(crate) agent_control: AgentControl,
    pub(crate) dynamic_tools: Vec<DynamicToolSpec>,
    pub(crate) parent_thread_id: Option<ThreadId>,
    /// Effective originator for this thread's Responses requests and analytics
    /// events. Resolved by the thread manager (service-name / persisted /
    /// inherited / env / default precedence) and threaded into
    /// `SessionConfiguration.originator`.
    pub(crate) originator: String,
    pub(crate) inherited_multi_agent_version: Option<MultiAgentVersion>,
    pub(crate) initial_multi_agent_mode: Option<MultiAgentMode>,
    pub(crate) persist_extended_history: bool,
    pub(crate) metrics_service_name: Option<String>,
    pub(crate) inherited_exec_policy: Option<Arc<ExecPolicyManager>>,
    pub(crate) inherited_environments: Option<TurnEnvironmentSnapshot>,
    /// Parent rollout trace used only to derive fresh spawned child traces.
    ///
    /// Root sessions and non-thread-spawn subagents pass a disabled context;
    /// `Session::new` creates the root trace itself when rollout tracing is enabled.
    pub(crate) parent_rollout_thread_trace: ThreadTraceContext,
    pub(crate) user_shell_override: Option<shell::Shell>,
    pub(crate) parent_trace: Option<W3cTraceContext>,
    pub(crate) environment_selections: Vec<TurnEnvironmentSelection>,
    pub(crate) thread_extension_init: ExtensionDataInit,
    pub(crate) supports_openai_form_elicitation: bool,
    pub(crate) analytics_events_client: Option<AnalyticsEventsClient>,
    pub(crate) thread_store: Arc<dyn ThreadStore>,
    pub(crate) live_thread_factory: Arc<dyn LiveThreadFactory>,
    pub(crate) state_db: Option<state_db::StateDbHandle>,
    pub(crate) attestation_provider: Option<Arc<dyn AttestationProvider>>,
    pub(crate) external_time_provider: Option<Arc<dyn TimeProvider>>,
}

pub(crate) const INITIAL_SUBMIT_ID: &str = "";
pub(crate) const SUBMISSION_CHANNEL_CAPACITY: usize = 512;

impl Codex {
    /// Spawn a new [`Codex`] and initialize the session.
    pub(crate) async fn spawn(args: CodexSpawnArgs) -> CodexResult<CodexSpawnOk> {
        let parent_trace = match args.parent_trace {
            Some(trace) => {
                if codex_otel::context_from_w3c_trace_context(&trace).is_some() {
                    Some(trace)
                } else {
                    warn!("ignoring invalid thread spawn trace carrier");
                    None
                }
            }
            None => None,
        };
        let thread_spawn_span = info_span!("thread_spawn", otel.name = "thread_spawn");
        if let Some(trace) = parent_trace.as_ref() {
            let _ = set_parent_from_w3c_trace_context(&thread_spawn_span, trace);
        }
        Self::spawn_internal(CodexSpawnArgs {
            parent_trace,
            ..args
        })
        .instrument(thread_spawn_span)
        .await
    }

    async fn spawn_internal(args: CodexSpawnArgs) -> CodexResult<CodexSpawnOk> {
        let CodexSpawnArgs {
            mut config,
            user_instructions,
            installation_id,
            auth_manager,
            models_manager,
            environment_manager,
            skills_service,
            plugins_manager,
            mcp_manager,
            extensions,
            conversation_history,
            session_source,
            forked_from_thread_id,
            thread_source,
            agent_control,
            dynamic_tools,
            parent_thread_id,
            originator,
            inherited_multi_agent_version,
            initial_multi_agent_mode,
            persist_extended_history: _,
            metrics_service_name,
            user_shell_override,
            inherited_exec_policy,
            inherited_environments,
            parent_rollout_thread_trace,
            parent_trace: _,
            environment_selections,
            thread_extension_init,
            supports_openai_form_elicitation,
            analytics_events_client,
            thread_store,
            live_thread_factory,
            state_db,
            attestation_provider,
            external_time_provider,
        } = args;
        let (tx_sub, rx_sub) = async_channel::bounded(SUBMISSION_CHANNEL_CAPACITY);
        let (tx_event, rx_event) = async_channel::unbounded();

        let LoadedUserInstructions {
            instructions: user_instructions,
            // Recoverable user-instruction warnings are surfaced by the loader; the
            // merged `Config` has no `startup_warnings` field, so drop them here
            // (matching the fork's prior behavior).
            warnings: _user_instruction_provider_warnings,
        } = user_instructions;

        // The fork's `SessionConfiguration.user_instructions` field (restored by
        // A1) is `Option<LoadedAgentsMd>`. Build it from the loaded
        // `UserInstructions` via the public constructor so downstream readers
        // (e.g. `Codex::instruction_sources`) keep working, while the raw
        // `Option<UserInstructions>` is still handed to `Session::new`.
        let loaded_user_instructions = user_instructions
            .as_ref()
            .map(|ui| LoadedAgentsMd::new_user(ui.text.clone(), ui.source.clone()));

        if let SessionSource::SubAgent(SubAgentSource::ThreadSpawn { depth, .. }) = session_source
            && depth >= config.agent_max_depth
            && !config.features.enabled(Feature::MultiAgentV2)
        {
            let _ = config.features.disable(Feature::SpawnCsv);
            let _ = config.features.disable(Feature::Collab);
        }

        let exec_policy = if crate::guardian::is_guardian_reviewer_source(&session_source) {
            // Guardian review should rely on the built-in shell safety checks,
            // not on caller-provided exec-policy rules that could shape the
            // reviewer or silently auto-approve commands.
            Arc::new(ExecPolicyManager::default())
        } else if let Some(exec_policy) = &inherited_exec_policy {
            Arc::clone(exec_policy)
        } else {
            Arc::new(
                ExecPolicyManager::load(&config.config_layer_stack)
                    .await
                    .map_err(|err| CodexErr::Fatal(format!("failed to load rules: {err}")))?,
            )
        };

        let config = Arc::new(config);
        let refresh_strategy = if session_source.is_non_root_agent() {
            codex_models_manager::manager::RefreshStrategy::Offline
        } else {
            codex_models_manager::manager::RefreshStrategy::OnlineIfUncached
        };
        if config.model.is_none()
            || !matches!(
                refresh_strategy,
                codex_models_manager::manager::RefreshStrategy::Offline
            )
        {
            let _ = models_manager.list_models(refresh_strategy).await;
        }
        let model = models_manager
            .get_default_model(&config.model, refresh_strategy)
            .await;

        // Resolve base instructions for the session. Priority order:
        // 1. config.base_instructions override
        // 2. conversation history => session_meta.base_instructions
        // 3. base_instructions for current model
        let model_info = models_manager
            .get_model_info(model.as_str(), &config.to_models_manager_config())
            .await;
        let base_instructions = config
            .base_instructions
            .clone()
            .or_else(|| conversation_history.get_base_instructions().map(|s| s.text))
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality));

        // Respect thread-start tools. When missing (resumed/forked threads), read from the db
        // first, then fall back to rollout-file tools.
        let persisted_tools = if dynamic_tools.is_empty() {
            let thread_id = match &conversation_history {
                InitialHistory::Resumed(resumed) => Some(resumed.conversation_id),
                InitialHistory::Forked(_) => conversation_history.forked_from_id(),
                InitialHistory::New | InitialHistory::Cleared => None,
            };
            match thread_id {
                Some(thread_id) if !config.ephemeral => thread_store
                    .read_thread_dynamic_tools(ReadThreadDynamicToolsParams { thread_id })
                    .await
                    .unwrap_or(None),
                None => None,
                Some(_) => None,
            }
        } else {
            None
        };
        let dynamic_tools = if dynamic_tools.is_empty() {
            persisted_tools
                .or_else(|| conversation_history.get_dynamic_tools())
                .unwrap_or_default()
        } else {
            dynamic_tools
        };
        // TODO (aibrahim): Consolidate config.model and config.model_reasoning_effort into config.collaboration_mode
        // to avoid extracting these fields separately and constructing CollaborationMode here.
        let collaboration_mode = CollaborationMode {
            mode: ModeKind::Default,
            settings: Settings {
                model: model.clone(),
                reasoning_effort: config.model_reasoning_effort,
                developer_instructions: None,
            },
        };
        let service_tier = get_service_tier(
            config.service_tier.clone(),
            config.features.enabled(Feature::FastMode),
            &model_info,
        );
        let multi_agent_version = crate::session::resolve_multi_agent_version(
            &conversation_history,
            inherited_multi_agent_version,
        );
        let multi_agent_mode = initial_multi_agent_mode;

        let session_configuration = SessionConfiguration {
            provider: config.model_provider.clone(),
            collaboration_mode: collaboration_mode.clone(),
            // Upstream made `SessionConfiguration.multi_agent_mode` a non-optional
            // `MultiAgentMode`; the fork's spawn arg is still `Option` (absent on
            // most spawn paths). Map `None` to the enum default
            // (`ExplicitRequestOnly`) so the stored thread mode matches upstream's
            // convention without dropping any caller-provided mode.
            multi_agent_mode: multi_agent_mode.unwrap_or_default(),
            model_reasoning_summary: config.model_reasoning_summary,
            service_tier,
            context_budget_mode: config.context_budget_mode,
            developer_instructions: config.developer_instructions.clone(),
            loaded_agents_md: None,
            user_instructions: loaded_user_instructions,
            personality: config.personality,
            fork_features: ForkFeaturesState::new(
                collaboration_mode,
                config.context_budget_mode,
                config.personality,
            ),
            base_instructions,
            compact_prompt: config.compact_prompt.clone(),
            approval_policy: config.permissions.approval_policy.clone(),
            approvals_reviewer: config.approvals_reviewer,
            permission_profile: config.permissions.permission_profile.clone(),
            active_permission_profile: config.permissions.active_permission_profile(),
            windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
            workspace_roots: vec![config.cwd.clone()],
            profile_workspace_roots: Vec::new(),
            codex_home: config.codex_home.clone(),
            thread_name: None,
            // `cwd` is no longer a `SessionConfiguration` field; it lives as the
            // `legacy_fallback_cwd` inside the environments wrapper.
            environments: TurnEnvironmentSelections::new(
                config.cwd.clone(),
                environment_selections,
            ),
            original_config_do_not_use: Arc::clone(&config),
            metrics_service_name,
            app_server_client_name: None,
            app_server_client_version: None,
            session_source,
            forked_from_thread_id,
            parent_thread_id,
            thread_source,
            originator,
            dynamic_tools,
            user_shell_override,
        };

        // Generate a unique ID for the lifetime of this Codex session.
        let session_source_clone = session_configuration.session_source.clone();
        let (agent_status_tx, agent_status_rx) = watch::channel(AgentStatus::PendingInit);

        let session = Session::new(
            session_configuration,
            config.clone(),
            user_instructions,
            installation_id,
            auth_manager.clone(),
            models_manager.clone(),
            exec_policy,
            tx_event.clone(),
            agent_status_tx.clone(),
            conversation_history,
            session_source_clone,
            skills_service,
            plugins_manager,
            mcp_manager.clone(),
            extensions,
            thread_extension_init,
            supports_openai_form_elicitation,
            agent_control,
            environment_manager,
            inherited_environments,
            analytics_events_client,
            thread_store,
            live_thread_factory,
            state_db,
            parent_rollout_thread_trace,
            attestation_provider,
            external_time_provider,
            multi_agent_version,
        )
        .await
        .map_err(|e| {
            error!("Failed to create session: {e:#}");
            map_session_init_error(&e, &config.codex_home)
        })?;
        let thread_id = session.thread_id;

        // This task will run until Op::Shutdown is received.
        let session_for_loop = Arc::clone(&session);
        let session_loop_handle = tokio::spawn(async move {
            submission_loop(session_for_loop, config, rx_sub)
                .instrument(info_span!("session_loop", thread_id = %thread_id))
                .await;
        });
        let codex = Codex {
            tx_sub,
            rx_event,
            agent_status: agent_status_rx,
            session,
            session_loop_termination: session_loop_termination_from_handle(session_loop_handle),
        };

        Ok(CodexSpawnOk { codex, thread_id })
    }

    /// Submit the `op` wrapped in a `Submission` with a unique ID.
    pub async fn submit(&self, op: Op) -> CodexResult<String> {
        self.submit_with_trace(op, /*trace*/ None).await
    }

    pub async fn submit_with_trace(
        &self,
        op: Op,
        trace: Option<W3cTraceContext>,
    ) -> CodexResult<String> {
        let id = Uuid::now_v7().to_string();
        let sub = Submission {
            id: id.clone(),
            op,
            client_user_message_id: None,
            trace,
        };
        self.submit_with_id(sub).await?;
        Ok(id)
    }

    /// Use sparingly: prefer `submit()` so Codex is responsible for generating
    /// unique IDs for each submission.
    pub async fn submit_with_id(&self, mut sub: Submission) -> CodexResult<()> {
        if sub.trace.is_none() {
            sub.trace = current_span_w3c_trace_context();
        }
        self.tx_sub
            .send(sub)
            .await
            .map_err(|_| CodexErr::InternalAgentDied)?;
        Ok(())
    }

    /// Persist a thread-level memory mode update for the active session.
    ///
    /// This is a local-only operation that updates rollout metadata directly
    /// and does not involve the model.
    pub async fn set_thread_memory_mode(
        &self,
        mode: codex_protocol::protocol::ThreadMemoryMode,
    ) -> anyhow::Result<()> {
        handlers::persist_thread_memory_mode_update(&self.session, mode).await
    }

    pub async fn shutdown_and_wait(&self) -> CodexResult<()> {
        let session_loop_termination = self.session_loop_termination.clone();
        match self.submit(Op::Shutdown).await {
            Ok(_) => {}
            Err(CodexErr::InternalAgentDied) => {}
            Err(err) => return Err(err),
        }
        session_loop_termination.await;
        Ok(())
    }

    pub async fn next_event(&self) -> CodexResult<Event> {
        let event = self
            .rx_event
            .recv()
            .await
            .map_err(|_| CodexErr::InternalAgentDied)?;
        Ok(event)
    }

    pub async fn steer_input(
        &self,
        input: Vec<UserInput>,
        expected_turn_id: Option<&str>,
        responsesapi_client_metadata: Option<HashMap<String, String>>,
    ) -> Result<String, SteerInputError> {
        self.session
            .steer_input(input, expected_turn_id, responsesapi_client_metadata)
            .await
    }

    pub(crate) async fn set_app_server_client_info(
        &self,
        app_server_client_name: Option<String>,
        app_server_client_version: Option<String>,
        mcp_elicitations_auto_deny: bool,
    ) -> ConstraintResult<()> {
        self.session
            .update_settings(SessionSettingsUpdate {
                app_server_client_name,
                app_server_client_version,
                ..Default::default()
            })
            .await?;
        let mcp_connection_manager = self.session.services.mcp_connection_manager.load();
        mcp_connection_manager.set_elicitations_auto_deny(mcp_elicitations_auto_deny);
        Ok(())
    }

    pub(crate) async fn agent_status(&self) -> AgentStatus {
        self.agent_status.borrow().clone()
    }

    pub(crate) async fn thread_config_snapshot(&self) -> ThreadConfigSnapshot {
        let state = self.session.state.lock().await;
        state.session_configuration.thread_config_snapshot()
    }

    pub(crate) async fn instruction_sources(&self) -> Vec<PathUri> {
        let state = self.session.state.lock().await;
        state
            .session_configuration
            .user_instructions
            .as_ref()
            .map_or_else(Vec::new, |instructions| instructions.sources().collect())
    }

    pub(crate) async fn thread_environment_selections(&self) -> Vec<TurnEnvironmentSelection> {
        let state = self.session.state.lock().await;
        state
            .session_configuration
            .environment_selections()
            .to_vec()
    }

    pub(crate) fn state_db(&self) -> Option<state_db::StateDbHandle> {
        self.session.state_db()
    }

    pub(crate) fn enabled(&self, feature: Feature) -> bool {
        self.session.enabled(feature)
    }
}
