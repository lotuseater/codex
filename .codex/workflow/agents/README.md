# SOLID Refactor Agent Work Queue

External Codex sessions use this directory to coordinate with the root
director.

Rules for every session:

- Read `.codex/workflow/solid-refactor-delegation-director-plan.md` first.
- Read `.codex/workflow/solid-refactor-subagent-contract.md` before editing.
- Do not use Git staging, commits, resets, or checkouts.
- Do not edit `codex-rs/Cargo.toml`, `codex-rs/Cargo.lock`, Bazel files, or
  lockfiles unless a prompt explicitly grants that.
- Do not run formatters, broad Cargo builds, or Just tasks unless a prompt
  explicitly grants that.
- Edit only owned paths.
- Write final state to the assigned `*.handoff.md` file.
- If delegating further inside the external session, keep child agents inside
  the same owned paths and summarize their output in the same handoff file.

Root remains the only integrator.
