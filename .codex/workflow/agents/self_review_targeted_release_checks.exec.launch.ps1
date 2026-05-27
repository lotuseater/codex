$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self_review_targeted_release_checks'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$prompt = @'
# self_review_targeted_release_checks

You are a separate external noninteractive Codex exec worker in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

CONTEXT_AREA:
- Self-review feature targeted verification only.
- Current root session is overseer only; do not ask it for broad context unless blocked.

DO_NOT_INSPECT:
- Do not broadly audit the repo or unrelated merge fallout.
- Do not repair `codex-network-proxy`, unrelated protocol warnings, or merge/build-owner failures.
- Do not revert, overwrite, reformat, or clean up changes you did not make.
- Do not commit, stage, merge, rebase, or run broad builds.

SCOUT_EVIDENCE:
- Root checked `.git/MERGE_HEAD`: absent.
- Root saw active cargo/rustc before sleeping, then rechecked after ~5 minutes and saw no active cargo/rustc.
- Previous handoffs `self-review-check-core-protocol.md`, `self-review-check-app-server.md`, and `self-review-targeted-checks-root.md` reported unrelated compile failures, not self-review assertion failures.

WHY_AGENT / ROI:
- Root delegates verification to preserve overseer role and keep external noninteractive workflow.
- ROI after idle: new_agent_cost=3, parallel_gain=1, context_gain=2, repeat_gain=1, loop_followup_gain=3, risk_penalty=2, net=2.
- No recursive delegation.

FIRST_READS:
- `.codex/workflow/agents/handoffs/self-review-root-overseer.md`
- `scripts/test-local-codex-release.ps1` only if you need wrapper syntax.

TOOL_HINTS:
- Before tests, check `Get-Process cargo,rustc -ErrorAction SilentlyContinue` and `.git\MERGE_HEAD`.
- If another cargo/rustc is active, wait up to 5 minutes once, then write handoff and stop if still active.
- Use the release wrapper, not raw broad debug cargo, unless wrapper invocation itself is impossible.

VERIFICATION:
Run only these targeted lanes, in order, stopping after the first unrelated compile/build-owner failure:
1. `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\test-local-codex-release.ps1 -Package codex-core -Filter tasks::review -Lib -Jobs 1 -NoCleanup`
2. `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\test-local-codex-release.ps1 -Package codex-protocol -Filter review -Lib -Jobs 1 -NoCleanup`
3. `powershell -NoProfile -ExecutionPolicy Bypass -Command "& { .\scripts\test-local-codex-release.ps1 -Package codex-app-server -Filter review -Jobs 1 -AllowIntegrationTargets -ExtraCargoArgs @('--test','all') -NoCleanup }"`

If a command fails:
- Inspect enough of the log to classify it as self-review-specific assertion failure vs unrelated compile/build failure.
- If unrelated compile/build failure, stop immediately. Do not repair it.
- If self-review-specific assertion failure, write exact failure and suggested narrow fix; do not patch in this worker.

HANDOFF:
Write `.codex/workflow/agents/handoffs/self-review-targeted-release-checks.md` with:
- Status: pass / blocked-unrelated-build / self-review-test-failure / skipped-busy.
- Exact commands attempted and exit codes.
- Short failure classification with key compiler/test lines.
- Whether root should stop or delegate a fix worker.
- Percent estimate and ETA.
'@
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', $prompt)
& 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self_review_targeted_release_checks.exec.visible.log'
