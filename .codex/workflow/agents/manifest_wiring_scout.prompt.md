# Agent Prompt: manifest_wiring_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only manifest and workspace wiring scout.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/auth_boundary.handoff.md`
- `.codex/workflow/agents/thread_projection_boundary.handoff.md`
- `.codex/workflow/agents/mcp_elicitation_boundary.handoff.md`
- `codex-rs/Cargo.toml`

Task:

- Identify exact root workspace member entries and workspace dependency entries
  needed for prepared crates.
- Check likely per-crate `Cargo.toml` dependency names from the worker-owned
  crate files.
- Do not edit manifests or source files.
- Do not run Cargo, Just, formatters, or Git staging/commits.
- You may delegate focused read-only questions if useful.

Write `.codex/workflow/agents/manifest_wiring_scout.handoff.md` with:

- exact manifest edits root should make
- crate package/lib names involved
- dependency order risks
- anything that should not be wired yet
