# Agent Prompt: plugin_tool_compile_worker

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are an edit-owned implementation worker for a narrow plugin tool compile
blocker. You are not alone in this worktree; preserve edits from other sessions,
especially the DAB handler changes, and do not revert unrelated changes.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/agents/compile_plugin_tool_scout.handoff.md`
- `.codex/workflow/agents/dab_availability_worker.handoff.md`
- `codex-rs/core/src/tools/handlers/request_plugin_install.rs`
- `codex-rs/core/src/tools/handlers/mod.rs`
- any directly referenced plugin install/list spec file under
  `codex-rs/core/src/tools/handlers/`

Owned edit paths:

- `codex-rs/core/src/tools/handlers/request_plugin_install.rs`
- `codex-rs/core/src/tools/handlers/request_plugin_install_spec.rs`
- `codex-rs/core/src/tools/handlers/mod.rs`
- `.codex/workflow/agents/plugin_tool_compile_worker.handoff.md`

Rules:

- Do not edit manifests, lockfiles, Bazel files, generated files, tests, or any
  other source paths unless the handoff proves the named file does not exist and
  a directly equivalent plugin handler/spec file is required.
- Do not run Cargo, Just, formatters, Git staging/commits, or broad build lanes.
- You may run targeted `rg`/read-only searches.
- You may delegate bounded read-only questions, but keep ownership of the edit.
- If the patch is unsafe, stop after writing the handoff; do not make speculative
  edits.

Task:

- Replace stale references to removed list-available-plugin tool symbols with
  the current `codex-tools` request-plugin-install source of truth.
- Keep the tool surfaced as request plugin install with the correct description.
- Stop compiling/exporting removed list-available-plugin handler/spec modules if
  the scout's findings are still accurate.

Write `.codex/workflow/agents/plugin_tool_compile_worker.handoff.md` with:

- files read and changed
- exact stale symbols removed or replaced
- commands/searches used for verification
- remaining compile blockers outside your owned files
- exact commit pathspec if root can commit this slice after verification
