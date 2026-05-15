use std::collections::VecDeque;
use std::ops::Deref;
use std::ops::DerefMut;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueuedInputAction {
    Plain,
    AutomaticSelfReview,
    ParseSlash,
    RunShell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedInput<T, H> {
    input: T,
    history: H,
}

impl<T, H> QueuedInput<T, H> {
    pub fn new(input: T, history: H) -> Self {
        Self { input, history }
    }

    pub fn input(&self) -> &T {
        &self.input
    }

    pub fn history(&self) -> &H {
        &self.history
    }

    pub fn into_parts(self) -> (T, H) {
        (self.input, self.history)
    }
}

impl<T, H> Deref for QueuedInput<T, H> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.input
    }
}

impl<T, H> DerefMut for QueuedInput<T, H> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.input
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputQueue<T, H> {
    entries: VecDeque<QueuedInput<T, H>>,
}

impl<T, H> Default for InputQueue<T, H> {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }
}

impl<T, H> InputQueue<T, H> {
    pub fn push_back_with_history(&mut self, input: impl Into<T>, history: H) {
        self.entries
            .push_back(QueuedInput::new(input.into(), history));
    }

    pub fn push_front_with_history(&mut self, input: impl Into<T>, history: H) {
        self.entries
            .push_front(QueuedInput::new(input.into(), history));
    }

    pub fn pop_front(&mut self) -> Option<QueuedInput<T, H>> {
        self.entries.pop_front()
    }

    pub fn pop_back(&mut self) -> Option<QueuedInput<T, H>> {
        self.entries.pop_back()
    }

    pub fn front(&self) -> Option<&QueuedInput<T, H>> {
        self.entries.front()
    }

    pub fn back(&self) -> Option<&QueuedInput<T, H>> {
        self.entries.back()
    }

    pub fn iter(&self) -> impl Iterator<Item = &QueuedInput<T, H>> {
        self.entries.iter()
    }

    pub fn drain_all(&mut self) -> Vec<QueuedInput<T, H>> {
        self.entries.drain(..).collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl<T, H: Default> InputQueue<T, H> {
    pub fn push_back(&mut self, input: T) {
        self.push_back_with_history(input, H::default());
    }

    pub fn push_front(&mut self, input: T) {
        self.push_front_with_history(input, H::default());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NextQueuedInput<T, H> {
    RejectedBatch(Vec<QueuedInput<T, H>>),
    Queued(QueuedInput<T, H>),
}

pub fn pop_next_or_rejected_batch<T, H>(
    queued: &mut InputQueue<T, H>,
    rejected: &mut InputQueue<T, H>,
) -> Option<NextQueuedInput<T, H>> {
    if !rejected.is_empty() {
        Some(NextQueuedInput::RejectedBatch(rejected.drain_all()))
    } else {
        queued.pop_front().map(NextQueuedInput::Queued)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn consecutive_plain_inputs_pop_one_at_a_time() {
        let mut queued = InputQueue::<QueuedInputAction, &'static str>::default();
        let mut rejected = InputQueue::<QueuedInputAction, &'static str>::default();
        queued.push_back_with_history(QueuedInputAction::Plain, "first");
        queued.push_back_with_history(QueuedInputAction::Plain, "second");

        assert_eq!(
            pop_next_or_rejected_batch(&mut queued, &mut rejected),
            Some(NextQueuedInput::Queued(QueuedInput::new(
                QueuedInputAction::Plain,
                "first"
            )))
        );
        assert_eq!(
            pop_next_or_rejected_batch(&mut queued, &mut rejected),
            Some(NextQueuedInput::Queued(QueuedInput::new(
                QueuedInputAction::Plain,
                "second"
            )))
        );
        assert_eq!(pop_next_or_rejected_batch(&mut queued, &mut rejected), None);
    }

    #[test]
    fn automatic_self_review_can_be_pushed_to_front() {
        let mut queued = InputQueue::<QueuedInputAction, &'static str>::default();
        let mut rejected = InputQueue::<QueuedInputAction, &'static str>::default();
        queued.push_back_with_history(QueuedInputAction::Plain, "plain");
        queued.push_front_with_history(QueuedInputAction::AutomaticSelfReview, "review");

        assert_eq!(
            pop_next_or_rejected_batch(&mut queued, &mut rejected),
            Some(NextQueuedInput::Queued(QueuedInput::new(
                QueuedInputAction::AutomaticSelfReview,
                "review"
            )))
        );
        assert_eq!(
            pop_next_or_rejected_batch(&mut queued, &mut rejected),
            Some(NextQueuedInput::Queued(QueuedInput::new(
                QueuedInputAction::Plain,
                "plain"
            )))
        );
    }

    #[test]
    fn command_actions_are_not_merged() {
        let mut queued = InputQueue::<QueuedInputAction, &'static str>::default();
        let mut rejected = InputQueue::<QueuedInputAction, &'static str>::default();
        queued.push_back_with_history(QueuedInputAction::ParseSlash, "slash");
        queued.push_back_with_history(QueuedInputAction::RunShell, "shell");

        assert_eq!(
            pop_next_or_rejected_batch(&mut queued, &mut rejected),
            Some(NextQueuedInput::Queued(QueuedInput::new(
                QueuedInputAction::ParseSlash,
                "slash"
            )))
        );
        assert_eq!(
            pop_next_or_rejected_batch(&mut queued, &mut rejected),
            Some(NextQueuedInput::Queued(QueuedInput::new(
                QueuedInputAction::RunShell,
                "shell"
            )))
        );
    }

    #[test]
    fn rejected_inputs_drain_before_queued_inputs() {
        let mut queued = InputQueue::<QueuedInputAction, &'static str>::default();
        let mut rejected = InputQueue::<QueuedInputAction, &'static str>::default();
        queued.push_back_with_history(QueuedInputAction::Plain, "queued");
        rejected.push_back_with_history(QueuedInputAction::Plain, "rejected one");
        rejected.push_back_with_history(QueuedInputAction::Plain, "rejected two");

        assert_eq!(
            pop_next_or_rejected_batch(&mut queued, &mut rejected),
            Some(NextQueuedInput::RejectedBatch(vec![
                QueuedInput::new(QueuedInputAction::Plain, "rejected one"),
                QueuedInput::new(QueuedInputAction::Plain, "rejected two"),
            ]))
        );
        assert_eq!(
            pop_next_or_rejected_batch(&mut queued, &mut rejected),
            Some(NextQueuedInput::Queued(QueuedInput::new(
                QueuedInputAction::Plain,
                "queued"
            )))
        );
    }
}
