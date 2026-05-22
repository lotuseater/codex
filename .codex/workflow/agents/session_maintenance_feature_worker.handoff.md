# Session Maintenance Feature Worker Handoff

## Changed Files

- `.codex/workflow/scripts/CodexSessionMaintenance.psm1`
  - Keeps the existing token parser, threshold planner, compaction-reduction checks, `codex --loop resume` command builder, shared-read JSONL session access, and named-pipe control path.
  - Adds conservative Wizard sidecar pipe resolution via `Resolve-CodexWizardManagedPipe`. It resolves only when the session/project cwd matches exactly one live Codex sidecar with `loop_target_pwsh_pipe`; stale process metadata is ignored.
- `.codex/workflow/scripts/Watch-CodexSessionMaintenance.ps1`
  - Accepts and passes `-PipeName` / `-PipeTimeoutMs`.
  - Adds explicit `-ResolveManagedPipe` plus `-ManagedTerminalRoot`; if requested and no live matching pipe is found, it fails instead of guessing or falling back to focus-based terminal control.
  - Logs `pipeName` and `managedPipeSource` in JSONL watch events.
- `.codex/workflow/scripts/Test-CodexSessionMaintenance.ps1`
  - Covers parser fixtures, threshold decisions, compaction reduction validation, `codex --loop resume` command construction, pipe payload creation, fake named-pipe interrupt/write/reminder behavior, conservative sidecar resolution, stale sidecar rejection, and watcher JSONL logging.
- `.codex/workflow/scripts/Start-CodexDirectorLoop.ps1`
  - Existing added Director resume/fresh-start launcher; verified by test and dry-run.
- `.codex/workflow/agents/test-terminal-escape-canary.ps1`
- `.codex/workflow/agents/test-terminal-esc-compact-canary.ps1`
  - Pre-existing canary edits from this feature branch use `{ESCAPE}` instead of `{ESC}`.

## Verification

- PASS: `powershell -NoProfile -ExecutionPolicy Bypass -File .codex\workflow\scripts\Test-CodexSessionMaintenance.ps1`
  - Result: `status = passed`, `assertions = 49`.
- PASS: `powershell -NoProfile -ExecutionPolicy Bypass -File .codex\workflow\scripts\Start-CodexDirectorLoop.ps1 -DryRun | ConvertTo-Json -Depth 6`
- PASS: `powershell -NoProfile -ExecutionPolicy Bypass -File .codex\workflow\scripts\Watch-CodexSessionMaintenance.ps1 -Profile Director -SessionPath .codex\workflow\tmp\session-maintenance-tests\rollout-test.jsonl -DryRun -Once -LogPath .codex\workflow\tmp\session-maintenance-tests\watch.jsonl`
- PASS: `powershell -NoProfile -ExecutionPolicy Bypass -File .codex\workflow\scripts\Watch-CodexSessionMaintenance.ps1 -Profile Director -SessionPath .codex\workflow\tmp\session-maintenance-tests\rollout-test.jsonl -DryRun -Once -PipeName fake-maintenance-pipe -PipeTimeoutMs 1234 -LogPath .codex\workflow\tmp\session-maintenance-tests\watch-pipename.jsonl`
- PASS: `powershell -NoProfile -ExecutionPolicy Bypass -Command "Import-Module 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\scripts\CodexSessionMaintenance.psm1' -Force -ErrorAction Stop; $pipe = Resolve-CodexWizardManagedPipe -Project 'C:\Users\Oleh\Documents\GitHub\Serial_to_Google_Doc_topdown'; if ($null -eq $pipe) { 'no-live-pipe' } else { $pipe | ConvertTo-Json -Depth 4 }"`
  - Result: `no-live-pipe`.
- PASS: `git diff --check -- .codex/workflow/scripts/CodexSessionMaintenance.psm1 .codex/workflow/scripts/Watch-CodexSessionMaintenance.ps1 .codex/workflow/scripts/Test-CodexSessionMaintenance.ps1 .codex/workflow/scripts/Start-CodexDirectorLoop.ps1 .codex/workflow/agents/test-terminal-escape-canary.ps1 .codex/workflow/agents/test-terminal-esc-compact-canary.ps1`
  - Note: Git warned that the two canary files will be normalized from LF to CRLF when touched.

## Remaining Blockers

- Live Director automation is not proven ready. The only real Wizard sidecar pipe found locally was stale: the sidecar had `loop_target_pwsh_pipe`, but its recorded PowerShell process was not live, and the resolver returned `no-live-pipe`.
- GUI terminal canaries remain diagnostic only from this tool session because launched windows are not reliably discoverable here.

## Root Commands

- Re-run the full non-GUI coverage:
  `powershell -NoProfile -ExecutionPolicy Bypass -File .codex\workflow\scripts\Test-CodexSessionMaintenance.ps1`
- Dry-run Director resume planning:
  `powershell -NoProfile -ExecutionPolicy Bypass -File .codex\workflow\scripts\Start-CodexDirectorLoop.ps1 -DryRun`
- Watch with an explicit verified pipe:
  `powershell -NoProfile -ExecutionPolicy Bypass -File .codex\workflow\scripts\Watch-CodexSessionMaintenance.ps1 -Profile Director -SessionPath <session.jsonl> -PipeName <wizard-loop-pipe> -PipeTimeoutMs 5000 -LogPath .codex\workflow\session-maintenance\director-watch.jsonl`
- Watch with conservative Wizard sidecar resolution after a live managed Codex terminal exists:
  `powershell -NoProfile -ExecutionPolicy Bypass -File .codex\workflow\scripts\Watch-CodexSessionMaintenance.ps1 -Profile Director -SessionPath <session.jsonl> -ResolveManagedPipe -LogPath .codex\workflow\session-maintenance\director-watch.jsonl`
