//! Submission and slash-command dispatch for [`ChatComposer`]: building the
//! submission text/elements, queueing, and parsing/dispatching slash commands.
use super::*;

impl ChatComposer {
    /// Prepare text for submission/queuing. Returns None if submission should be suppressed.
    /// On success, clears pending paste payloads because placeholders have been expanded.
    ///
    /// When `record_history` is true, the final submission is stored for ↑/↓ recall.
    fn prepare_submission_text(
        &mut self,
        record_history: bool,
    ) -> Option<(String, Vec<TextElement>)> {
        self.prepare_submission_text_with_options(record_history, SlashValidation::Immediate)
    }

    fn prepare_submission_text_with_options(
        &mut self,
        record_history: bool,
        slash_validation: SlashValidation,
    ) -> Option<(String, Vec<TextElement>)> {
        let mut text = self.current_text();
        let original_input = text.clone();
        let original_text_elements = self.current_text_elements();
        let original_mention_bindings = self.snapshot_mention_bindings();
        let original_local_image_paths = self
            .attached_images
            .iter()
            .map(|img| img.path.clone())
            .collect::<Vec<_>>();
        let original_pending_pastes = self.pending_pastes.clone();
        let mut text_elements = original_text_elements.clone();
        let input_starts_with_space = original_input.starts_with(' ');
        self.recent_submission_mention_bindings.clear();
        self.textarea.set_text_clearing_elements("");
        self.is_bash_mode = false;

        if !self.pending_pastes.is_empty() {
            // Expand placeholders so element byte ranges stay aligned.
            let (expanded, expanded_elements) =
                Self::expand_pending_pastes(&text, text_elements, &self.pending_pastes);
            text = expanded;
            text_elements = expanded_elements;
        }

        let expanded_input = text.clone();

        // If there is neither text nor attachments, suppress submission entirely.
        text = text.trim().to_string();
        text_elements = Self::trim_text_elements(&expanded_input, &text, text_elements);

        if slash_validation == SlashValidation::Immediate
            && self.slash_commands_enabled()
            && let Some((name, _rest, _rest_offset)) = parse_slash_name(&text)
        {
            let treat_as_plain_text = input_starts_with_space || name.contains('/');
            if !treat_as_plain_text {
                let is_known = find_slash_command(
                    name,
                    self.builtin_command_flags(),
                    &self.service_tier_commands,
                )
                .is_some();
                if !is_known {
                    let message = format!(
                        r#"Unrecognized command '/{name}'. Type "/" for a list of supported commands."#
                    );
                    self.app_event_tx.send(AppEvent::InsertHistoryCell(Box::new(
                        history_cell::new_info_event(message, /*hint*/ None),
                    )));
                    self.set_text_content_with_mention_bindings(
                        original_input.clone(),
                        original_text_elements,
                        original_local_image_paths,
                        original_mention_bindings,
                    );
                    self.pending_pastes.clone_from(&original_pending_pastes);
                    self.textarea.set_cursor(original_input.len());
                    return None;
                }
            }
        }

        let actual_chars = text.chars().count();
        if actual_chars > MAX_USER_INPUT_TEXT_CHARS {
            let message = user_input_too_large_message(actual_chars);
            self.app_event_tx.send(AppEvent::InsertHistoryCell(Box::new(
                history_cell::new_error_event(message),
            )));
            self.set_text_content_with_mention_bindings(
                original_input.clone(),
                original_text_elements,
                original_local_image_paths,
                original_mention_bindings,
            );
            self.pending_pastes.clone_from(&original_pending_pastes);
            self.textarea.set_cursor(original_input.len());
            return None;
        }
        self.prune_attached_images_for_submission(&text, &text_elements);
        if text.is_empty() && self.attached_images.is_empty() && self.remote_image_urls.is_empty() {
            return None;
        }
        self.recent_submission_mention_bindings = original_mention_bindings.clone();
        if record_history
            && (!text.is_empty()
                || !self.attached_images.is_empty()
                || !self.remote_image_urls.is_empty())
        {
            let local_image_paths = self
                .attached_images
                .iter()
                .map(|img| img.path.clone())
                .collect();
            self.history.record_local_submission(HistoryEntry {
                text: text.clone(),
                text_elements: text_elements.clone(),
                local_image_paths,
                remote_image_urls: self.remote_image_urls.clone(),
                mention_bindings: original_mention_bindings,
                pending_pastes: Vec::new(),
            });
        }
        self.pending_pastes.clear();
        Some((text, text_elements))
    }

    /// Common logic for handling message submission/queuing.
    /// Returns the appropriate InputResult based on `should_queue`.
    pub(crate) fn handle_submission(&mut self, should_queue: bool) -> (InputResult, bool) {
        let result = self.handle_submission_with_time(should_queue, Instant::now());
        self.reset_vim_mode_after_successful_dispatch(&result.0);
        result
    }

    pub(crate) fn reset_vim_mode_after_successful_dispatch(&mut self, result: &InputResult) {
        if matches!(
            result,
            InputResult::Submitted { .. }
                | InputResult::Queued { .. }
                | InputResult::Command(_)
                | InputResult::ServiceTierCommand(_)
                | InputResult::CommandWithArgs(_, _, _)
        ) {
            self.textarea.enter_vim_normal_mode();
        }
    }

    fn handle_submission_with_time(
        &mut self,
        should_queue: bool,
        now: Instant,
    ) -> (InputResult, bool) {
        if should_queue {
            if let Some(pasted) = self.paste_burst.flush_before_modified_input() {
                self.handle_paste(pasted);
            }
            let raw_text = self.textarea.text();
            let defer_slash_validation =
                self.should_parse_as_slash_on_dequeue_from_raw_text(raw_text);
            if let Some((text, text_elements)) = self.prepare_submission_text_with_options(
                /*record_history*/ true,
                if defer_slash_validation {
                    SlashValidation::Deferred
                } else {
                    SlashValidation::Immediate
                },
            ) {
                let action = self.queued_input_action(&text, defer_slash_validation);
                return (
                    InputResult::Queued {
                        text,
                        text_elements,
                        action,
                    },
                    true,
                );
            }
            return (InputResult::None, true);
        }

        // If the first line is a bare built-in slash command (no args),
        // dispatch it even when the slash popup isn't visible. This preserves
        // the workflow: type a prefix ("/di"), press Tab to complete to
        // "/diff ", then press Enter/Ctrl+Shift+Q to run it. Tab moves the cursor beyond
        // the '/name' token and our caret-based heuristic hides the popup,
        // but Enter/Ctrl+Shift+Q should still dispatch the command rather than submit
        // literal text.
        if let Some(result) = self.try_dispatch_bare_slash_command() {
            return (result, true);
        }

        // If we're in a paste-like burst capture, treat Enter/Ctrl+Shift+Q as part of the burst
        // and accumulate it rather than submitting or inserting immediately.
        // Do not treat as paste inside a slash-command context.
        let in_slash_context = self.slash_commands_enabled()
            && !self.is_bash_mode
            && (matches!(self.active_popup, ActivePopup::Command(_))
                || self
                    .textarea
                    .text()
                    .lines()
                    .next()
                    .unwrap_or("")
                    .starts_with('/'));
        if !self.disable_paste_burst
            && self.paste_burst.is_active()
            && !in_slash_context
            && self.paste_burst.append_newline_if_active(now)
        {
            return (InputResult::None, true);
        }

        // During a paste-like burst, treat Enter/Ctrl+Shift+Q as a newline instead of submit.
        if !in_slash_context
            && !self.disable_paste_burst
            && self
                .paste_burst
                .newline_should_insert_instead_of_submit(now)
        {
            self.textarea.insert_str("\n");
            self.paste_burst.extend_window(now);
            return (InputResult::None, true);
        }

        let original_input = self.current_text();
        let original_text_elements = self.current_text_elements();
        let original_mention_bindings = self.snapshot_mention_bindings();
        let original_local_image_paths = self
            .attached_images
            .iter()
            .map(|img| img.path.clone())
            .collect::<Vec<_>>();
        let original_pending_pastes = self.pending_pastes.clone();
        if let Some(result) = self.try_dispatch_slash_command_with_args() {
            return (result, true);
        }

        if let Some((text, text_elements)) =
            self.prepare_submission_text(/*record_history*/ true)
        {
            if should_queue {
                (
                    InputResult::Queued {
                        text,
                        text_elements,
                        action: QueuedInputAction::Plain,
                    },
                    true,
                )
            } else {
                // Do not clear attached_images here; ChatWidget drains them via take_recent_submission_images().
                (
                    InputResult::Submitted {
                        text,
                        text_elements,
                    },
                    true,
                )
            }
        } else {
            // Restore text if submission was suppressed.
            self.set_text_content_with_mention_bindings(
                original_input,
                original_text_elements,
                original_local_image_paths,
                original_mention_bindings,
            );
            self.pending_pastes = original_pending_pastes;
            (InputResult::None, true)
        }
    }

    /// Check if the first line is a bare slash command (no args) and dispatch it.
    /// Returns Some(InputResult) if a command was dispatched, None otherwise.
    fn try_dispatch_bare_slash_command(&mut self) -> Option<InputResult> {
        if !self.slash_commands_enabled() || self.is_bash_mode {
            return None;
        }
        let text = self.textarea.text();
        let first_line = text.lines().next().unwrap_or("");
        let (name, rest, _rest_offset) = parse_slash_name(first_line)?;
        if !rest.is_empty() {
            return None;
        }
        let command = find_slash_command(
            name,
            self.builtin_command_flags(),
            &self.service_tier_commands,
        )?;
        if command.supports_inline_args()
            && parse_slash_name(text).is_some_and(|(_, full_rest, _)| !full_rest.is_empty())
        {
            return None;
        }
        if self.reject_slash_command_if_unavailable(&command) {
            self.stage_slash_command_history(&command);
            self.record_pending_slash_command_history();
            return Some(InputResult::None);
        }
        self.stage_slash_command_history(&command);
        self.textarea.set_text_clearing_elements("");
        self.is_bash_mode = false;
        Some(match command {
            SlashCommandItem::Builtin(cmd) => InputResult::Command(cmd),
            SlashCommandItem::ServiceTier(command) => InputResult::ServiceTierCommand(command),
        })
    }

    /// Check if the input is a slash command with args (e.g., /review args) and dispatch it.
    /// Returns Some(InputResult) if a command was dispatched, None otherwise.
    fn try_dispatch_slash_command_with_args(&mut self) -> Option<InputResult> {
        if !self.slash_commands_enabled() || self.is_bash_mode {
            return None;
        }
        let text = self.textarea.text().to_string();
        if text.starts_with(' ') {
            return None;
        }

        let (name, rest, rest_offset) = parse_slash_name(&text)?;
        if rest.is_empty() || name.contains('/') {
            return None;
        }

        let command = find_slash_command(
            name,
            self.builtin_command_flags(),
            &self.service_tier_commands,
        )?;

        if !command.supports_inline_args() {
            return None;
        }
        if self.reject_slash_command_if_unavailable(&command) {
            self.stage_slash_command_history(&command);
            self.record_pending_slash_command_history();
            return Some(InputResult::None);
        }

        self.stage_slash_command_history(&command);

        let mut args_elements =
            Self::slash_command_args_elements(rest, rest_offset, &self.textarea.text_elements());
        let trimmed_rest = rest.trim();
        args_elements = Self::trim_text_elements(rest, trimmed_rest, args_elements);
        let SlashCommandItem::Builtin(cmd) = command else {
            return None;
        };
        Some(InputResult::CommandWithArgs(
            cmd,
            trimmed_rest.to_string(),
            args_elements,
        ))
    }

    /// Expand pending placeholders and extract normalized inline-command args.
    ///
    /// Inline-arg commands are initially dispatched using the raw draft so command rejection does
    /// not consume user input. Once a command needs its args, this helper performs the usual
    /// submission preparation (paste expansion, element trimming) and rebases element ranges from
    /// full-text offsets to command-arg offsets.
    ///
    /// Callers that already staged slash-command history should normally pass `false` for
    /// `record_history`; otherwise a command such as `/plan investigate` would be entered into
    /// local recall through both the slash-command path and the message-submission path.
    pub(crate) fn prepare_inline_args_submission(
        &mut self,
        record_history: bool,
    ) -> Option<(String, Vec<TextElement>)> {
        let (prepared_text, prepared_elements) = self.prepare_submission_text(record_history)?;
        let (_, prepared_rest, prepared_rest_offset) = parse_slash_name(&prepared_text)?;
        let mut args_elements = Self::slash_command_args_elements(
            prepared_rest,
            prepared_rest_offset,
            &prepared_elements,
        );
        let trimmed_rest = prepared_rest.trim();
        args_elements = Self::trim_text_elements(prepared_rest, trimmed_rest, args_elements);
        Some((trimmed_rest.to_string(), args_elements))
    }

    fn reject_slash_command_if_unavailable(&self, command: &SlashCommandItem) -> bool {
        if !self.is_task_running || command.available_during_task() {
            return false;
        }
        let message = format!(
            "'/{}' is disabled while a task is in progress.",
            command.command()
        );
        self.app_event_tx.send(AppEvent::InsertHistoryCell(Box::new(
            history_cell::new_error_event(message),
        )));
        true
    }

    fn should_parse_as_slash_on_dequeue_from_raw_text(&self, text: &str) -> bool {
        self.slash_commands_enabled() && !text.starts_with(' ') && text.trim().starts_with('/')
    }

    fn queued_input_action(
        &self,
        prepared_text: &str,
        defer_slash_validation: bool,
    ) -> QueuedInputAction {
        if defer_slash_validation && prepared_text.starts_with('/') {
            QueuedInputAction::ParseSlash
        } else if prepared_text.starts_with('!') {
            QueuedInputAction::RunShell
        } else {
            QueuedInputAction::Plain
        }
    }

    /// Stage the current slash-command text for later local recall.
    ///
    /// Staging snapshots the rich composer state before the textarea is cleared. `ChatWidget`
    /// commits the staged entry after dispatch so command recall follows the submitted text, not
    /// the command outcome.
    fn stage_slash_command_history(&mut self, command: &SlashCommandItem) {
        if matches!(command, SlashCommandItem::Builtin(SlashCommand::Clear)) {
            return;
        }
        self.stage_slash_command_history_text(self.textarea.text().trim().to_string());
    }

    /// Stage a popup-selected command using its canonical command text.
    ///
    /// Popup filtering text can be partial, so recording the selected command avoids recalling
    /// `/di` after the user actually accepted `/diff`.
    pub(crate) fn stage_selected_slash_command_history(&mut self, command: &CommandItem) {
        if matches!(command, CommandItem::Builtin(SlashCommand::Clear)) {
            return;
        }
        self.stage_slash_command_history_text(format!("/{}", command.command()));
    }

    /// Store the provided command text and the current composer adornments in the pending slot.
    ///
    /// The pending entry intentionally has the same shape as other local history entries so recall
    /// can rehydrate attachments, mention bindings, and pending paste placeholders if command
    /// workflows start carrying those through in the future.
    fn stage_slash_command_history_text(&mut self, text: String) {
        self.pending_slash_command_history = Some(HistoryEntry {
            text,
            text_elements: self.textarea.text_elements(),
            local_image_paths: self
                .attached_images
                .iter()
                .map(|img| img.path.clone())
                .collect(),
            remote_image_urls: self.remote_image_urls.clone(),
            mention_bindings: self.snapshot_mention_bindings(),
            pending_pastes: self.pending_pastes.clone(),
        });
    }

    /// Translate full-text element ranges into command-argument ranges.
    ///
    /// `rest_offset` is the byte offset where `rest` begins in the full text.
    fn slash_command_args_elements(
        rest: &str,
        rest_offset: usize,
        text_elements: &[TextElement],
    ) -> Vec<TextElement> {
        if rest.is_empty() || text_elements.is_empty() {
            return Vec::new();
        }
        text_elements
            .iter()
            .filter_map(|elem| {
                if elem.byte_range.end <= rest_offset {
                    return None;
                }
                let start = elem.byte_range.start.saturating_sub(rest_offset);
                let mut end = elem.byte_range.end.saturating_sub(rest_offset);
                if start >= rest.len() {
                    return None;
                }
                end = end.min(rest.len());
                (start < end).then_some(elem.map_range(|_| ByteRange { start, end }))
            })
            .collect()
    }

}
