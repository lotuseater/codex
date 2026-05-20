# Agent Prompt: bazel_lock_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only Bazel and lockfile wiring scout.

First read:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/manifest_wiring_scout.handoff.md`
- `codex-rs/Cargo.toml`
- `codex-rs/Cargo.lock`
- `codex-rs/MODULE.bazel.lock`

Task:

- Inspect what new crates or dependency moves are already present in
  manifests and lockfiles.
- Identify whether Bazel lock refresh or BUILD.bazel changes are likely needed
  because of new crates, compile-time data, or dependency movement.
- Do not edit source files, manifests, lockfiles, BUILD files, generated files,
  or handoff documents other than your own handoff.
- Do not run Cargo, Just, Bazel, formatters, or Git staging/commits.
- You may delegate focused read-only questions to helper agents if useful.

Write `.codex/workflow/agents/bazel_lock_scout.handoff.md` with:

- exact manifest/lockfile drift observed
- any `BUILD.bazel` or `MODULE.bazel.lock` follow-up likely required
- whether `just bazel-lock-update` / `just bazel-lock-check` should run later
- commit readiness notes, but do not make commits
