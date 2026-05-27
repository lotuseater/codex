$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: wave6_frame_requester_fix'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', '# External Worker: wave6_frame_requester_fix

You are running as an external non-interactive Codex worker. The root session is only overseeing. Do not spawn subagents.

CONTEXT_AREA:
- Repo: C:/Users/Oleh/Documents/GitHub/open_ai/codex
- Branch: slow-context-budget-mode
- Task: integrate the reviewed fix for long-running Codex session TUI performance degradation/freezes.
- Current known blocker from prior handoff: .codex/workflow/agents/long_session_perf_final_static_review.handoff.md says current HEAD must not deploy because codex-rs/tui/src/tui/frame_requester.rs has a pending-deadline race / request coalescing issue.

OWNERSHIP:
- You own edits only in codex-rs/tui/src/tui/frame_requester.rs and narrowly related in-file tests if present.
- You may update your own handoff file: .codex/workflow/agents/wave6_frame_requester_fix.handoff.md
- You are not alone in the codebase. Do not revert or rewrite changes outside your ownership. Do not touch docs, build scripts, or unrelated source files.

DO_NOT_INSPECT:
- Do not do broad repo sweeps. Do not inspect generated/vendor/cache directories. Do not run broad `rg` over the whole repo unless a first read proves it is necessary.

SCOUT_EVIDENCE:
- Root inspected git status and prior handoffs. The relevant prior review handoffs are long_session_perf_final_static_review.handoff.md and long_session_perf_tui_review.handoff.md.

WHY_AGENT / ROI:
- Positive ROI: this is the current deployment blocker and requires focused code inspection/editing while the interactive root avoids long-session freeze risk.

FIRST_READS:
1. .codex/workflow/agents/long_session_perf_final_static_review.handoff.md
2. .codex/workflow/agents/long_session_perf_tui_review.handoff.md
3. codex-rs/tui/src/tui/frame_requester.rs
4. Any tests inside that same file only, if present.

TOOL_HINTS:
- Use focused reads and `git diff -- codex-rs/tui/src/tui/frame_requester.rs`.
- Use apply_patch for manual edits.
- Do not run cargo build, cargo test, rustc, npm, deploy scripts, or broad test/build commands. This stage is code inspection and editing only.

IMPLEMENTATION TARGET:
- Fix the pending-deadline/request coalescing race identified by the final static review.
- Preserve low-overhead frame wake behavior for long sessions.
- Prefer a small, robust state machine over ad hoc timing logic.
- Ensure shutdown/lifetime behavior remains sane and no render request can be permanently stranded behind a stale pending deadline.

VERIFICATION:
- Code inspection only in this worker.
- Include the exact commands you did and did not run in the handoff.

HANDOFF:
- Write .codex/workflow/agents/wave6_frame_requester_fix.handoff.md with: summary, files changed, reasoning for the fix, residual risks, and exact final `git diff -- codex-rs/tui/src/tui/frame_requester.rs` summary.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\wave6_frame_requester_fix.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
