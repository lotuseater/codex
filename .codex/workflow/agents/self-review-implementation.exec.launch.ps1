$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-implementation'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA:
Implement the self-review feature improvements described in `self-review feature.md` and the user''s latest additions. Repo root: `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

DO_NOT_INSPECT:
Do not do broad unrelated repo sweeps. Do not revert unrelated dirty work. Do not commit this repo manually. Root will handle final integration, build, and deploy. You are not alone in the codebase; avoid overwriting other sessions'' changes.

SCOUT_EVIDENCE:
Root checked live agents; none were reusable. Root read the task memo and prior handoff `self-review-verification-worker.md`. Initial search for literal self-review names was sparse, so start from prompt/session/collaboration-mode code rather than assuming names. Existing dirty files include `RefactorGoOnPrompt.txt`, `codex-rs/Cargo.lock`, and `codex-rs/collaboration-mode-templates/templates/plan.md`; inspect before touching and preserve unrelated changes.

WHY_AGENT / ROI:
Main implementation is being delegated per user preference for non-interactive worker sessions. ROI estimate: new_agent_cost=3, parallel_gain=2, context_gain=3, repeat_gain=3, loop_followup_gain=2, risk_penalty=1, net=6. Your compact handoff should let root integrate and verify without re-reading everything.

FIRST_READS:
1. `AGENTS.md`
2. `self-review feature.md`
3. `codex-rs/collaboration-mode-templates/templates/plan.md`
4. Search narrowly for the self-review trigger/prompt injection path, go-on loop continuation, collaboration mode templates, activity journaling, session file/git change tracking, and any existing auto-review code.
5. Read prior handoff `.codex/workflow/agents/handoffs/self-review-verification-worker.md` if present.

REQUIREMENTS:
- Self-review must review only current-session/agent-specific changes tracked by code since the last review, then clear that tracked set after review.
- Track changed file paths in code even when a shell command modified a file that was already dirty before the command; include all files modified by the current session.
- Track git commits made by the current agent/session in code, and clear after review.
- Review is suggestive: inserted like a user prompt and sent in-session, not a disruptive hard mode switch.
- Before the review prompt, insert/send a prompt asking the model to: "please 1. sum-up your recent changes, 2. write your next plans 3. next actions to do"
- Remember that answer in code and after review/actions are done, reinsert a reminder prompt: "Please resume your before-review tasks. Here is the reminder about them: <...>"
- If review findings are not fixed automatically, append/insert another prompt after review findings asking Codex to act on/fix them.
- Self-review should prefer automatic code-driven checkpoint commits before review for all changed files in the current session and after review for all changed files after review. Do not implement this by spending LLM tokens deciding file lists.
- Automatic commits must include untracked code files for Python, Rust, C/C++, batch, PowerShell, JavaScript, PHP, Java, Kotlin, Scala, Swift, Objective-C, C#, and Prolog. Use a centralized allowlist/helper.
- Extend self-review prompt to ask the agent to review and reflect on own actions: fit to current task/user request, strategy optimality, long-term strategy, delegation/parallelization, automation/scripting, prototyping before broad builds/tests, architecture/SOLID/decoupling/quality/complexity, and planning/structure.
- Preserve by code, not by LLM suggestion: initial user prompts, initial accepted plan, and an activity journal since last review. These artifacts should be available to the review prompt.

TOOL_HINTS:
Use `rg` for routing. Use small scripts or focused Rust tests for repeated verification. Prefer adding unit tests around tracking/allowlist/prompt assembly rather than relying only on manual inspection.

VERIFICATION:
Run focused tests for the touched crate(s). If release profile is needed for a known Codex Windows issue, note exact commands and results. Do not run final deployment; root owns build/deploy.

HANDOFF:
Write a concise handoff to `.codex/workflow/agents/self-review-implementation.handoff.md` with:
- files changed
- behavior implemented
- tests run and exact results
- known risks/gaps
- any required root follow-up
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-implementation.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
