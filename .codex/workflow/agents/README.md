# SOLID Refactor Agent Work Queue

External Codex sessions use this directory to coordinate with the root
director.

Rules for every session:

- Read `.codex/workflow/solid-refactor-handoff.md` first.
- Read `.codex/workflow/worker-delegation-commit-protocol.md` before editing.
- Do not spawn additional worker sessions or subagents unless root explicitly
  assigns that.
- Do not edit `codex-rs/Cargo.toml`, `codex-rs/Cargo.lock`, Bazel files, or
  lockfiles unless a prompt explicitly grants that.
- Do not run builds or tests while your owned refactor is still in progress.
  After the refactor is complete, run only the focused verification lane your
  prompt allows.
- Edit only owned paths.
- Write final state to the assigned `*.handoff.md` file.
- Commit coherent scoped changes when safe. If a clean commit is blocked, leave
  changes unstaged or path-staged only and record the exact blocker.

Root remains the only integrator.

## Spawn Reliability

Use in-app Codex worktree/session delegation until terminal-spawned workers pass
the repo canaries on the deployed fixed build.

- Do not hand-roll `codex exec` launch arguments. Use
  `CodexWorkerLaunch.psm1` through `start-codex-workers.ps1`,
  `start-codex-interactive.ps1`, or `.codex/workflow/scripts/Start-CodexWorker.ps1`.
- Runtime flags must stay as root `codex` flags before `exec` or `resume`:
  `--cd <repo> --ask-for-approval never --sandbox danger-full-access`.
- The worker prompt must be the final argument so quoting cannot swallow later
  runtime flags.
- Treat `.codex/local-builds/codex-custom-*` provenance as unhealthy for
  terminal workers unless the fixed build has just been deployed and a live
  spawned-terminal probe proves shell/file tools are present.

Cheap static canary:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\Test-CodexWorkerLaunch.ps1 -Repo C:\Users\Oleh\Documents\GitHub\open_ai\codex
```

If the fixed local build is intentionally deployed through the wrapper, allow
that provenance for the static check:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\Test-CodexWorkerLaunch.ps1 -Repo C:\Users\Oleh\Documents\GitHub\open_ai\codex -AllowCustomBuild
```

Post-deploy live terminal probe:

```powershell
$marker = ".codex\workflow\tmp\spawn-terminal-shell-tools.txt"
Remove-Item -LiteralPath $marker -Force -ErrorAction SilentlyContinue
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\start-codex-workers.ps1 -Repo C:\Users\Oleh\Documents\GitHub\open_ai\codex -WorkerNames spawn_terminal_shell_canary -Prompt "You are a spawned terminal canary. Use shell_command to create .codex\workflow\tmp\spawn-terminal-shell-tools.txt containing spawn-shell-ok, then use shell_command to read it back. Finish by reporting the marker path and exact content. Do not edit any other files." -CodexCommand codex
```

After the visible worker exits:

```powershell
Get-Content -LiteralPath .codex\workflow\tmp\spawn-terminal-shell-tools.txt
Get-Content -LiteralPath .codex\workflow\agents\spawn_terminal_shell_canary.exec.visible.log -Tail 80
```
