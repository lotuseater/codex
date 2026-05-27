$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: wave7_frame_requester_fix_review'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', '# External Worker: wave7_frame_requester_fix_review

You are running as an external non-interactive Codex worker. The root session is only overseeing. Do not spawn subagents.

CONTEXT_AREA:
- Repo: C:/Users/Oleh/Documents/GitHub/open_ai/codex
- Branch: slow-context-budget-mode
- There is an active merge from main. Do not resolve it, do not touch unmerged files, and do not commit.
- Feature under review: codex-rs/tui/src/tui/frame_requester.rs fix made by wave6_frame_requester_fix.

OWNERSHIP:
- Read-only review. Write only .codex/workflow/agents/wave7_frame_requester_fix_review.handoff.md
- Do not modify source files.

DO_NOT_INSPECT:
- No broad repo search. No build/test/deploy. Do not inspect merge conflicts except to note that merge state blocks final build/deploy.

SCOUT_EVIDENCE:
- Prior static review found a blocking pending-deadline/request coalescing race in codex-rs/tui/src/tui/frame_requester.rs.
- wave6_frame_requester_fix added about 96 lines and 1 deletion in that file.

WHY_AGENT / ROI:
- Positive ROI: user requested delegated inspection before integration, and root interactive session should stay lightweight.

FIRST_READS:
1. .codex/workflow/agents/long_session_perf_final_static_review.handoff.md
2. .codex/workflow/agents/wave6_frame_requester_fix.handoff.md
3. git diff -- codex-rs/tui/src/tui/frame_requester.rs
4. codex-rs/tui/src/tui/frame_requester.rs, only relevant sections/tests

TOOL_HINTS:
- Use focused git diff/read commands only.
- It is okay to use `git diff --check -- codex-rs/tui/src/tui/frame_requester.rs` if desired; do not run broader checks.

REVIEW TARGET:
- Determine whether the frame requester race is actually fixed by inspection.
- Look for deadlocks, missed wakeups, leaked scheduler tasks, broken shutdown semantics, or test expectations that do not match implementation.
- Classify findings as blocking/non-blocking. If clean, say clean.

VERIFICATION:
- Code inspection only; no builds/tests.

HANDOFF:
- Write .codex/workflow/agents/wave7_frame_requester_fix_review.handoff.md with verdict, findings, exact commands run, and whether root can treat this feature fix as integrated pending final build/test after merge resolution.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\wave7_frame_requester_fix_review.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
