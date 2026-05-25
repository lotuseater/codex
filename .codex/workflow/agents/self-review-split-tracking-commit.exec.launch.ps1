$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-split-tracking-commit'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: per-agent changed-file/commit tracking and automatic commit scope before/after self-review.
DO_NOT_INSPECT: Do not build, test, generate schemas, deploy, commit, or edit source files. Only write your handoff file. Avoid broad searches unless a symbol is missing.
SCOUT_EVIDENCE: Root confirmed no live internal agents and old marker PIDs 23384/33488 are not running. User specifically corrected the plan: include all files modified by the current session, and self-review should prefer committing all changed files before review and all changed files after review, including untracked code files.
WHY_AGENT / ROI: Tracking and commit scope is separable from prompt flow; parallel research reduces root context. Agent ROI Estimate: new_agent_cost=3, parallel_gain=3, context_gain=3, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=7.
FIRST_READS:
- `self-review feature.md`
- latest handoffs matching `self-review-research-tracking-commit*.md` and `self-review-sidecar-commit-scope.md` if present
- `codex-rs/app-server/src/bespoke_event_handling.rs`
- `codex-rs/app-server/src/request_processors/turn_processor.rs`
- `codex-rs/app-server/tests/suite/v2/review.rs`
- `rg "git|commit|changed|dirty|untracked|session|Review" codex-rs/app-server codex-rs/app-server-protocol -g ''*.rs''`
TOOL_HINTS: Use focused `rg`; do not dump large diffs. Include concrete language extensions for untracked code files: Python, Rust, C/C++, bat, ps1, JavaScript/TypeScript, PHP, Java, Kotlin, Scala, Swift, Objective-C, C#, Prolog.
TOKEN_TIP: Prefer a small implementation sketch and edge cases over exhaustive source copying.
TASK: Determine where to track files changed by the current agent/session since last review, track git commits by the agent/session, clear those remembered paths/commits after review, and implement auto-commit selection by code rather than LLM. Include how shell-command file modifications should be attributed to current session. Include suggested untracked code extension list.
VERIFICATION: No tests/builds. Reason from source and test patterns only.
HANDOFF: Write `.codex/workflow/agents/handoffs/self-review-split-tracking-commit.md` with sections: Existing Mechanism, Required State, Commit Scope Rules, Untracked Extensions, Code Edit Targets, Tests Later, Commands Run. Keep under 140 lines.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-split-tracking-commit.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
