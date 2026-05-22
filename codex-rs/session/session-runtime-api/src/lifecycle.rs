use codex_session_api::SessionLifecycleState;

use crate::SessionRuntimeCommand;

/// Decision made by the runtime lifecycle before dispatching a command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionRuntimeCommandDecision {
    Accepted,
    Rejected(SessionLifecycleState),
}

/// Records a lifecycle state change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionLifecycleTransition {
    previous: SessionLifecycleState,
    current: SessionLifecycleState,
}

impl SessionLifecycleTransition {
    pub fn new(previous: SessionLifecycleState, current: SessionLifecycleState) -> Self {
        Self { previous, current }
    }

    pub fn previous(self) -> SessionLifecycleState {
        self.previous
    }

    pub fn current(self) -> SessionLifecycleState {
        self.current
    }

    pub fn changed(self) -> bool {
        self.previous != self.current
    }
}

/// Small state machine for session runtime shutdown and terminal transitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionRuntimeLifecycle {
    current: SessionLifecycleState,
}

impl SessionRuntimeLifecycle {
    pub fn new(current: SessionLifecycleState) -> Self {
        Self { current }
    }

    pub fn created() -> Self {
        Self::new(SessionLifecycleState::Created)
    }

    pub fn current(&self) -> SessionLifecycleState {
        self.current
    }

    pub fn can_accept_input(&self) -> bool {
        matches!(
            self.current,
            SessionLifecycleState::Created | SessionLifecycleState::Active
        )
    }

    pub fn can_start_background_task(&self) -> bool {
        self.can_accept_input()
    }

    pub fn command_decision(
        &self,
        command: &SessionRuntimeCommand,
    ) -> SessionRuntimeCommandDecision {
        match command {
            SessionRuntimeCommand::SubmitInput { .. } if self.can_accept_input() => {
                SessionRuntimeCommandDecision::Accepted
            }
            SessionRuntimeCommand::SubmitInput { .. } => {
                SessionRuntimeCommandDecision::Rejected(self.current)
            }
            SessionRuntimeCommand::Shutdown => SessionRuntimeCommandDecision::Accepted,
        }
    }

    pub fn mark_active(&mut self) -> SessionLifecycleTransition {
        match self.current {
            SessionLifecycleState::Created => self.transition_to(SessionLifecycleState::Active),
            SessionLifecycleState::Active
            | SessionLifecycleState::Draining
            | SessionLifecycleState::Completed
            | SessionLifecycleState::Failed => self.unchanged(),
        }
    }

    pub fn request_shutdown(&mut self) -> SessionLifecycleTransition {
        match self.current {
            SessionLifecycleState::Created | SessionLifecycleState::Active => {
                self.transition_to(SessionLifecycleState::Draining)
            }
            SessionLifecycleState::Draining
            | SessionLifecycleState::Completed
            | SessionLifecycleState::Failed => self.unchanged(),
        }
    }

    pub fn mark_completed(&mut self) -> SessionLifecycleTransition {
        match self.current {
            SessionLifecycleState::Created
            | SessionLifecycleState::Active
            | SessionLifecycleState::Draining => {
                self.transition_to(SessionLifecycleState::Completed)
            }
            SessionLifecycleState::Completed | SessionLifecycleState::Failed => self.unchanged(),
        }
    }

    pub fn mark_failed(&mut self) -> SessionLifecycleTransition {
        match self.current {
            SessionLifecycleState::Created
            | SessionLifecycleState::Active
            | SessionLifecycleState::Draining => self.transition_to(SessionLifecycleState::Failed),
            SessionLifecycleState::Completed | SessionLifecycleState::Failed => self.unchanged(),
        }
    }

    fn transition_to(&mut self, current: SessionLifecycleState) -> SessionLifecycleTransition {
        let previous = self.current;
        self.current = current;
        SessionLifecycleTransition::new(previous, current)
    }

    fn unchanged(&self) -> SessionLifecycleTransition {
        SessionLifecycleTransition::new(self.current, self.current)
    }
}

/// Lifecycle states for one runtime-owned background task.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackgroundTaskState {
    Idle,
    Running,
    Aborting,
    Finished,
}

/// Records a background task state change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackgroundTaskTransition {
    previous: BackgroundTaskState,
    current: BackgroundTaskState,
}

impl BackgroundTaskTransition {
    pub fn new(previous: BackgroundTaskState, current: BackgroundTaskState) -> Self {
        Self { previous, current }
    }

    pub fn previous(self) -> BackgroundTaskState {
        self.previous
    }

    pub fn current(self) -> BackgroundTaskState {
        self.current
    }

    pub fn changed(self) -> bool {
        self.previous != self.current
    }
}

/// Small state machine for background task start, abort, and completion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackgroundTaskLifecycle {
    current: BackgroundTaskState,
}

impl Default for BackgroundTaskLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

impl BackgroundTaskLifecycle {
    pub fn new() -> Self {
        Self {
            current: BackgroundTaskState::Idle,
        }
    }

    pub fn current(&self) -> BackgroundTaskState {
        self.current
    }

    pub fn can_start(&self) -> bool {
        matches!(
            self.current,
            BackgroundTaskState::Idle | BackgroundTaskState::Finished
        )
    }

    pub fn start(&mut self) -> BackgroundTaskTransition {
        match self.current {
            BackgroundTaskState::Idle | BackgroundTaskState::Finished => {
                self.transition_to(BackgroundTaskState::Running)
            }
            BackgroundTaskState::Running | BackgroundTaskState::Aborting => self.unchanged(),
        }
    }

    pub fn request_abort(&mut self) -> BackgroundTaskTransition {
        match self.current {
            BackgroundTaskState::Running => self.transition_to(BackgroundTaskState::Aborting),
            BackgroundTaskState::Idle
            | BackgroundTaskState::Aborting
            | BackgroundTaskState::Finished => self.unchanged(),
        }
    }

    pub fn finish(&mut self) -> BackgroundTaskTransition {
        match self.current {
            BackgroundTaskState::Running | BackgroundTaskState::Aborting => {
                self.transition_to(BackgroundTaskState::Finished)
            }
            BackgroundTaskState::Idle | BackgroundTaskState::Finished => self.unchanged(),
        }
    }

    fn transition_to(&mut self, current: BackgroundTaskState) -> BackgroundTaskTransition {
        let previous = self.current;
        self.current = current;
        BackgroundTaskTransition::new(previous, current)
    }

    fn unchanged(&self) -> BackgroundTaskTransition {
        BackgroundTaskTransition::new(self.current, self.current)
    }
}
