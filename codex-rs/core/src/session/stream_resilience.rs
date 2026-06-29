//! Bounded extended auto-retry for transient stream disconnects.
//!
//! This is a decoupled, fork-owned sibling of [`super::usage_hint_reminder`].
//! It exists to recover a turn that died from a transient transport disconnect
//! (`CodexErr::Stream(..)`, e.g. "stream disconnected before completion: stream
//! closed before response.completed") AFTER the built-in per-stream retry budget
//! (`stream_max_retries`, default 5, ~6s total) has already been exhausted by
//! [`crate::responses_retry::handle_retryable_response_stream_error`].
//!
//! Today, once those fast retries are spent the turn-loop `Err(e)` arm emits an
//! error event and goes idle, so a disconnect lasting more than ~6 seconds
//! permanently loses the turn even after connectivity returns. This module adds
//! a longer-horizon, BOUNDED wait-and-retry: it waits a configurable interval
//! (default 60s, up to a configurable number of attempts, default 3) and tells
//! the caller to re-enter the turn loop. The re-entry rebuilds the prompt from
//! `sess.clone_history()`, which still contains the original user message
//! (recorded before the first sampling attempt), so the goal is re-sent cleanly
//! with no re-injection and no duplicated user input.
//!
//! All policy lives here; the `turn.rs` hook is the smallest possible call. The
//! wait is cancellation-aware so a deliberate user interrupt during the wait
//! aborts immediately and preserves today's give-up behavior. The give-up path
//! (any condition below not met) returns `false`, leaving the existing
//! error-emit-and-break behavior exactly as it was.

use std::time::Duration;

use codex_protocol::error::CodexErr;
use tokio_util::sync::CancellationToken;

use super::session::Session;
use super::turn_context::TurnContext;
use crate::config::StreamResilienceConfig;

/// Decide whether a failed turn should wait and retry once more after the
/// built-in stream retries are exhausted, and if so perform the user-facing
/// notice + the cancellation-aware wait.
///
/// Returns `true` only when ALL of the following hold, in which case the caller
/// should `continue` the turn loop (re-sample from history):
/// * `err` is a transient stream disconnect (`CodexErr::Stream(..)`) -- never a
///   quota/auth/context/abort error;
/// * the fork's extended auto-retry is enabled (`auto_retry_enabled`);
/// * the cancellation token has not already been tripped (no deliberate stop);
/// * the per-submission extended-retry budget (`max_extended_waits`) is not yet
///   spent.
///
/// On the `true` path it increments `extended_retries`, surfaces a
/// `Connection lost - auto-retrying ...` notice via the same
/// [`Session::notify_stream_error`] channel the built-in retry uses, then waits
/// `extended_wait_secs` while watching `cancellation_token`. If the user stops
/// during the wait it returns `false` (clean abort). Any other case also
/// returns `false`, so the caller falls through to today's
/// emit-error-and-break behavior unchanged.
pub(crate) async fn maybe_continue_after_disconnect(
    err: &CodexErr,
    config: &StreamResilienceConfig,
    sess: &Session,
    turn_context: &TurnContext,
    cancellation_token: &CancellationToken,
    extended_retries: &mut u64,
) -> bool {
    // Only transient stream disconnects are eligible. A user interrupt surfaces
    // as `CodexErr::TurnAborted` (handled by an earlier match arm and returned
    // before this hook is reached), so auto-retry never fights a deliberate
    // stop. Quota/auth/context errors are not `Stream` and are left to fail.
    if !matches!(err, CodexErr::Stream(..)) {
        return false;
    }

    if !config.auto_retry_enabled {
        return false;
    }

    // If a stop was already requested, do not start another wait.
    if cancellation_token.is_cancelled() {
        return false;
    }

    if *extended_retries >= config.max_extended_waits {
        return false;
    }

    *extended_retries += 1;
    let attempt = *extended_retries;
    let max = config.max_extended_waits;
    let wait_secs = config.extended_wait_secs;

    // Surface the wait through the same stream-error channel the per-attempt
    // built-in retry uses ("Reconnecting... N/M"), so the front-end shows a
    // recovery notice instead of a frozen screen. `CodexErr` is not `Clone`, so
    // rebuild an equivalent `Stream` error for the event's detail field.
    let detail = match err {
        CodexErr::Stream(message, _) => message.clone(),
        other => other.to_string(),
    };
    sess.notify_stream_error(
        turn_context,
        format!("Connection lost - auto-retrying in {wait_secs}s (attempt {attempt}/{max})"),
        CodexErr::Stream(detail, None),
    )
    .await;

    // Cancellation-aware wait: a deliberate user stop during the wait aborts the
    // retry cleanly. Completing the sleep means "retry"; cancellation means
    // "give up" (preserving today's behavior).
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(wait_secs)) => true,
        _ = cancellation_token.cancelled() => false,
    }
}
