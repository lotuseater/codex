$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-main'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Codex Rust self-review workflow implementation in this repo.
DO_NOT_INSPECT: Do not investigate the prompt-reducer verification finding unless it directly blocks compilation in files you touch. Do not browse the internet. Do not run broad builds, full workspace tests, release builds, deploy scripts, or schema generation. Do not edit unrelated dirty files such as RefactorGoOnPrompt.txt, Cargo.lock, Cargo.toml dependency churn, or collaboration template changes unless they are directly required for this self-review feature.
SCOUT_EVIDENCE: Root read AGENTS.md, self-review feature.md, live git status, existing worker handoffs, and found no reusable live internal agent. Existing self-review-verification-worker handoff concerns prompt-reducer PowerShell path labeling and is not the main self-review implementation.
WHY_AGENT / ROI: User requested non-interactive external-terminal delegation. Main work is code-editable and benefits from preserving root context while you implement. ROI estimate: new_agent_cost=3, parallel_gain=2, context_gain=3, repeat_gain=1, loop_followup_gain=2, risk_penalty=1, net=4.
OWNERSHIP: You own the core source edits needed for self-review behavior. If code changes are needed, keep them focused to the self-review/loop/TUI/session modules and their focused tests. You are not alone in the codebase: do not revert or overwrite other agents'' or user''s unrelated work; work with the live dirty tree.
FIRST_READS:
- self-review feature.md
- AGENTS.md
- Use rg to locate existing self-review, auto-loop, Plan mode, inserted user prompt, turn history, and git/worktree tracking code.
- Read exact files before editing; do not raw-sweep the whole repo if targeted rg gives enough context.
TASK:
Implement the requested self-review improvements:
1. Track this current agent/session''s changed file paths by code since last self-review, then forget those paths after the review cycle. Include files modified by the current session even if already dirty before the command. Do not rely on LLM judgement to choose review files.
2. Track git commits made by this agent/session by code since last review, then forget those commit ids after the review cycle.
3. Make self-review suggestive: insert it as a normal user-style prompt into the session and submit it the same way as a user prompt, not as a disruptive system-only side path.
4. Before review, insert and submit this prompt exactly or semantically unchanged:
   please 1. sum-up your recent changes, 2. write your next plans 3. next actions to do
   Remember that answer by code, and after review plus any actions are complete, reinsert:
   Please resume your before-review tasks. Here is the reminder about them: <...>
5. After review findings, ensure they are acted on. If Codex does not fix findings automatically, add a follow-up prompt that asks Codex to fix the review findings.
6. Auto-commit by code, without spending LLM tokens choosing files: before review, commit all agent/session changed files and relevant untracked code files; after review/actions, commit all changed files again and relevant untracked code files. Relevant untracked code extensions include at least Python, Rust, C/C++, bat, ps1, JavaScript/TypeScript, PHP, Java, Kotlin, Scala, Swift, Objective-C, C#, and Prolog. Use deterministic code for inclusion.
7. Extend self-review prompt to review own actions and reflect on whether actions are optimal for the current task/user request, strategy, long-term perspective, delegation/parallelization, automation/scripting, prototyping before broad tests/builds, architecture/SOLID/decoupling/quality/complexity, and planning/structure.
8. Preserve by code, not LLM suggestion, these artifacts for review context: initial user prompts, initially accepted plan, and an activity journal since last review. The activity journal should be maintained by code, likely as a separate file or session artifact, and reset/rolled after review as appropriate.
9. Prefer small simulations/unit tests or narrow test updates if cheap. Do not run broad build/test now.
TOOL_HINTS:
- Prefer apply_patch for edits.
- Use rg/read-file probes and small scripts for deterministic reasoning if useful.
- Keep changes minimal and integrated with existing types/helpers.
- If there is an existing test harness for TUI/loop self-review behavior, update focused tests only; otherwise leave clear handoff notes for root.
VERIFICATION:
- Do not run cargo build/test, release tests, deploy scripts, or full workspace checks.
- You may run formatting on individual touched Rust files only if clearly safe and cheap; otherwise leave it for root.
- Optionally simulate prompt sequencing with a focused unit test or local code inspection.
HANDOFF:
Write .codex/workflow/agents/handoffs/self-review-main.md with:
- Summary
- Files changed
- Behavior implemented
- Focused verification or simulations run
- Known risks / root follow-ups
- Exact commands not run because of the no-broad-build constraint
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-main.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
