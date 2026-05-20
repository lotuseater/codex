use super::*;

impl Session {
    pub(crate) async fn get_total_token_usage(&self) -> i64 {
        let state = self.state.lock().await;
        state.get_total_token_usage(state.server_reasoning_included())
    }

    pub(crate) async fn semantic_compact_decision(
        &self,
        input: codex_context_reduction::SemanticCompactInput,
    ) -> codex_context_reduction::SemanticCompactDecision {
        let state = self.state.lock().await;
        state.semantic_compact_decision(input)
    }

    pub(crate) async fn refresh_git_checkpoint_baseline(&self, cwd: &Path) {
        if !self.enabled(Feature::SemanticCheckpointGitSync) {
            return;
        }
        let worktree = match checkpoint_git::worktree_key(cwd) {
            Ok(Some(worktree)) => worktree,
            Ok(None) => return,
            Err(err) => {
                warn!("failed to resolve git checkpoint worktree: {err}");
                return;
            }
        };
        match checkpoint_git::dirty_paths(cwd) {
            Ok(paths) => {
                let mut state = self.state.lock().await;
                state.set_git_checkpoint_baseline_dirty_paths(worktree, paths);
            }
            Err(err) => {
                warn!("failed to initialize git checkpoint baseline: {err}");
            }
        }
    }

    pub(crate) async fn semantic_checkpoint_git_sync(
        &self,
        turn_context: &TurnContext,
        reason: CompactionReason,
    ) -> checkpoint_git::GitCheckpointOutcome {
        if !turn_context
            .features
            .enabled(Feature::SemanticCheckpointGitSync)
        {
            return checkpoint_git::GitCheckpointOutcome::Disabled;
        }
        let worktree = match checkpoint_git::worktree_key(turn_context.cwd.as_ref()) {
            Ok(Some(worktree)) => worktree,
            Ok(None) => return checkpoint_git::GitCheckpointOutcome::NotRepository,
            Err(err) => return checkpoint_git::GitCheckpointOutcome::Failed(err),
        };
        let baseline_dirty_paths = {
            let state = self.state.lock().await;
            state.git_checkpoint_baseline_dirty_paths(&worktree)
        };
        let Some(baseline_dirty_paths) = baseline_dirty_paths else {
            return match checkpoint_git::dirty_paths(turn_context.cwd.as_ref()) {
                Ok(paths) => {
                    let mut state = self.state.lock().await;
                    state.set_git_checkpoint_baseline_dirty_paths(worktree, paths);
                    checkpoint_git::GitCheckpointOutcome::NoChanges
                }
                Err(err) => checkpoint_git::GitCheckpointOutcome::Failed(err),
            };
        };
        let title = "checkpoint: semantic compact";
        let body = format!(
            "Automatic checkpoint before semantic compaction.\n\nReason: {reason:?}\nTurn: {}",
            turn_context.sub_id
        );
        let (outcome, next_baseline) = checkpoint_git::commit_and_push_checkpoint(
            turn_context.cwd.as_ref(),
            &baseline_dirty_paths,
            title,
            &body,
        );
        let mut state = self.state.lock().await;
        state.set_git_checkpoint_baseline_dirty_paths(worktree, next_baseline);
        outcome
    }

    pub(crate) async fn write_semantic_compact_scratchpad(
        &self,
        turn_context: &TurnContext,
        reason: CompactionReason,
        git_summary: &str,
    ) -> Option<PathBuf> {
        let codex_home = {
            let state = self.state.lock().await;
            state.session_configuration.codex_home.clone()
        };
        match checkpoint_scratchpad::write_scratchpad(
            codex_home.as_ref(),
            self.conversation_id,
            &turn_context.sub_id,
            reason,
            git_summary,
        ) {
            Ok(path) => Some(path),
            Err(err) => {
                warn!("failed to write semantic compaction scratchpad: {err}");
                None
            }
        }
    }

    pub(crate) fn cleanup_semantic_compact_scratchpad(&self, path: Option<PathBuf>) {
        checkpoint_scratchpad::cleanup_scratchpad(path);
    }

    pub(crate) async fn last_user_message_is_continuation_for_semantic_compact(&self) -> bool {
        let state = self.state.lock().await;
        state
            .history
            .raw_items()
            .iter()
            .rev()
            .find_map(|item| match crate::parse_turn_item(item) {
                Some(codex_protocol::items::TurnItem::UserMessage(user_message)) => Some(
                    codex_agent_policy::is_continuation_message(&user_message.message()),
                ),
                _ => None,
            })
            .unwrap_or(false)
    }

    pub(crate) async fn record_regular_turn_finished_for_semantic_compact(
        &self,
        turn_token_usage: &TokenUsage,
        tool_calls: u64,
        git_commit_observed: bool,
        is_continuation_turn: bool,
    ) {
        let mut state = self.state.lock().await;
        state.record_regular_turn_finished_for_semantic_compact(semantic_compact_turn_input(
            turn_token_usage,
            tool_calls,
            git_commit_observed,
            is_continuation_turn,
        ));
    }

    pub(crate) async fn record_compaction_finished_for_semantic_compact(
        &self,
        reason: Option<CompactionReason>,
    ) {
        let mut state = self.state.lock().await;
        state.record_compaction_finished_for_semantic_compact(
            reason.and_then(compaction_reason_to_context_reduction_reason),
        );
    }

    pub(crate) async fn take_plan_self_review_checkpoint_slot(&self) -> bool {
        let turn_state = {
            let active = self.active_turn.lock().await;
            active
                .as_ref()
                .map(|active_turn| Arc::clone(&active_turn.turn_state))
        };
        let Some(turn_state) = turn_state else {
            return false;
        };
        turn_state
            .lock()
            .await
            .take_plan_self_review_checkpoint_slot()
    }

    pub(crate) async fn get_total_token_usage_breakdown(&self) -> TotalTokenUsageBreakdown {
        let state = self.state.lock().await;
        state.history.get_total_token_usage_breakdown()
    }

    pub(crate) async fn total_token_usage(&self) -> Option<TokenUsage> {
        let state = self.state.lock().await;
        state.token_info().map(|info| info.total_token_usage)
    }

    pub(crate) async fn visible_context_percent_used(&self) -> Option<i64> {
        let state = self.state.lock().await;
        let token_info = state.token_info()?;
        let token_info_percent = token_context_percent_used(
            token_info.last_token_usage.total_tokens,
            token_info.model_context_window,
        );
        let active_context_percent = token_context_percent_used(
            state.get_total_token_usage(state.server_reasoning_included()),
            token_info.model_context_window,
        );
        [token_info_percent, active_context_percent]
            .into_iter()
            .flatten()
            .max()
    }

    /// Returns the complete token usage snapshot currently cached for this session.
    ///
    /// Resume and fork reconstruction seed this state from the last persisted rollout
    /// `TokenCount` event. Callers that need to replay restored usage to a client
    /// should use this accessor instead of `total_token_usage`, because the app-server
    /// notification includes both total and last-turn usage.
    pub(crate) async fn token_usage_info(&self) -> Option<TokenUsageInfo> {
        let state = self.state.lock().await;
        state.token_info()
    }

    pub(crate) async fn get_estimated_token_count(
        &self,
        turn_context: &TurnContext,
    ) -> Option<i64> {
        let state = self.state.lock().await;
        state.history.estimate_token_count(turn_context)
    }

    pub(crate) async fn take_restored_session_auto_compact_pending(&self) -> bool {
        let mut state = self.state.lock().await;
        state.take_restored_session_auto_compact_pending()
    }

    pub(crate) async fn maybe_inject_task_memory_for_sampling(
        &self,
        input: &mut Vec<ResponseItem>,
        auto_compact_limit: i64,
    ) {
        let current_memory = crate::task_memory::build_task_memory(input);
        let contains_existing_task_memory = crate::task_memory::contains_task_memory_item(input);
        let existing_digest = crate::task_memory::find_task_memory_digest(input);
        if contains_existing_task_memory
            && existing_digest.as_deref()
                == current_memory
                    .as_ref()
                    .map(crate::task_memory::BuiltTaskMemory::digest)
            && existing_digest.is_some()
        {
            return;
        }

        if contains_existing_task_memory {
            crate::task_memory::remove_task_memory_items(input);
        }
        let Some(task_memory) = current_memory else {
            return;
        };

        let estimated_tokens = crate::task_memory::estimated_prompt_tokens(input);
        if !crate::task_memory::should_inject_under_pressure(estimated_tokens, auto_compact_limit) {
            return;
        }

        let real_user_message_count = crate::task_memory::real_user_message_count(input);
        {
            let mut state = self.state.lock().await;
            if !state.task_memory_throttle_state.should_inject(
                task_memory.digest(),
                real_user_message_count,
                std::time::Instant::now(),
            ) {
                return;
            }
        }

        let memory_item = task_memory.into_response_item();
        *input = compact::insert_initial_context_before_last_real_user_or_summary(
            std::mem::take(input),
            vec![memory_item],
        );
    }

    pub(crate) async fn reset_task_memory_throttle_after_compaction(&self, digest: Option<&str>) {
        let mut state = self.state.lock().await;
        state
            .task_memory_throttle_state
            .reset_after_compaction(digest);
    }
}
