$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-min-tracking-commit'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: missing narrow handoff for self-review changed-file/commit tracking and auto-commit scope.
DO_NOT_INSPECT: Do not build, test, generate schemas, deploy, commit, or edit source files. Do not run broad `rg` or broad `git diff`. Only write your handoff file.
SCOUT_EVIDENCE: Root has three handoffs already: `.codex/workflow/agents/handoffs/self-review-retry-current-diff.md`, `.codex/workflow/agents/handoffs/self-review-split-event-flow.md`, `.codex/workflow/agents/handoffs/self-review-split-reflection-artifacts.md`. The earlier tracking worker is alive but has no handoff and shows command timeouts, so this is a narrow replacement.
WHY_AGENT / ROI: This fills only the missing tracking/commit gap while root keeps integration context. Agent ROI Estimate: new_agent_cost=3, parallel_gain=2, context_gain=3, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=6.
FIRST_READS ONLY:
- `self-review feature.md`
- `.codex/workflow/agents/handoffs/self-review-retry-current-diff.md`
- `.codex/workflow/agents/handoffs/self-review-split-event-flow.md`
- `.codex/workflow/agents/handoffs/self-review-split-reflection-artifacts.md`
- `.codex/workflow/agents/handoffs/self-review-sidecar-commit-scope.md` if present
- `.codex/workflow/agents/handoffs/self-review-research-tracking-commit-2.md` if present
- `codex-rs/app-server/src/bespoke_event_handling.rs`
- `codex-rs/app-server/src/request_processors/turn_processor.rs`
- `codex-rs/app-server/tests/suite/v2/review.rs`
TOOL_HINTS: Use `Select-String` on the exact named files only if needed. Keep shell timeouts low. Avoid broad repo scans.
TOKEN_TIP: Stop as soon as the implementation sketch is actionable.
TASK: Provide the missing compact tracking/commit handoff. Cover: per-current-agent/session file tracking since last review; commit tracking; clearing remembered paths/commits after review; shell-command modifications should count as current-session modifications; auto-commit before review for all current-session changed files; auto-commit after review for all changed files after review; include untracked code files by extension: `.py`, `.rs`, `.c`, `.h`, `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hh`, `.bat`, `.cmd`, `.ps1`, `.psm1`, `.js`, `.jsx`, `.ts`, `.tsx`, `.mjs`, `.cjs`, `.php`, `.java`, `.kt`, `.kts`, `.scala`, `.sc`, `.swift`, `.m`, `.mm`, `.cs`, `.pl`, `.pro`, `.prolog`.
VERIFICATION: No tests/builds. Reason from source and existing tests only.
HANDOFF: Write `.codex/workflow/agents/handoffs/self-review-min-tracking-commit.md` with sections: Required State, Attribution Rules, Commit Scope Rules, Untracked Extensions, Code Edit Targets, Test Ideas, Commands Run. Keep under 110 lines.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-min-tracking-commit.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
