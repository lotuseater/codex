$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-research-diff-risk-3'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
Current working-tree risk check for the self-review feature. Focus on which files are currently modified, which appear to belong to the active self-review implementation, and where concurrent worker edits may conflict. This is documentation-only, not implementation.

DO_NOT_INSPECT:
Do not run cargo/rustc/npm/build/test/deploy/schema generation. Do not edit product code. Do not stage or commit anything. Do not attribute ownership by guessing author intent beyond visible diffs and file names.

SCOUT_EVIDENCE:
Root found live external workers PID 21776, 22108, and 20496, with self-review-main.exec.log still moving. The user asked for clear documentation of findings before further split work.

WHY_AGENT / ROI:
Parallel diff-risk documentation reduces root''s integration risk while code work continues elsewhere. ROI estimate: new_agent_cost=3, parallel_gain=2, context_gain=3, repeat_gain=2, loop_followup_gain=3, risk_penalty=1, net=6.

FIRST_READS:
1. git status --short
2. git diff --stat
3. git diff -- codex-rs/core/src/review_prompts.rs
4. git diff -- codex-rs/core/src/session/handlers.rs
5. git diff -- codex-rs/app-server/src/request_processors/turn_processor.rs
6. git diff -- codex-rs/app-server/tests/suite/v2/review.rs

TOOL_HINTS:
Use git diff with explicit path arguments. Do not inspect unrelated files unless git status shows a self-review-named file needing classification.

TOKEN_TIP:
Keep the handoff compact: file, observed change type, likely conflict risk, and suggested root action.

VERIFICATION:
Verify by git status/diff only. Do not build/test.

HANDOFF:
Write .codex/workflow/agents/handoffs/self-review-research-diff-risk-3.md with sections:
- Scope
- Current modified/untracked files
- Likely self-review implementation files
- Conflict risks
- Suggested root actions
- Commands not run
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-research-diff-risk-3.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
