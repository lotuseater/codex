# Delegated Session-Maintenance Continuation

## Summary

- Root should stop implementation work after one orchestration step: keep session-maintenance development delegated through the repo's Director-style worker scripts.
- The worker owns finalizing the session-maintenance PowerShell feature, rerunning thorough tests, and writing a concise handoff. Root remains only overseer after compaction.
- Self-review correction: do not spawn multiple coding workers against the same small file set unless the active one is stale or explicitly conflicts.

## Agent ROI Estimate

- Current root-side agent state shows no reusable MultiAgent child agents; the active work is in external Codex/PowerShell worker sessions.
- `loop_followup_gain`: high. A persistent external worker keeps development moving while root compacts and avoids more root context growth.
- Delegation decision: reuse the active session-maintenance worker if it is alive and progressing. Start exactly one takeover worker only if the active worker is stale, closed, or editing against the wrong assignment.
- Director should be relaunched with the repo's normal Director script. Resume by known session ID/state if available; otherwise start a fresh Director and provide the usual marker/log/state info.

## Implementation Changes

- Keep ownership limited to:
  - `.codex/workflow/scripts/CodexSessionMaintenance.psm1`
  - `.codex/workflow/scripts/Watch-CodexSessionMaintenance.ps1`
  - `.codex/workflow/scripts/Start-CodexDirectorLoop.ps1`
  - `.codex/workflow/scripts/Test-CodexSessionMaintenance.ps1`
  - `.codex/workflow/agents/test-terminal-escape-canary.ps1`
  - `.codex/workflow/agents/test-terminal-esc-compact-canary.ps1`
- Preserve the current public surface unless tests expose a real gap:
  - exported maintenance helpers
  - watch script `-PipeName` / `-PipeTimeoutMs` forwarding
  - fake pipe payload path
  - dry-run watch mode
  - Director loop dry-run launch planning
- Workers must not revert unrelated dirty files, overwrite unknown edits, or stage/commit without explicit root/user direction.

## Test Plan

- Run the main harness:
  `powershell -NoProfile -ExecutionPolicy Bypass -File .codex\workflow\scripts\Test-CodexSessionMaintenance.ps1`
- Run the watch dry-run fake-pipe lane with explicit `-PipeName` and `-PipeTimeoutMs`, and confirm the JSON log contains the supplied `pipeName`.
- Run Director loop dry-run:
  `powershell -NoProfile -ExecutionPolicy Bypass -File .codex\workflow\scripts\Start-CodexDirectorLoop.ps1 -DryRun`
- Run relevant terminal escape canary checks for `{ESCAPE}`.
- Verify whitespace:
  `git diff --check -- .codex/workflow/scripts .codex/workflow/agents/test-terminal-escape-canary.ps1 .codex/workflow/agents/test-terminal-esc-compact-canary.ps1`
- Resolve any failing repo-controlled check before handoff.

## Handoff And Defaults

- The active worker handoff must list changed files, exact commands run, pass/fail results, remaining risks, and the focused commit-ready file set.
- If another worker finishes first, the active/finalizing worker uses that handoff as input and verifies it instead of duplicating work.
- No user-choice blocker remains; loop automation can accept the implementation prompt automatically after this plan.
- Broad Rust builds are out of scope because this feature is PowerShell workflow automation.
- Unrelated dirty SOLID/app-server files remain untouched.
