# SOLID Refactor Subagent Contract

Use this contract for every subagent spawned for the SOLID crate-splitting
refactor.

## Root-Owned Work

- Only the root agent edits `codex-rs/Cargo.toml`, `Cargo.lock`, Bazel files,
  Git state, merge state, or shared boundary canaries.
- Workers may make path-scoped commits only under
  `.codex/workflow/worker-delegation-commit-protocol.md`; root still owns
  root-level manifest, lockfile, Bazel, merge-state, push, and final aggregate
  commits.
- The root agent wires worker-created crates into the workspace after reviewing
  the worker handoff.

## Worker-Owned Work

- Each worker owns one explicit folder tree and must not edit outside it.
- Workers may create an adjacent crate only when it directly reduces coupling
  with an assigned area and they state that architectural reason in the handoff.
  Root reviews and wires those crates; workers still must not edit root
  manifests.
- Workers create only normal crate folders with `Cargo.toml` and `src/lib.rs`.
- Workers must not create nested workspaces, local `Cargo.lock` files, `target/`
  directories, root manifest edits, or path-local verification artifacts.
- Workers must use `{ workspace = true }` dependencies for sibling crates and
  must not add path dependencies between new workspace members.
- Workers must not run `cargo`, `just`, formatters, Bazel commands, codegen, or
  broad checks unless root explicitly assigns that verification task.
- Workers stop after skeleton creation and return:
  - created crate paths;
  - crate package names;
  - required root workspace member entries;
  - required root workspace dependency entries;
  - SOLID or dependency concerns.

## Architecture Rules

- API/domain crates must not depend on `codex-core`, `codex-core-api`,
  `codex-app-server-protocol`, `codex-thread-store`, `codex-app-server`,
  `codex-tui`, or `codex-mcp-server`.
- Do not add compatibility re-export shims to hide old imports.
- Prefer one entity or policy per crate. If a crate starts owning multiple
  reasons to change, split it before adding behavior.
- Concrete factories and adapters belong in outer adapter or implementation
  crates, never in API/domain crates.
- Do not move MCP elicitation or thread projection types as drive-by cleanup;
  those need a separate boundary review.

## Prompt Prefix

```text
You are not alone in the codebase. This repo is in a dirty merge/refactor.
Do not revert or edit work outside your assigned ownership. Follow
.codex/workflow/solid-refactor-subagent-contract.md exactly. Root owns the
workspace manifest, lockfiles, Bazel files, Git state, staging, commits,
formatting, and verification. You own only: <OWNED_PATHS>. You may create an
adjacent crate only if it directly reduces coupling with your assigned area and
you explain that in your handoff. Do not create nested workspaces, Cargo.lock
files, target directories, path dependencies, or run cargo/just/formatters/checks.
Stop after creating the assigned skeletons and return created paths, package
names, required workspace entries, and concerns.
```

## Adjacent Crate Rule

Workers may propose or create an adjacent crate when it reduces coupling with
the assigned lane and has one clear responsibility. The worker must state:

- why the existing assigned crate is the wrong owner,
- which dependency edge the adjacent crate removes,
- which paths the crate owns,
- which crates may depend on it,
- and which root-owned manifest entries are required.

Workers must not create broad compatibility crates, `core-api` replacements, or
neighboring crates outside the assigned lane merely to make local compilation
easier. Root remains responsible for final workspace wiring.
