$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: wave6_final_verification_plan'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', '# External Worker: wave6_final_verification_plan

You are running as an external non-interactive Codex worker. The root session is only overseeing. Do not spawn subagents.

CONTEXT_AREA:
- Repo: C:/Users/Oleh/Documents/GitHub/open_ai/codex
- Branch: slow-context-budget-mode
- Task: prepare the final build/test/deploy command plan for a local Windows Codex Rust checkout after static review/fixes land.

OWNERSHIP:
- Read-only planning. Write only .codex/workflow/agents/wave6_final_verification_plan.handoff.md
- You are not alone in the codebase. Do not modify source files.

DO_NOT_INSPECT:
- Do not do broad source sweeps. Do not run any build/test/deploy command yourself.

SCOUT_EVIDENCE:
- Root read the codex-rust-build skill and saw build scripts: scripts/build-local-codex.ps1 and scripts/test-local-codex-release.ps1.

WHY_AGENT / ROI:
- Positive ROI: final verification/deploy must happen after integration, and a read-only worker can prepare a precise command sequence without consuming root context.

FIRST_READS:
1. C:/Users/Oleh/.codex/skills/codex-rust-build/SKILL.md
2. scripts/test-local-codex-release.ps1
3. scripts/build-local-codex.ps1
4. .cargo/config.toml
5. docs/long-running-session-performance-verification.md

TOOL_HINTS:
- Use focused reads only.
- Do not run cargo build, cargo test, rustc, npm, deploy scripts, schema generation, or any deploy/activation command.

PLANNING TARGET:
- Produce a concrete final-stage command sequence for root to run after all static blockers are fixed.
- Include command order, expected artifacts, log paths, and how to decide whether a failure should be retried or inspected.
- Include whether deployment is via build script activation/copy and how to verify `codex --version` or equivalent after deploy.

VERIFICATION:
- Planning only; no tests/builds/deploy.

HANDOFF:
- Write .codex/workflow/agents/wave6_final_verification_plan.handoff.md with: command sequence, rationale, prerequisites, expected outputs/artifacts, failure triage notes, and any assumptions.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\wave6_final_verification_plan.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
