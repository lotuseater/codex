$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-gap-tracking-commit'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Codex Rust self-review change tracking and automatic commit scope.
DO_NOT_INSPECT: Do not edit files. Do not run cargo, rustc, build scripts, deploy scripts, broad tests, schema generation, or generated-code steps. Do not duplicate the live main implementation worker; root verified PID 21776 is alive and owns implementation.
SCOUT_EVIDENCE: Root checked live worker state and current dirty tree. Existing sidecar reports cover tracking, commit scope, prompt flow, and artifacts/journal. Main handoff was not present at launch time.
WHY_AGENT / ROI: Change tracking and commit scope are separable from prompt flow and can be researched in parallel. Positive ROI from parallel_gain=3, context_gain=3, loop_followup_gain=3, cost=3, risk=0.
FIRST_READS:
- self-review feature.md
- .codex/workflow/agents/handoffs/self-review-sidecar-tracking.md
- .codex/workflow/agents/handoffs/self-review-sidecar-commit-scope.md
- codex-rs/core/src/session.rs
- codex-rs/core/src/session/handlers.rs
- codex-rs/core/src/exec.rs
- codex-rs/tools/src/plan_tool.rs
- codex-rs/app-server/src/bespoke_event_handling.rs
TASK:
- Find where tool/file events and git commits can be tracked by code per current session/agent since last self-review.
- Document how to remember changed file paths and agent-created git commits, then forget them after review.
- Document how auto-commit should prefer committing all tracked changed files before review and all tracked changed files after review.
- Include untracked code files for these extensions: .py, .pyw, .rs, .c, .h, .cpp, .cxx, .cc, .hpp, .hh, .bat, .cmd, .ps1, .psm1, .js, .jsx, .ts, .tsx, .mjs, .cjs, .php, .java, .kt, .kts, .scala, .sc, .swift, .m, .mm, .cs, .pro, .pl.
- Flag any extension ambiguity, especially Prolog .pl versus Perl, and recommend a conservative rule if needed.
- Document how shell commands that modify already-dirty files should still be attributed to the current session when they are modified in this session, per user correction.
TOOL_HINTS: Prefer rg for exact symbols and a short PowerShell or Rust-source grep if needed. Avoid broad searches that dump thousands of lines.
TOKEN_TIP: Keep the result actionable: exact files/functions, proposed data structures, edge cases, no long source excerpts.
VERIFICATION: No build/test. Include suggested unit-level checks or simulations only.
HANDOFF: Write .codex/workflow/agents/handoffs/self-review-gap-tracking-commit.md with sections Findings, Recommended data model, Auto-commit scope, Edge cases, Suggested focused checks.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-gap-tracking-commit.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
