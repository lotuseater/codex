$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: prompt_policy_noninteractive_followup'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Follow-up implementation for Codex Rust prompt-policy reinforcement around worker delegation. The review handoff says current changes are close but still too weak on separate non-interactive worker sessions.

DO_NOT_INSPECT: Do not do broad repo exploration. Do not change build/deploy scripts except the local `.codex/workflow/agents/start-codex-workers.ps1` helper if you find a direct bug. Do not commit, push, deploy, or revert unrelated user/root changes. You are not alone in this codebase; coordinate by touching only the owned files below and preserving existing dirty changes.

SCOUT_EVIDENCE: Root already inspected `.codex/workflow/agents/prompt_policy_reinforce_impl.handoff.md` and `.codex/workflow/agents/prompt_policy_reinforce_review.handoff.md`. Review findings: prompts do not strongly prefer separate non-interactive Codex sessions in separate PowerShell terminals; do not explicitly push main/root to start at least one highest-capability worker for complex/planning/likely-compaction tasks when no suitable workers are active; highest-model/effort guidance is not central enough; tests do not lock down these exact requirements.

WHY_AGENT / ROI: User explicitly wants the main agent to delegate implementation/testing, prefer non-interactive external sessions, keep root narrow, and sleep/check between handoffs. Agent ROI estimate: new external worker cost=3, parallel_gain=1, context_gain=3, repeat_gain=2, loop_followup_gain=3, risk_penalty=1 => net=5. This bounded task should be done by a separate worker.

FIRST_READS:
- `codex-rs/agent-policy/src/lib.rs`
- `codex-rs/agent-policy/src/plan_prompt.rs`
- `codex-rs/tools/src/plan_tool.rs`
- `codex-rs/tools/src/dynamic_tool_tests.rs`
- `codex-rs/collaboration-mode-templates/templates/plan.md`
- `RefactorGoOnPrompt.txt`
- `.codex/workflow/agents/prompt_policy_reinforce_review.handoff.md`

OWNERSHIP / REQUIRED CHANGES:
- Strengthen the central prompt/policy language so for complex enough tasks, planning tasks, or likely context-drift/context-compaction tasks, the main/root agent should strongly prefer spawning at least one separate non-interactive Codex worker session in a separate PowerShell terminal when no suitable worker is already active.
- Say highest-capability model and highest useful reasoning effort should be preferred for those workers.
- Say non-interactive external sessions are preferred over in-session spawned agents and over interactive terminals for most bounded implementation/testing work, because more can run with less hanging. Interactive sessions are for live course correction, commands, redirects, or follow-ups.
- Preserve the existing guidance to avoid recursive delegation. Workers should not spawn more workers unless explicitly authorized by the root prompt.
- Make root/main responsibilities clear: root owns orchestration, context, handoffs, integration decisions, follow-ups, and sleeping about 5 minutes between worker checks; workers should do most implementation/testing for delegated subtasks and return compact handoffs.
- Keep portable PowerShell guidance independent from this PC''s script path. Mention creating prompt/handoff files in the workspace and launching from the workspace with `Start-Process powershell`/`powershell -NoExit ...`; do not rely on a machine-specific absolute script path.
- Add/update tests so the exact behavioral requirements above are checked where practical: non-interactive external session preference, at least one worker when no suitable active worker exists, highest model/effort preference, root sleep/check cadence, root not doing implementation/testing for delegated tasks, recursion avoidance, and `go on`/loop planning coverage.

TOOL_HINTS: Use `rg`/targeted reads first. Use `apply_patch` for edits. Focus tests around `codex-agent-policy` and `codex-tools`; release cargo tests are acceptable. Do not run full workspace builds or deploys.

TOKEN_TIP: Keep the final handoff short. Include changed files, tests run with results, and any residual risks.

VERIFICATION: Run the narrow tests you changed or that cover the changed prompt generation. If test execution is blocked, capture the exact command and failure.

HANDOFF: Write `.codex/workflow/agents/prompt_policy_noninteractive_followup.handoff.md` with concise findings/results for root. Do not leave the handoff vague.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\prompt_policy_noninteractive_followup.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
