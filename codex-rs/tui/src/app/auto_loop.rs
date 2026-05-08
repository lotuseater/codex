use super::*;
use crate::app_event::AutoLoopUpdate;
use crate::chatwidget::AutoLoopSubmissionContext;

const AUTO_LOOP_DISABLED_SLEEP: Duration = Duration::from_secs(60 * 60 * 24 * 365);
const AUTO_LOOP_PROMPT_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
struct PendingAutoLoopPrompt {
    signature: String,
    started_at: Instant,
}

#[derive(Debug, Clone)]
pub(crate) struct AutoLoopSettings {
    pub(crate) enabled: bool,
    pub(crate) period: Duration,
    pub(crate) message: String,
}

impl AutoLoopSettings {
    pub(crate) fn new(enabled: bool, period: Duration, message: String) -> Self {
        Self {
            enabled,
            period,
            message,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct AutoLoopState {
    pub(super) settings: AutoLoopSettings,
    last_activity_at: Instant,
    pending_prompt: Option<PendingAutoLoopPrompt>,
}

impl AutoLoopState {
    pub(super) fn new(settings: AutoLoopSettings) -> Self {
        Self {
            settings,
            last_activity_at: Instant::now(),
            pending_prompt: None,
        }
    }

    pub(super) fn note_activity(&mut self) {
        self.last_activity_at = Instant::now();
        self.pending_prompt = None;
    }

    pub(super) fn sleep_duration(&self) -> Duration {
        if !self.settings.enabled {
            return AUTO_LOOP_DISABLED_SLEEP;
        }
        let regular_sleep = self
            .settings
            .period
            .saturating_sub(self.last_activity_at.elapsed());
        regular_sleep.min(AUTO_LOOP_PROMPT_IDLE_TIMEOUT)
    }

    fn is_due(&self) -> bool {
        self.settings.enabled && self.last_activity_at.elapsed() >= self.settings.period
    }

    fn clear_prompt_wait(&mut self) {
        self.pending_prompt = None;
    }

    fn prompt_idle_duration(&mut self, signature: String) -> Duration {
        let now = Instant::now();
        if let Some(wait) = &self.pending_prompt
            && wait.signature == signature
        {
            return now.saturating_duration_since(wait.started_at);
        }

        let started_at = self.last_activity_at;
        self.pending_prompt = Some(PendingAutoLoopPrompt {
            signature,
            started_at,
        });
        now.saturating_duration_since(started_at)
    }

    fn status_line(&self) -> String {
        let state = if self.settings.enabled { "on" } else { "off" };
        format!(
            "Loop is {state}. period={}, message={:?}",
            crate::cli::format_loop_period(self.settings.period),
            self.settings.message
        )
    }
}

impl App {
    pub(crate) fn handle_auto_loop_update(&mut self, update: AutoLoopUpdate) {
        match update {
            AutoLoopUpdate::Status => {}
            AutoLoopUpdate::Enable => {
                self.auto_loop.settings.enabled = true;
                self.auto_loop.note_activity();
            }
            AutoLoopUpdate::Disable => {
                self.auto_loop.settings.enabled = false;
                self.auto_loop.note_activity();
            }
            AutoLoopUpdate::SetPeriod(period) => {
                self.auto_loop.settings.period = period;
                self.auto_loop.note_activity();
            }
            AutoLoopUpdate::SetMessage(message) => {
                self.auto_loop.settings.message = message;
                self.auto_loop.note_activity();
            }
        }
        self.chat_widget
            .add_info_message(self.auto_loop.status_line(), None);
    }

    pub(super) fn handle_auto_loop_tick(&mut self) {
        if !self.auto_loop.settings.enabled {
            return;
        }

        if let Some(signature) = self.chat_widget.auto_loop_prompt_signature() {
            let idle_duration = self.auto_loop.prompt_idle_duration(signature);
            if idle_duration < AUTO_LOOP_PROMPT_IDLE_TIMEOUT {
                return;
            }
            if self.chat_widget.try_handle_auto_loop_prompt() {
                self.auto_loop.note_activity();
            } else {
                tracing::debug!("auto-loop prompt action postponed");
                self.auto_loop.note_activity();
            }
            return;
        }

        self.auto_loop.clear_prompt_wait();
        if !self.auto_loop.is_due() {
            return;
        }
        self.try_submit_auto_loop_message(AutoLoopSubmissionContext::Periodic);
    }

    pub(super) fn handle_auto_loop_after_self_review(&mut self) -> bool {
        if !self.auto_loop.settings.enabled {
            return false;
        }
        self.try_submit_auto_loop_message(AutoLoopSubmissionContext::AfterSelfReview)
    }

    fn try_submit_auto_loop_message(&mut self, context: AutoLoopSubmissionContext) -> bool {
        let message = self.auto_loop.settings.message.clone();
        if self.chat_widget.submit_auto_loop_message(message, context) {
            self.auto_loop.note_activity();
            return true;
        }

        let reason = match self.chat_widget.can_submit_auto_loop_message() {
            Ok(()) => "loop message is empty",
            Err(reason) => reason,
        };
        tracing::debug!(reason = %reason, context = context.trace_name(), "auto-loop postponed");
        self.auto_loop.note_activity();
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleep_duration_checks_prompt_timeout_before_long_loop_period() {
        let state = AutoLoopState::new(AutoLoopSettings::new(
            /*enabled*/ true,
            Duration::from_secs(300),
            "go on".to_string(),
        ));

        assert!(state.sleep_duration() <= AUTO_LOOP_PROMPT_IDLE_TIMEOUT);
    }

    #[test]
    fn prompt_idle_duration_starts_from_last_activity() {
        let mut state = AutoLoopState::new(AutoLoopSettings::new(
            /*enabled*/ true,
            Duration::from_secs(300),
            "go on".to_string(),
        ));
        state.last_activity_at = Instant::now() - AUTO_LOOP_PROMPT_IDLE_TIMEOUT;

        assert!(
            state.prompt_idle_duration("request_user_input:call-1".to_string())
                >= AUTO_LOOP_PROMPT_IDLE_TIMEOUT
        );
    }
}
