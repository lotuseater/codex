# session_maintenance_fast_followup_worker handoff

## Summary

Reviewed the owned session-maintenance files from disk and patched two concrete issues:

- `.codex/workflow/scripts/CodexSessionMaintenance.psm1`
  - Normalized the exported pipe payload helper so `send_keys` maps to the Wizard PowerShell control pipe's wire verb, `keys`.
  - Added explicit pipe reply validation for `status = error` and `ok = false` responses before callers continue.
  - Preserved `Plan` on live compaction failure and success results so the watcher can log real live outcomes, not only dry-run outcomes.
- `.codex/workflow/scripts/Test-CodexSessionMaintenance.ps1`
  - Added coverage for the `send_keys` -> `keys` payload shape.
  - Added a fake named-pipe error-response test that verifies explicit pipe failures throw.
  - Added live fake-pipe assertion that successful maintenance results retain `Plan` for watcher logging.

No edits were made in this pass to:

- `.codex/workflow/scripts/Watch-CodexSessionMaintenance.ps1`
- `.codex/workflow/scripts/Start-CodexDirectorLoop.ps1`
- `.codex/workflow/agents/test-terminal-escape-canary.ps1`
- `.codex/workflow/agents/test-terminal-esc-compact-canary.ps1`

## Verification

- `powershell -NoProfile -ExecutionPolicy Bypass -File .codex\workflow\scripts\Test-CodexSessionMaintenance.ps1`
  - Passed: `status = passed`, `assertions = 53`.
- `powershell -NoProfile -ExecutionPolicy Bypass -File .codex\workflow\scripts\Watch-CodexSessionMaintenance.ps1 -Profile Director -SessionPath .codex\workflow\tmp\session-maintenance-tests\rollout-test.jsonl -PipeName fake-pipe-for-dry-run -DryRun -Once -LogPath .codex\workflow\tmp\session-maintenance-tests\watch-review.jsonl`
  - Passed: logged `status = dry_run_threshold_reached`, `pipeName = fake-pipe-for-dry-run`, `dryRun = True`.
- `powershell -NoProfile -ExecutionPolicy Bypass -File .codex\workflow\scripts\Start-CodexDirectorLoop.ps1 -SessionPath .codex\workflow\tmp\session-maintenance-tests\rollout-test.jsonl -SessionRoot .codex\workflow\tmp\session-maintenance-tests -DryRun`
  - Passed: returned `status = resume_planned`, resolved `sessionId = 019e-test-session`, and built `codex --loop resume ...`.
- `git diff --check -- .codex\workflow\scripts\CodexSessionMaintenance.psm1 .codex\workflow\scripts\Watch-CodexSessionMaintenance.ps1 .codex\workflow\scripts\Start-CodexDirectorLoop.ps1 .codex\workflow\scripts\Test-CodexSessionMaintenance.ps1 .codex\workflow\agents\test-terminal-escape-canary.ps1 .codex\workflow\agents\test-terminal-esc-compact-canary.ps1 .codex\workflow\agents\session_maintenance_fast_followup_worker.handoff.md`
  - Passed with no output.
- Trailing-whitespace scan over the same owned files:
  - Passed: no trailing whitespace in owned files.

Current scoped git status:

- Modified before/around this slice: `.codex/workflow/agents/test-terminal-escape-canary.ps1`, `.codex/workflow/agents/test-terminal-esc-compact-canary.ps1`.
- Untracked session-maintenance files in this slice: `.codex/workflow/scripts/CodexSessionMaintenance.psm1`, `.codex/workflow/scripts/Watch-CodexSessionMaintenance.ps1`, `.codex/workflow/scripts/Start-CodexDirectorLoop.ps1`, `.codex/workflow/scripts/Test-CodexSessionMaintenance.ps1`, and this handoff.

## Remaining blockers / caveats

- No repo-controlled blocker remains for the non-GUI path.
- The GUI/window canaries remain diagnostic only and were not run in this pass, because this tool session has known trouble discovering its own spawned windows.
- Real live Wizard PowerShell pipe control was not exercised against an actual Codex terminal; the fake named-pipe path now covers request order, explicit error handling, token reduction, and reminder send.

Root can safely use the scripts for dry-run inspection and fake-pipe verified maintenance. For a real live terminal, start with explicit `-PipeName` or `-ResolveManagedPipe` and keep the watcher log path visible.

## Root follow-up implementation

Added a small repo-local hardening slice without touching the live Director:

- `.codex/workflow/scripts/Watch-CodexSessionMaintenance.ps1`
  - Watch JSONL records now include `pipeTimeoutMs`, `pipeSource`, `resolveManagedPipe`, `rootPid`, and `windowHandle` so fake-pipe and future live-pipe runs are self-auditing from the log alone.
- `.codex/workflow/scripts/Test-CodexSessionMaintenance.ps1`
  - Added script-level watcher dry-run coverage that asserts explicit fake pipe name, timeout forwarding, explicit pipe source, and no invented managed source.
  - Added static coverage for both terminal canary scripts so they keep using `{ESCAPE}` and do not regress to control-c style cancellation.

Verification after this follow-up:

- `powershell -NoProfile -ExecutionPolicy Bypass -File .codex\workflow\scripts\Test-CodexSessionMaintenance.ps1`
  - Passed: `status = passed`, `assertions = 63`.
