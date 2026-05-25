//! Frame draw scheduling utilities for the TUI.
//!
//! This module exposes [`FrameRequester`], a lightweight handle that widgets and
//! background tasks can clone to request future redraws of the TUI.
//!
//! Internally it spawns a [`FrameScheduler`] task that coalesces many requests
//! into a single notification on a broadcast channel used by the main TUI event
//! loop. This keeps animations and status updates smooth without redrawing more
//! often than necessary.
//!
//! This follows the actor-style design from
//! [“Actors with Tokio”](https://ryhl.io/blog/actors-with-tokio/), with a
//! dedicated scheduler task and lightweight request handles.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use tokio::sync::Notify;
use tokio::sync::broadcast;

use super::frame_rate_limiter::FrameRateLimiter;

/// A requester for scheduling future frame draws on the TUI event loop.
///
/// This is the handler side of an actor/handler pair with `FrameScheduler`, which coalesces
/// multiple frame requests into a single draw operation.
///
/// Clones of this type can be freely shared across tasks to make it possible to trigger frame draws
/// from anywhere in the TUI code.
#[derive(Debug)]
pub struct FrameRequester {
    schedule_state: Arc<FrameScheduleState>,
}

impl Clone for FrameRequester {
    fn clone(&self) -> Self {
        self.schedule_state.add_requester();
        Self {
            schedule_state: Arc::clone(&self.schedule_state),
        }
    }
}

impl FrameRequester {
    /// Create a new FrameRequester and spawn its associated FrameScheduler task.
    ///
    /// The provided `draw_tx` is used to notify the TUI event loop of scheduled draws.
    pub fn new(draw_tx: broadcast::Sender<()>) -> Self {
        let schedule_state = Arc::new(FrameScheduleState::default());
        let scheduler = FrameScheduler::new(Arc::clone(&schedule_state), draw_tx);
        tokio::spawn(scheduler.run());
        Self { schedule_state }
    }

    /// Schedule a frame draw as soon as possible.
    pub fn schedule_frame(&self) {
        self.schedule_state.request_frame(Instant::now());
    }

    /// Schedule a frame draw to occur after the specified duration.
    pub fn schedule_frame_in(&self, dur: Duration) {
        self.schedule_state.request_frame(Instant::now() + dur);
    }
}

impl Drop for FrameRequester {
    fn drop(&mut self) {
        if self.schedule_state.release_requester() {
            self.schedule_state.notify_scheduler();
        }
    }
}

#[cfg(test)]
impl FrameRequester {
    /// Create a no-op frame requester for tests.
    pub(crate) fn test_dummy() -> Self {
        FrameRequester {
            schedule_state: Arc::new(FrameScheduleState::default()),
        }
    }

    fn pending_frame_deadline(&self) -> Option<Instant> {
        self.schedule_state.pending_deadline()
    }
}

/// A scheduler for coalescing frame draw requests and notifying the TUI event loop.
///
/// This type is internal to `FrameRequester` and is spawned as a task to handle scheduling logic.
///
/// To avoid wasted redraw work, draw notifications are clamped to a maximum of 120 FPS (see
/// [`FrameRateLimiter`]).
#[derive(Debug)]
struct FrameScheduleState {
    deadlines: Mutex<FrameDeadlineState>,
    requester_count: AtomicUsize,
    notify: Notify,
}

#[derive(Debug, Default)]
struct FrameDeadlineState {
    pending_deadline: Option<Instant>,
    in_flight_deadline: Option<InFlightDeadline>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InFlightDeadline {
    Waiting(Instant),
    Emitting(Instant),
}

impl InFlightDeadline {
    fn target(self) -> Instant {
        match self {
            Self::Waiting(target) | Self::Emitting(target) => target,
        }
    }
}

impl Default for FrameScheduleState {
    fn default() -> Self {
        Self {
            deadlines: Mutex::new(FrameDeadlineState::default()),
            requester_count: AtomicUsize::new(1),
            notify: Notify::new(),
        }
    }
}

impl FrameScheduleState {
    fn add_requester(&self) {
        self.requester_count.fetch_add(1, Ordering::Relaxed);
    }

    fn release_requester(&self) -> bool {
        self.requester_count.fetch_sub(1, Ordering::AcqRel) == 1
    }

    fn requesters_are_dropped(&self) -> bool {
        self.requester_count.load(Ordering::Acquire) == 0
    }

    fn request_frame(&self, draw_at: Instant) {
        let now = Instant::now();
        let mut deadlines = self.deadlines.lock().unwrap_or_else(|e| e.into_inner());
        let should_notify = deadlines.request_frame(draw_at, now);
        drop(deadlines);

        if should_notify {
            self.notify_scheduler();
        }
    }

    fn pending_deadline(&self) -> Option<Instant> {
        self.deadlines
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .next_deadline()
    }

    fn take_pending_deadline(&self) -> Option<Instant> {
        self.deadlines
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pending_deadline
            .take()
    }

    fn set_in_flight_waiting(&self, target: Instant) {
        self.deadlines
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .in_flight_deadline = Some(InFlightDeadline::Waiting(target));
    }

    fn mark_in_flight_emitting(&self, target: Instant) {
        let mut deadlines = self.deadlines.lock().unwrap_or_else(|e| e.into_inner());
        if deadlines.in_flight_deadline == Some(InFlightDeadline::Waiting(target)) {
            deadlines.in_flight_deadline = Some(InFlightDeadline::Emitting(target));
        }
    }

    fn clear_in_flight_deadline(&self, target: Instant) {
        let mut deadlines = self.deadlines.lock().unwrap_or_else(|e| e.into_inner());
        if deadlines
            .in_flight_deadline
            .is_some_and(|deadline| deadline.target() == target)
        {
            deadlines.in_flight_deadline = None;
        }
    }

    fn has_pending_deadline_before(&self, target: Instant) -> bool {
        self.deadlines
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pending_deadline
            .is_some_and(|deadline| deadline < target)
    }

    fn notify_scheduler(&self) {
        self.notify.notify_one();
    }

    async fn notified(&self) {
        self.notify.notified().await;
    }
}

impl FrameDeadlineState {
    fn request_frame(&mut self, draw_at: Instant, now: Instant) -> bool {
        match self.in_flight_deadline {
            Some(InFlightDeadline::Waiting(target)) => {
                if draw_at < target || (now >= target && draw_at > now) {
                    self.store_pending_deadline(draw_at, Some(target))
                } else {
                    false
                }
            }
            Some(InFlightDeadline::Emitting(_target)) => {
                if draw_at > now {
                    self.store_pending_deadline(draw_at, None)
                } else {
                    false
                }
            }
            None => self.store_pending_deadline(draw_at, None),
        }
    }

    fn store_pending_deadline(
        &mut self,
        draw_at: Instant,
        waiting_target: Option<Instant>,
    ) -> bool {
        let should_notify = self
            .pending_deadline
            .is_none_or(|current| draw_at < current)
            && waiting_target.is_none_or(|target| draw_at < target);
        self.pending_deadline = Some(
            self.pending_deadline
                .map_or(draw_at, |current| current.min(draw_at)),
        );
        should_notify
    }

    fn next_deadline(&self) -> Option<Instant> {
        let in_flight_deadline = self.in_flight_deadline.map(InFlightDeadline::target);
        match (self.pending_deadline, in_flight_deadline) {
            (Some(pending), Some(in_flight)) => Some(pending.min(in_flight)),
            (Some(pending), None) => Some(pending),
            (None, Some(in_flight)) => Some(in_flight),
            (None, None) => None,
        }
    }
}

struct FrameScheduler {
    schedule_state: Arc<FrameScheduleState>,
    draw_tx: broadcast::Sender<()>,
    rate_limiter: FrameRateLimiter,
}

enum FrameScheduleEvent {
    DeadlineElapsed,
    Rescheduled,
}

impl FrameScheduler {
    /// Create a new FrameScheduler with shared schedule state and a draw notification sender.
    fn new(schedule_state: Arc<FrameScheduleState>, draw_tx: broadcast::Sender<()>) -> Self {
        Self {
            schedule_state,
            draw_tx,
            rate_limiter: FrameRateLimiter::default(),
        }
    }

    /// Run the scheduling loop, coalescing frame requests and notifying the TUI event loop.
    ///
    /// This method runs indefinitely until all requesters are dropped. A single draw notification
    /// is sent for multiple requests scheduled before the next draw deadline.
    async fn run(mut self) {
        loop {
            let Some(requested_deadline) = self.schedule_state.take_pending_deadline() else {
                if self.requesters_are_dropped() {
                    break;
                }
                self.schedule_state.notified().await;
                continue;
            };

            if self.requesters_are_dropped() {
                break;
            }

            let target = self.rate_limiter.clamp_deadline(requested_deadline);
            self.schedule_state.set_in_flight_waiting(target);
            match self.wait_for_deadline_or_reschedule(target).await {
                FrameScheduleEvent::Rescheduled => {
                    self.schedule_state.clear_in_flight_deadline(target);
                    if self.requesters_are_dropped() {
                        break;
                    }
                    // A newly earlier deadline may have arrived; recompute the
                    // sleep target before sending a draw.
                    continue;
                }
                FrameScheduleEvent::DeadlineElapsed => {
                    if self.requesters_are_dropped() {
                        break;
                    }
                    self.schedule_state.mark_in_flight_emitting(target);
                    self.emit_draw(target);

                    if self.requesters_are_dropped() {
                        break;
                    }
                }
            }
        }
    }

    async fn wait_for_deadline_or_reschedule(&self, target: Instant) -> FrameScheduleEvent {
        let deadline = tokio::time::sleep_until(target.into());
        tokio::pin!(deadline);

        loop {
            tokio::select! {
                _ = self.schedule_state.notified() => {
                    if self.requesters_are_dropped()
                        || self.schedule_state.has_pending_deadline_before(target)
                    {
                        break FrameScheduleEvent::Rescheduled;
                    }
                }
                _ = &mut deadline => break FrameScheduleEvent::DeadlineElapsed,
            }
        }
    }

    fn emit_draw(&mut self, target: Instant) {
        self.rate_limiter.mark_emitted(target);
        let _ = self.draw_tx.send(());
        self.schedule_state.clear_in_flight_deadline(target);
    }

    fn requesters_are_dropped(&self) -> bool {
        self.schedule_state.requesters_are_dropped()
    }
}
#[cfg(test)]
mod tests {
    use super::super::frame_rate_limiter::MIN_FRAME_INTERVAL;
    use super::*;
    use tokio::time;
    use tokio_util::time::FutureExt;

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn test_schedule_frame_immediate_triggers_once() {
        let (draw_tx, mut draw_rx) = broadcast::channel(16);
        let requester = FrameRequester::new(draw_tx);

        requester.schedule_frame();

        // Advance time minimally to let the scheduler process and hit the deadline == now.
        time::advance(Duration::from_millis(1)).await;

        // First draw should arrive.
        let first = draw_rx
            .recv()
            .timeout(Duration::from_millis(50))
            .await
            .expect("timed out waiting for first draw");
        assert!(first.is_ok(), "broadcast closed unexpectedly");

        // No second draw should arrive.
        let second = draw_rx.recv().timeout(Duration::from_millis(20)).await;
        assert!(second.is_err(), "unexpected extra draw received");
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn test_schedule_frame_in_triggers_at_delay() {
        let (draw_tx, mut draw_rx) = broadcast::channel(16);
        let requester = FrameRequester::new(draw_tx);

        requester.schedule_frame_in(Duration::from_millis(50));

        // Advance less than the delay: no draw yet.
        time::advance(Duration::from_millis(30)).await;
        let early = draw_rx.recv().timeout(Duration::from_millis(10)).await;
        assert!(early.is_err(), "draw fired too early");

        // Advance past the deadline: one draw should fire.
        time::advance(Duration::from_millis(25)).await;
        let first = draw_rx
            .recv()
            .timeout(Duration::from_millis(50))
            .await
            .expect("timed out waiting for scheduled draw");
        assert!(first.is_ok(), "broadcast closed unexpectedly");

        // No second draw should arrive.
        let second = draw_rx.recv().timeout(Duration::from_millis(20)).await;
        assert!(second.is_err(), "unexpected extra draw received");
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn test_delayed_frame_is_not_emitted_after_last_requester_drops() {
        let (draw_tx, mut draw_rx) = broadcast::channel(16);
        let requester = FrameRequester::new(draw_tx);

        requester.schedule_frame_in(Duration::from_millis(100));
        drop(requester);

        time::advance(Duration::from_millis(1)).await;
        let shutdown = draw_rx.recv().timeout(Duration::from_millis(10)).await;
        assert!(
            matches!(shutdown, Ok(Err(broadcast::error::RecvError::Closed))),
            "scheduler should exit without emitting the delayed draw"
        );
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn test_coalesces_multiple_requests_into_single_draw() {
        let (draw_tx, mut draw_rx) = broadcast::channel(16);
        let requester = FrameRequester::new(draw_tx);

        // Schedule multiple immediate requests close together.
        requester.schedule_frame();
        requester.schedule_frame();
        requester.schedule_frame();

        // Allow the scheduler to process and hit the coalesced deadline.
        time::advance(Duration::from_millis(1)).await;

        // Expect only a single draw notification despite three requests.
        let first = draw_rx
            .recv()
            .timeout(Duration::from_millis(50))
            .await
            .expect("timed out waiting for coalesced draw");
        assert!(first.is_ok(), "broadcast closed unexpectedly");

        // No additional draw should be sent for the same coalesced batch.
        let second = draw_rx.recv().timeout(Duration::from_millis(20)).await;
        assert!(second.is_err(), "unexpected extra draw received");
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn test_coalesces_mixed_immediate_and_delayed_requests() {
        let (draw_tx, mut draw_rx) = broadcast::channel(16);
        let requester = FrameRequester::new(draw_tx);

        // Schedule a delayed draw and then an immediate one; should coalesce and fire at the earliest (immediate).
        requester.schedule_frame_in(Duration::from_millis(100));
        requester.schedule_frame();

        time::advance(Duration::from_millis(1)).await;

        let first = draw_rx
            .recv()
            .timeout(Duration::from_millis(50))
            .await
            .expect("timed out waiting for coalesced immediate draw");
        assert!(first.is_ok(), "broadcast closed unexpectedly");

        // The later delayed request should have been coalesced into the earlier one; no second draw.
        let second = draw_rx.recv().timeout(Duration::from_millis(120)).await;
        assert!(second.is_err(), "unexpected extra draw received");
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn test_limits_draw_notifications_to_120fps() {
        let (draw_tx, mut draw_rx) = broadcast::channel(16);
        let requester = FrameRequester::new(draw_tx);

        requester.schedule_frame();
        time::advance(Duration::from_millis(1)).await;
        let first = draw_rx
            .recv()
            .timeout(Duration::from_millis(50))
            .await
            .expect("timed out waiting for first draw");
        assert!(first.is_ok(), "broadcast closed unexpectedly");

        requester.schedule_frame();
        time::advance(Duration::from_millis(1)).await;
        let early = draw_rx.recv().timeout(Duration::from_millis(1)).await;
        assert!(
            early.is_err(),
            "draw fired too early; expected max 120fps (min interval {MIN_FRAME_INTERVAL:?})"
        );

        time::advance(MIN_FRAME_INTERVAL).await;
        let second = draw_rx
            .recv()
            .timeout(Duration::from_millis(50))
            .await
            .expect("timed out waiting for second draw");
        assert!(second.is_ok(), "broadcast closed unexpectedly");
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn test_rate_limit_clamps_early_delayed_requests() {
        let (draw_tx, mut draw_rx) = broadcast::channel(16);
        let requester = FrameRequester::new(draw_tx);

        requester.schedule_frame();
        time::advance(Duration::from_millis(1)).await;
        let first = draw_rx
            .recv()
            .timeout(Duration::from_millis(50))
            .await
            .expect("timed out waiting for first draw");
        assert!(first.is_ok(), "broadcast closed unexpectedly");

        requester.schedule_frame_in(Duration::from_millis(1));

        time::advance(MIN_FRAME_INTERVAL / 2).await;
        let too_early = draw_rx.recv().timeout(Duration::from_millis(1)).await;
        assert!(
            too_early.is_err(),
            "draw fired too early; expected max 120fps (min interval {MIN_FRAME_INTERVAL:?})"
        );

        time::advance(MIN_FRAME_INTERVAL).await;
        let second = draw_rx
            .recv()
            .timeout(Duration::from_millis(50))
            .await
            .expect("timed out waiting for clamped draw");
        assert!(second.is_ok(), "broadcast closed unexpectedly");
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn test_rate_limit_does_not_delay_future_draws() {
        let (draw_tx, mut draw_rx) = broadcast::channel(16);
        let requester = FrameRequester::new(draw_tx);

        requester.schedule_frame();
        time::advance(Duration::from_millis(1)).await;
        let first = draw_rx
            .recv()
            .timeout(Duration::from_millis(50))
            .await
            .expect("timed out waiting for first draw");
        assert!(first.is_ok(), "broadcast closed unexpectedly");

        requester.schedule_frame_in(Duration::from_millis(50));

        time::advance(Duration::from_millis(49)).await;
        let early = draw_rx.recv().timeout(Duration::from_millis(1)).await;
        assert!(early.is_err(), "draw fired too early");

        time::advance(Duration::from_millis(1)).await;
        let second = draw_rx
            .recv()
            .timeout(Duration::from_millis(50))
            .await
            .expect("timed out waiting for delayed draw");
        assert!(second.is_ok(), "broadcast closed unexpectedly");
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn test_multiple_delayed_requests_coalesce_to_earliest() {
        let (draw_tx, mut draw_rx) = broadcast::channel(16);
        let requester = FrameRequester::new(draw_tx);

        // Schedule multiple delayed draws; they should coalesce to the earliest (10ms).
        requester.schedule_frame_in(Duration::from_millis(100));
        requester.schedule_frame_in(Duration::from_millis(20));
        requester.schedule_frame_in(Duration::from_millis(120));

        // Advance to just before the earliest deadline: no draw yet.
        time::advance(Duration::from_millis(10)).await;
        let early = draw_rx.recv().timeout(Duration::from_millis(10)).await;
        assert!(early.is_err(), "draw fired too early");

        // Advance past the earliest deadline: one draw should fire.
        time::advance(Duration::from_millis(20)).await;
        let first = draw_rx
            .recv()
            .timeout(Duration::from_millis(50))
            .await
            .expect("timed out waiting for earliest coalesced draw");
        assert!(first.is_ok(), "broadcast closed unexpectedly");

        // No additional draw should fire for the later delayed requests.
        let second = draw_rx.recv().timeout(Duration::from_millis(120)).await;
        assert!(second.is_err(), "unexpected extra draw received");
    }

    #[tokio::test]
    async fn test_burst_frame_requests_use_single_pending_deadline() {
        time::pause();
        let (draw_tx, mut draw_rx) = broadcast::channel(16);
        let requester = FrameRequester::new(draw_tx);

        for _ in 0..100_000 {
            requester.schedule_frame();
        }

        assert!(
            requester.pending_frame_deadline().is_some(),
            "burst should leave one pending deadline"
        );

        time::advance(MIN_FRAME_INTERVAL).await;
        let first = draw_rx
            .recv()
            .timeout(Duration::from_millis(50))
            .await
            .expect("timed out waiting for coalesced draw");
        assert!(first.is_ok(), "broadcast closed unexpectedly");
        assert!(
            requester.pending_frame_deadline().is_none(),
            "coalesced draw should clear the pending deadline"
        );

        let second = draw_rx.recv().timeout(Duration::from_millis(20)).await;
        assert!(second.is_err(), "unexpected extra draw received");
    }

    #[test]
    fn test_delayed_request_after_elapsed_target_stays_pending() {
        let state = FrameScheduleState::default();
        let target = Instant::now() - Duration::from_millis(1);
        let delayed_deadline = Instant::now() + Duration::from_millis(100);

        state.set_in_flight_waiting(target);
        state.request_frame(delayed_deadline);

        assert_eq!(
            state
                .deadlines
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .pending_deadline,
            Some(delayed_deadline),
            "future delayed request during the emit window must survive the current draw"
        );

        state.mark_in_flight_emitting(target);
        state.clear_in_flight_deadline(target);
        assert_eq!(
            state.pending_deadline(),
            Some(delayed_deadline),
            "clearing the emitted draw must not clear a later pending request"
        );
    }

    #[test]
    fn test_later_request_before_waiting_target_coalesces() {
        let state = FrameScheduleState::default();
        let target = Instant::now() + Duration::from_millis(100);
        let later_deadline = target + Duration::from_millis(100);

        state.set_in_flight_waiting(target);
        state.request_frame(later_deadline);

        assert_eq!(
            state
                .deadlines
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .pending_deadline,
            None,
            "later requests made before the waiting target should be covered by that draw"
        );
    }
}
