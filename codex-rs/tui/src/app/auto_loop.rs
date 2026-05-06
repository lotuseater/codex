use super::*;
use crate::app_event::AutoLoopUpdate;

const AUTO_LOOP_DISABLED_SLEEP: Duration = Duration::from_secs(60 * 60 * 24 * 365);

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
}

impl AutoLoopState {
    pub(super) fn new(settings: AutoLoopSettings) -> Self {
        Self {
            settings,
            last_activity_at: Instant::now(),
        }
    }

    pub(super) fn note_activity(&mut self) {
        self.last_activity_at = Instant::now();
    }

    pub(super) fn sleep_duration(&self) -> Duration {
        if !self.settings.enabled {
            return AUTO_LOOP_DISABLED_SLEEP;
        }
        self.settings
            .period
            .saturating_sub(self.last_activity_at.elapsed())
    }

    fn is_due(&self) -> bool {
        self.settings.enabled && self.last_activity_at.elapsed() >= self.settings.period
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
        if !self.auto_loop.is_due() {
            return;
        }

        let message = self.auto_loop.settings.message.clone();
        if self.chat_widget.submit_auto_loop_message(message) {
            self.auto_loop.note_activity();
            return;
        }

        let reason = match self.chat_widget.can_submit_auto_loop_message() {
            Ok(()) => "loop message is empty",
            Err(reason) => reason,
        };
        tracing::debug!(reason = %reason, "auto-loop postponed");
        self.auto_loop.note_activity();
    }
}
