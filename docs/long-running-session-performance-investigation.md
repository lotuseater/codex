# Long-Running Session Performance Investigation

Date: 2026-05-25

## Scope

Investigated reports that Codex sessions degrade after hours even when CPU, memory,
and system load are low. The user observed that non-interactive sessions degrade
less, while interactive sessions can visually freeze.

This investigation intentionally avoided builds and tests. Only static reading,
diff review, and external non-interactive worker review were used before fixes.

## Sources Inspected

- Current dirty diff for `codex-rs/tui/src/tui/frame_requester.rs`.
- Static reads of TUI frame scheduling call sites under `codex-rs/tui/src`.
- External worker handoff:
  `.codex/workflow/agents/long_session_perf_tui_review.handoff.md`.
- External worker logs for the core/session and app-server/MCP reviews:
  `.codex/workflow/agents/long_session_perf_core_review.exec.visible.log` and
  `.codex/workflow/agents/long_session_perf_app_mcp_review.exec.visible.log`.

## Findings

### High confidence: TUI frame request backlog

The strongest explanation is the TUI frame requester path. The previous scheduler
shape accepted every frame request through an unbounded channel. Coalescing happened
only after the scheduler dequeued requests, so bursts from widgets or background
tasks could accumulate a large pending queue even though rate limiting still capped
actual draws. That matches the user-visible symptom: the process can look mostly
idle while the interactive UI stops responding or visually freezes.

The current working tree already contains an incoming fix that replaces the
unbounded frame-request channel with shared pending-deadline state and `Notify`.
That is the right direction because it bounds pending work to one deadline instead
of one queued item per request.

### Medium confidence: shutdown and delayed-deadline edges

Static review of the incoming frame requester change found two edges to harden:

- Scheduler lifetime should not depend only on `Arc::strong_count`, because the
  final requester can be dropped while a delayed frame is pending.
- Delayed scheduling should wake the scheduler when a new request moves the
  deadline earlier, while later duplicate requests should remain coalesced.

These are bounded TUI-local fixes and are worth completing before final
verification.

### Inconclusive: core/session and app-server/MCP surfaces

Two external non-interactive workers reviewed core/session and app-server/MCP
surfaces in parallel. After four 5-minute polling windows, neither produced a
handoff. Their logs showed targeted static reading but no completed high-confidence
finding. To avoid speculative changes, this pass treats those surfaces as
inconclusive and leaves them as follow-up investigation areas rather than patching
them now.

## Recommendation

Finish the TUI frame requester fix and add focused canary coverage for burst
coalescing, earlier-deadline wakeup, later-deadline coalescing, and shutdown after
the last requester is dropped. Then run formatting, focused TUI tests, and the
local build/deploy pipeline at the end of the task.
