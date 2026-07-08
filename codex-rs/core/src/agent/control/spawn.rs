use super::residency::is_v2_resident_session_source;
use super::*;
use crate::context::ContextualUserFragment;
use crate::context::InternalModelContextFragment;
use codex_protocol::config_types::MultiAgentMode;
use codex_state::DirectionalThreadSpawnEdgeStatus;

const AGENT_NAMES: &str = include_str!("../agent_names.txt");

/// Recover the session/thread source recorded in a resumed thread's first `SessionMeta`.
///
/// Upstream removed `InitialHistory::get_resumed_session_sources`; this mirrors the
/// equivalent free helper in `thread_manager` (only `Resumed` histories carry a source).
fn resumed_session_sources(
    history: &InitialHistory,
) -> Option<(SessionSource, Option<ThreadSource>)> {
    match history {
        InitialHistory::New | InitialHistory::Cleared | InitialHistory::Forked(_) => None,
        InitialHistory::Resumed(resumed) => resumed.history.iter().find_map(|item| match item {
            RolloutItem::SessionMeta(meta_line) => Some((
                meta_line.meta.source.clone(),
                meta_line.meta.thread_source.clone(),
            )),
            _ => None,
        }),
    }
}

struct SpawnAgentThreadInheritance {
    environments: Option<TurnEnvironmentSnapshot>,
    exec_policy: Option<Arc<crate::exec_policy::ExecPolicyManager>>,
    inherited_multi_agent_mode: Option<MultiAgentMode>,
}

fn default_agent_nickname_list() -> Vec<&'static str> {
    AGENT_NAMES
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect()
}

pub(super) fn agent_nickname_candidates(config: &Config, role_name: Option<&str>) -> Vec<String> {
    let role_name = role_name.unwrap_or(DEFAULT_ROLE_NAME);
    if let Some(candidates) =
        resolve_role_config(config, role_name).and_then(|role| role.nickname_candidates.clone())
    {
        return candidates;
    }

    default_agent_nickname_list()
        .into_iter()
        .map(ToOwned::to_owned)
        .collect()
}

fn keep_forked_rollout_item(item: &RolloutItem, preserve_reference_context_item: bool) -> bool {
    match item {
        RolloutItem::ResponseItem(ResponseItem::Message { role, phase, .. }) => match role.as_str()
        {
            "system" | "developer" | "user" => true,
            "assistant" => *phase == Some(MessagePhase::FinalAnswer),
            _ => false,
        },
        RolloutItem::ResponseItem(
            ResponseItem::AdditionalTools { .. }
            | ResponseItem::AgentMessage { .. }
            | ResponseItem::Reasoning { .. }
            | ResponseItem::LocalShellCall { .. }
            | ResponseItem::FunctionCall { .. }
            | ResponseItem::ToolSearchCall { .. }
            | ResponseItem::FunctionCallOutput { .. }
            | ResponseItem::CustomToolCall { .. }
            | ResponseItem::CustomToolCallOutput { .. }
            | ResponseItem::ToolSearchOutput { .. }
            | ResponseItem::WebSearchCall { .. }
            | ResponseItem::ImageGenerationCall { .. }
            | ResponseItem::Compaction { .. }
            | ResponseItem::CompactionTrigger { .. }
            | ResponseItem::ContextCompaction { .. }
            | ResponseItem::Other,
        ) => false,
        RolloutItem::InterAgentCommunication(_)
        | RolloutItem::InterAgentCommunicationMetadata { .. } => false,
        // Full-history forks preserve the cached prompt prefix and can keep diffing
        // from the parent's durable baseline. Truncated forks drop part of that prompt,
        // so they must rebuild context on their first child turn.
        RolloutItem::TurnContext(_) => preserve_reference_context_item,
        RolloutItem::Compacted(_) | RolloutItem::EventMsg(_) | RolloutItem::SessionMeta(_) => true,
    }
}

/// True when `text` is delegation-nudge material that must not leak into a
/// forked child's history: an exact match against a configured/generated usage
/// hint or the auto-coordinator framing (the existing exact-equality arm), or a
/// `<codex_internal_context ...>` fragment carrying nudge material — either
/// the mid-loop cadence reminder (stable marker) or the initial-context hint
/// item, whose wrapped body joins the hint with the coordinator framing and so
/// matches no single filter text exactly.
fn is_multi_agent_v2_usage_hint_text(text: &str, usage_hint_texts: &[String]) -> bool {
    if usage_hint_texts
        .iter()
        .any(|usage_hint_text| usage_hint_text == text)
    {
        return true;
    }
    InternalModelContextFragment::matches_text(text)
        && (text.contains(codex_agent_policy::MULTI_AGENT_V2_DELEGATION_REMINDER_MARKER)
            || usage_hint_texts.iter().any(|usage_hint_text| {
                !usage_hint_text.is_empty() && text.contains(usage_hint_text.as_str())
            }))
}

/// `retain_mut` predicate stripping delegation usage hints from forked-child
/// history. Matches on the hint TEXT regardless of role: the delegation nudges
/// may be delivered on the user channel (obeyed) rather than the developer
/// channel, and in either case they must be stripped from forked-subagent
/// history so they are not duplicated into every child. When EVERY content
/// block of a message matches, the whole message is dropped (the prior
/// whole-message behavior); when only SOME blocks match (e.g. the
/// auto-coordinator framing fused as a trailing block on a real user turn),
/// just the matching blocks are stripped and the message is kept.
fn retain_or_strip_multi_agent_v2_usage_hint_message(
    item: &mut ResponseItem,
    usage_hint_texts: &[String],
) -> bool {
    let ResponseItem::Message { content, .. } = item else {
        return true;
    };

    let is_hint_block = |content_item: &ContentItem| match content_item {
        ContentItem::InputText { text } => {
            is_multi_agent_v2_usage_hint_text(text, usage_hint_texts)
        }
        _ => false,
    };

    let matching_blocks = content.iter().filter(|block| is_hint_block(block)).count();
    if matching_blocks == 0 {
        return true;
    }
    if matching_blocks == content.len() {
        // Every block is hint material -- drop the whole message.
        return false;
    }
    // Mixed message: strip only the hint blocks, keep the user's own content.
    content.retain(|block| !is_hint_block(block));
    true
}

fn append_multi_agent_v2_subagent_usage_hint(
    mut operation: Op,
    subagent_usage_hint_text: Option<&str>,
) -> Op {
    let Some(subagent_usage_hint_text) = subagent_usage_hint_text else {
        return operation;
    };

    if let Op::UserInput { items, .. } = &mut operation {
        items.push(UserInput::Text {
            text: subagent_usage_hint_text.to_string(),
            text_elements: Vec::new(),
        });
    }

    operation
}

impl AgentControl {
    /// Spawn a new agent thread and submit the initial prompt.
    #[cfg(test)]
    pub(crate) async fn spawn_agent(
        &self,
        config: Config,
        initial_operation: Op,
        session_source: Option<SessionSource>,
    ) -> CodexResult<ThreadId> {
        let spawned_agent = Box::pin(self.spawn_agent_internal(
            config,
            initial_operation,
            session_source,
            SpawnAgentOptions::default(),
        ))
        .await?;
        Ok(spawned_agent.thread_id)
    }

    /// Spawn an agent thread with some metadata.
    pub(crate) async fn spawn_agent_with_metadata(
        &self,
        config: Config,
        initial_operation: Op,
        session_source: Option<SessionSource>,
        options: SpawnAgentOptions, // TODO(jif) drop with new fork.
    ) -> CodexResult<LiveAgent> {
        Box::pin(self.spawn_agent_internal(config, initial_operation, session_source, options))
            .await
    }

    pub(crate) async fn ensure_v2_agent_loaded(
        &self,
        config: Config,
        thread_id: ThreadId,
    ) -> CodexResult<()> {
        let state = self.upgrade()?;
        if state.get_thread(thread_id).await.is_ok() {
            self.touch_loaded_v2_residency(&state, thread_id).await;
            return Ok(());
        }
        if self.state.agent_metadata_for_thread(thread_id).is_none() {
            return Err(CodexErr::ThreadNotFound(thread_id));
        }

        let stored_thread = state
            .read_stored_thread(ReadThreadParams {
                thread_id,
                include_archived: true,
                include_history: true,
            })
            .await?;
        let stored_source = stored_thread.source.clone();
        let stored_parent_thread_id = stored_thread.parent_thread_id;
        let history = stored_thread
            .history
            .ok_or(CodexErr::ThreadNotFound(thread_id))?
            .items;
        let initial_history = InitialHistory::Resumed(ResumedHistory {
            conversation_id: thread_id,
            history: Arc::new(history),
            rollout_path: stored_thread.rollout_path,
        });
        if initial_history.get_multi_agent_version() != Some(MultiAgentVersion::V2) {
            return Err(CodexErr::ThreadNotFound(thread_id));
        }
        let residency_slot = self
            .reserve_v2_residency_slot(&state, &config, Some(thread_id))
            .await?;

        let (session_source, _) =
            resumed_session_sources(&initial_history).unwrap_or((stored_source, None));
        let parent_thread_id = initial_history
            .get_resumed_parent_thread_id()
            .or(stored_parent_thread_id);
        let inherited_environments = self
            .inherited_environments_for_source(&state, Some(&session_source))
            .await;
        let inherited_exec_policy = self
            .inherited_exec_policy_for_source(&state, Some(&session_source), &config)
            .await;

        match state
            .resume_thread_with_history_with_source(ResumeThreadWithHistoryOptions {
                config,
                initial_history,
                agent_control: self.clone(),
                session_source,
                parent_thread_id,
                inherited_environments,
                inherited_exec_policy,
            })
            .await
        {
            Ok(reloaded_thread) => {
                residency_slot.commit(reloaded_thread.thread_id);
                state.notify_thread_created(reloaded_thread.thread_id);
                Ok(())
            }
            Err(err) => {
                if state.get_thread(thread_id).await.is_ok() {
                    drop(residency_slot);
                    self.touch_loaded_v2_residency(&state, thread_id).await;
                    return Ok(());
                }
                Err(err)
            }
        }
    }

    async fn spawn_agent_internal(
        &self,
        config: Config,
        initial_operation: Op,
        session_source: Option<SessionSource>,
        options: SpawnAgentOptions,
    ) -> CodexResult<LiveAgent> {
        let state = self.upgrade()?;
        let multi_agent_version = state
            .effective_multi_agent_version_for_spawn(
                &InitialHistory::New,
                session_source.as_ref(),
                options.parent_thread_id,
                /*forked_from_thread_id*/ None,
                &config,
            )
            .await;
        if let Some(session_source) = session_source.as_ref() {
            self.ensure_execution_capacity(multi_agent_version, session_source)?;
        }
        let agent_max_threads = config.effective_agent_max_threads(multi_agent_version)?;
        let spawn_uses_v2_residency = multi_agent_version == MultiAgentVersion::V2
            && session_source
                .as_ref()
                .is_some_and(is_v2_resident_session_source);
        let residency_slot = if spawn_uses_v2_residency {
            Some(
                self.reserve_v2_residency_slot(&state, &config, /*protected_thread_id*/ None)
                    .await?,
            )
        } else {
            None
        };
        let reservation_max_threads = if spawn_uses_v2_residency {
            None
        } else {
            agent_max_threads
        };
        let mut reservation = self.state.reserve_spawn_slot(reservation_max_threads)?;
        let inheritance = SpawnAgentThreadInheritance {
            environments: self
                .inherited_environments_for_source(&state, session_source.as_ref())
                .await,
            exec_policy: self
                .inherited_exec_policy_for_source(&state, session_source.as_ref(), &config)
                .await,
            inherited_multi_agent_mode: options.initial_multi_agent_mode,
        };
        let (session_source, mut agent_metadata) = match session_source {
            Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth,
                agent_path,
                agent_role,
                ..
            })) => {
                let (session_source, agent_metadata) = self.prepare_thread_spawn(
                    &mut reservation,
                    &config,
                    parent_thread_id,
                    depth,
                    agent_path,
                    agent_role,
                    /*preferred_agent_nickname*/ None,
                )?;
                (Some(session_source), agent_metadata)
            }
            other => (other, AgentMetadata::default()),
        };
        let notification_source = session_source.clone();
        let subagent_usage_hint_text = if multi_agent_version == MultiAgentVersion::V2
            && config.multi_agent_v2.usage_hint_enabled
        {
            match config.multi_agent_v2.subagent_usage_hint_text.clone() {
                Some(text) => Some(text),
                // A forked child inherits the parent's (filtered) rollout and does
                // not rebuild fresh initial context, so the initial-context worker
                // hint (`multi_agents::usage_hint_text`, which falls back to the
                // generated default when unset) never fires for it. Mirror that
                // fallback here so a forked worker still receives its role guidance.
                // A non-forked worker already gets the hint from initial context, so
                // only fall back when forking to avoid a double injection.
                None if options.fork_mode.is_some() => Some(
                    codex_agent_policy::default_multi_agent_v2_subagent_usage_hint_text_with_k(
                        config.multi_agent_v2.plan_token_economy_delegation_k,
                    ),
                ),
                None => None,
            }
        } else {
            None
        };

        // The same `AgentControl` is sent to spawn the thread.
        let new_thread = match (session_source, options.fork_mode.as_ref(), inheritance) {
            (Some(session_source), Some(_), inheritance) => {
                Box::pin(self.spawn_forked_thread(
                    &state,
                    config,
                    session_source,
                    &options,
                    inheritance,
                    multi_agent_version,
                ))
                .await?
            }
            (Some(session_source), None, inheritance) => {
                Box::pin(state.spawn_new_thread_with_source(
                    config.clone(),
                    self.clone(),
                    session_source,
                    options.parent_thread_id,
                    /*forked_from_thread_id*/ None,
                    /*thread_source*/ Some(ThreadSource::Subagent),
                    /*metrics_service_name*/ None,
                    inheritance.inherited_multi_agent_mode,
                    inheritance.environments,
                    inheritance.exec_policy,
                    options.environments.clone(),
                ))
                .await?
            }
            (None, _, _) => Box::pin(state.spawn_new_thread(config.clone(), self.clone())).await?,
        };
        agent_metadata.agent_id = Some(new_thread.thread_id);
        reservation.commit(agent_metadata.clone());
        if let Some(residency_slot) = residency_slot {
            residency_slot.commit(new_thread.thread_id);
        }

        if let Some(SessionSource::SubAgent(
            subagent_source @ SubAgentSource::ThreadSpawn {
                parent_thread_id, ..
            },
        )) = notification_source.as_ref()
        {
            let client_metadata = match state.get_thread(*parent_thread_id).await {
                Ok(parent_thread) => {
                    parent_thread
                        .codex
                        .session
                        .app_server_client_metadata()
                        .await
                }
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        parent_thread_id = %parent_thread_id,
                        "skipping subagent thread analytics: failed to load parent thread metadata"
                    );
                    crate::session::session::AppServerClientMetadata {
                        client_name: None,
                        client_version: None,
                    }
                }
            };
            let thread_config = new_thread.thread.codex.thread_config_snapshot().await;
            let parent_thread_id = thread_config.parent_thread_id;
            emit_subagent_session_started(
                &new_thread
                    .thread
                    .codex
                    .session
                    .services
                    .analytics_events_client,
                client_metadata,
                new_thread.thread.codex.session.session_id(),
                new_thread.thread_id,
                parent_thread_id,
                thread_config,
                subagent_source.clone(),
            );
        }

        // Notify a new thread has been created. This notification will be processed by clients
        // to subscribe or drain this newly created thread.
        // TODO(jif) add helper for drain
        state.notify_thread_created(new_thread.thread_id);

        self.persist_thread_spawn_edge_for_source(
            new_thread.thread.as_ref(),
            new_thread.thread_id,
            notification_source.as_ref(),
        )
        .await;

        let initial_operation = append_multi_agent_v2_subagent_usage_hint(
            initial_operation,
            subagent_usage_hint_text.as_deref(),
        );
        self.send_input_after_capacity_check(new_thread.thread_id, &state, initial_operation)
            .await?;
        if multi_agent_version != MultiAgentVersion::V2 {
            let child_reference = agent_metadata
                .agent_path
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| new_thread.thread_id.to_string());
            self.maybe_start_completion_watcher(
                new_thread.thread_id,
                notification_source,
                child_reference,
                agent_metadata.agent_path.clone(),
            );
        }

        Ok(LiveAgent {
            thread_id: new_thread.thread_id,
            metadata: agent_metadata,
            status: self.get_status(new_thread.thread_id).await,
        })
    }

    async fn spawn_forked_thread(
        &self,
        state: &Arc<ThreadManagerState>,
        config: Config,
        session_source: SessionSource,
        options: &SpawnAgentOptions,
        inheritance: SpawnAgentThreadInheritance,
        multi_agent_version: MultiAgentVersion,
    ) -> CodexResult<crate::thread_manager::NewThread> {
        let SpawnAgentThreadInheritance {
            environments: inherited_environments,
            exec_policy: inherited_exec_policy,
            inherited_multi_agent_mode,
        } = inheritance;
        if options.fork_parent_spawn_call_id.is_none() {
            return Err(CodexErr::Fatal(
                "spawn_agent fork requires a parent spawn call id".to_string(),
            ));
        }
        let Some(fork_mode) = options.fork_mode.as_ref() else {
            return Err(CodexErr::Fatal(
                "spawn_agent fork requires a fork mode".to_string(),
            ));
        };
        let SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id, ..
        }) = &session_source
        else {
            return Err(CodexErr::Fatal(
                "spawn_agent fork requires a thread-spawn session source".to_string(),
            ));
        };

        let parent_thread_id = *parent_thread_id;
        let parent_thread = state.get_thread(parent_thread_id).await.ok();
        if let Some(parent_thread) = parent_thread.as_ref() {
            // `record_conversation_items` only queues persistence writes asynchronously.
            // Flush before snapshotting store history for a fork.
            parent_thread.ensure_rollout_materialized().await;
            parent_thread.flush_rollout().await?;
        }

        let parent_history = state
            .read_stored_thread(ReadThreadParams {
                thread_id: parent_thread_id,
                include_archived: true,
                include_history: true,
            })
            .await?
            .history
            .ok_or_else(|| {
                CodexErr::Fatal(format!(
                    "parent thread history unavailable for fork: {parent_thread_id}"
                ))
            })?;

        let mut forked_rollout_items = parent_history.items;
        if let SpawnAgentForkMode::LastNTurns(last_n_turns) = fork_mode {
            forked_rollout_items =
                truncate_rollout_to_last_n_fork_turns(&forked_rollout_items, *last_n_turns);
        }
        let multi_agent_v2_usage_hint_texts_to_filter: Vec<String> = if let Some(parent_thread) =
            parent_thread.as_ref()
        {
            if multi_agent_version == MultiAgentVersion::V2 {
                let parent_config = parent_thread.codex.session.get_config().await;
                let parent_k = parent_config.multi_agent_v2.plan_token_economy_delegation_k;
                [
                    parent_config
                        .multi_agent_v2
                        .root_agent_usage_hint_text
                        .clone(),
                    parent_config
                        .multi_agent_v2
                        .subagent_usage_hint_text
                        .clone(),
                    // When the overrides above are unset (the default), the
                    // parent's history carries the GENERATED hint texts,
                    // spliced with the parent's resolved K, and either
                    // channel may also carry the auto-coordinator framing;
                    // filter those too.
                    Some(
                        codex_agent_policy::default_multi_agent_v2_root_usage_hint_text_with_k(
                            parent_k,
                        ),
                    ),
                    Some(
                        codex_agent_policy::default_multi_agent_v2_subagent_usage_hint_text_with_k(
                            parent_k,
                        ),
                    ),
                    Some(codex_agent_policy::AUTO_COORDINATOR_FRAMING_TEXT.to_string()),
                ]
                .into_iter()
                .flatten()
                .collect()
            } else {
                Vec::new()
            }
        } else if multi_agent_version == MultiAgentVersion::V2 {
            let k = config.multi_agent_v2.plan_token_economy_delegation_k;
            [
                config.multi_agent_v2.root_agent_usage_hint_text.clone(),
                config.multi_agent_v2.subagent_usage_hint_text.clone(),
                Some(codex_agent_policy::default_multi_agent_v2_root_usage_hint_text_with_k(k)),
                Some(codex_agent_policy::default_multi_agent_v2_subagent_usage_hint_text_with_k(k)),
                Some(codex_agent_policy::AUTO_COORDINATOR_FRAMING_TEXT.to_string()),
            ]
            .into_iter()
            .flatten()
            .collect()
        } else {
            Vec::new()
        };
        let preserve_reference_context_item = matches!(fork_mode, SpawnAgentForkMode::FullHistory);
        forked_rollout_items.retain_mut(|item| {
            if !keep_forked_rollout_item(item, preserve_reference_context_item) {
                return false;
            }
            match item {
                RolloutItem::ResponseItem(response_item) => {
                    retain_or_strip_multi_agent_v2_usage_hint_message(
                        response_item,
                        &multi_agent_v2_usage_hint_texts_to_filter,
                    )
                }
                _ => true,
            }
        });
        for item in &mut forked_rollout_items {
            if let RolloutItem::Compacted(compacted) = item
                && let Some(replacement_history) = compacted.replacement_history.as_mut()
            {
                replacement_history.retain_mut(|response_item| {
                    retain_or_strip_multi_agent_v2_usage_hint_message(
                        response_item,
                        &multi_agent_v2_usage_hint_texts_to_filter,
                    )
                });
            }
        }
        if preserve_reference_context_item
            && multi_agent_version == MultiAgentVersion::V2
            && let Some(subagent_usage_hint_text) =
                config.multi_agent_v2.subagent_usage_hint_text.clone()
            && let Some(subagent_usage_hint_message) =
                crate::context_manager::updates::build_developer_update_item(vec![
                    subagent_usage_hint_text,
                ])
        {
            forked_rollout_items.push(RolloutItem::ResponseItem(subagent_usage_hint_message));
        }

        state
            .fork_thread_with_source(
                config.clone(),
                InitialHistory::Forked(forked_rollout_items),
                self.clone(),
                session_source,
                /*thread_source*/ Some(ThreadSource::Subagent),
                /*parent_thread_id*/ Some(parent_thread_id),
                /*forked_from_thread_id*/ Some(parent_thread_id),
                inherited_multi_agent_mode,
                inherited_environments,
                inherited_exec_policy,
                options.environments.clone(),
            )
            .await
    }

    /// Resume an existing agent thread from a recorded rollout file.
    pub(crate) async fn resume_agent_from_rollout(
        &self,
        config: Config,
        thread_id: ThreadId,
        session_source: SessionSource,
    ) -> CodexResult<ThreadId> {
        let root_depth = thread_spawn_depth(&session_source).unwrap_or(0);
        let (resumed_thread_id, resumed_multi_agent_version) = Box::pin(
            self.resume_single_agent_from_rollout(config.clone(), thread_id, session_source),
        )
        .await?;
        let state = self.upgrade()?;
        if config.multi_agent_version_from_features() == MultiAgentVersion::V2
            || resumed_multi_agent_version == MultiAgentVersion::V2
        {
            return Ok(resumed_thread_id);
        }
        let Ok(resumed_thread) = state.get_thread(resumed_thread_id).await else {
            return Ok(resumed_thread_id);
        };
        let Some(state_db_ctx) = resumed_thread.state_db() else {
            return Ok(resumed_thread_id);
        };

        let mut resume_queue = VecDeque::from([(thread_id, root_depth)]);
        while let Some((parent_thread_id, parent_depth)) = resume_queue.pop_front() {
            let child_ids = match state_db_ctx
                .list_thread_spawn_children_with_status(
                    parent_thread_id,
                    DirectionalThreadSpawnEdgeStatus::Open,
                )
                .await
            {
                Ok(child_ids) => child_ids,
                Err(err) => {
                    warn!(
                        "failed to load persisted thread-spawn children for {parent_thread_id}: {err}"
                    );
                    continue;
                }
            };

            for child_thread_id in child_ids {
                let child_depth = parent_depth + 1;
                let child_resumed = if state.get_thread(child_thread_id).await.is_ok() {
                    true
                } else {
                    let child_session_source =
                        SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                            parent_thread_id,
                            depth: child_depth,
                            agent_path: None,
                            agent_nickname: None,
                            agent_role: None,
                        });
                    match Box::pin(self.resume_single_agent_from_rollout(
                        config.clone(),
                        child_thread_id,
                        child_session_source,
                    ))
                    .await
                    {
                        Ok((_, _)) => true,
                        Err(err) => {
                            warn!("failed to resume descendant thread {child_thread_id}: {err}");
                            false
                        }
                    }
                };
                if child_resumed {
                    resume_queue.push_back((child_thread_id, child_depth));
                }
            }
        }

        Ok(resumed_thread_id)
    }

    async fn resume_single_agent_from_rollout(
        &self,
        config: Config,
        thread_id: ThreadId,
        session_source: SessionSource,
    ) -> CodexResult<(ThreadId, MultiAgentVersion)> {
        let state = self.upgrade()?;
        let state_db_ctx = state.state_db();
        let stored_thread = state
            .read_stored_thread(ReadThreadParams {
                thread_id,
                include_archived: true,
                include_history: true,
            })
            .await?;
        let history = stored_thread
            .history
            .ok_or_else(|| CodexErr::ThreadNotFound(thread_id))?
            .items;
        let initial_history = InitialHistory::Resumed(ResumedHistory {
            conversation_id: thread_id,
            history: Arc::new(history),
            rollout_path: stored_thread.rollout_path,
        });
        let parent_thread_id = stored_thread.parent_thread_id;
        let multi_agent_version = state
            .effective_multi_agent_version_for_spawn(
                &initial_history,
                Some(&session_source),
                parent_thread_id,
                /*forked_from_thread_id*/ None,
                &config,
            )
            .await;
        let agent_max_threads = config.effective_agent_max_threads(multi_agent_version)?;
        let mut reservation = self.state.reserve_spawn_slot(agent_max_threads)?;
        let (session_source, agent_metadata) = match session_source {
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth,
                agent_path,
                agent_role: _,
                agent_nickname: _,
            }) => {
                let (resumed_agent_nickname, resumed_agent_role) =
                    if let Some(state_db_ctx) = state_db_ctx.as_ref() {
                        match state_db_ctx.get_thread(thread_id).await {
                            Ok(Some(metadata)) => (metadata.agent_nickname, metadata.agent_role),
                            Ok(None) | Err(_) => (None, None),
                        }
                    } else {
                        (None, None)
                    };
                self.prepare_thread_spawn(
                    &mut reservation,
                    &config,
                    parent_thread_id,
                    depth,
                    agent_path,
                    resumed_agent_role,
                    resumed_agent_nickname,
                )?
            }
            other => (other, AgentMetadata::default()),
        };
        let notification_source = session_source.clone();
        let inherited_environments = self
            .inherited_environments_for_source(&state, Some(&session_source))
            .await;
        let inherited_exec_policy = self
            .inherited_exec_policy_for_source(&state, Some(&session_source), &config)
            .await;

        let resumed_thread = state
            .resume_thread_with_history_with_source(ResumeThreadWithHistoryOptions {
                config: config.clone(),
                initial_history,
                agent_control: self.clone(),
                session_source,
                parent_thread_id,
                inherited_environments,
                inherited_exec_policy,
            })
            .await?;
        let mut agent_metadata = agent_metadata;
        agent_metadata.agent_id = Some(resumed_thread.thread_id);
        reservation.commit(agent_metadata.clone());
        // Resumed threads are re-registered in-memory and need the same listener
        // attachment path as freshly spawned threads.
        state.notify_thread_created(resumed_thread.thread_id);
        if multi_agent_version != MultiAgentVersion::V2 {
            let child_reference = agent_metadata
                .agent_path
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| resumed_thread.thread_id.to_string());
            self.maybe_start_completion_watcher(
                resumed_thread.thread_id,
                Some(notification_source.clone()),
                child_reference,
                agent_metadata.agent_path.clone(),
            );
        }
        self.persist_thread_spawn_edge_for_source(
            resumed_thread.thread.as_ref(),
            resumed_thread.thread_id,
            Some(&notification_source),
        )
        .await;

        Ok((resumed_thread.thread_id, multi_agent_version))
    }
}
