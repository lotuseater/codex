# Codex Repo Review And Improvement Roadmap

## Scope

This note reviews the current repository shape and identifies practical improvements,
optimizations, and refactor candidates. The scan intentionally ignored large build
and output folders such as `target`, `build`, `dist`, and `node_modules`.

Sources inspected:

- Root project files: `README.md`, `package.json`, `justfile`, `AGENTS.md`.
- Rust workspace manifest: `codex-rs/Cargo.toml`.
- Repository docs under `docs/`.
- SDK/package manifests under `codex-cli/`, `sdk/python/`, and `sdk/typescript/`.
- CI documentation and workflow sizes under `.github/workflows/`.
- File-size and TODO scans across Rust, SDK, and script sources, excluding build
  folders and generated-heavy directories where practical.

This is a review and action plan only. It does not propose changing generated
schemas, snapshots, lockfiles, source code, or CI behavior in the same step.

## Current Shape

The repository is a broad monorepo around the Codex CLI and app-server ecosystem.
The main implementation surface is the Rust workspace under `codex-rs`, with
supporting npm packaging, Python and TypeScript SDKs, Bazel integration, release
workflows, docs, scripts, vendored/native patches, and local fork support notes.

Evidence from the scan:

- About 4,050 non-build files were scanned.
- About 1,663 Rust files were present outside ignored build/vendor areas.
- About 166 Markdown files and 425 snapshot files were present.
- `codex-rs/Cargo.toml` defines a large workspace with many focused crates, but
  `codex-rs/core`, `codex-rs/tui`, `codex-rs/app-server`, and
  `codex-rs/app-server-protocol` still dominate the source footprint.
- `.github/workflows/README.md` documents a useful split between fast PR checks
  and heavier post-merge Cargo-native validation.

The repository already has strong guardrails: strict clippy lints, snapshot
coverage for TUI, app-server schema generation, config schema generation, Bazel
checks, and local workflow docs. The highest-value improvements are therefore
not broad rewrites. They are targeted reductions in complexity around oversized
orchestration modules, generated artifact hygiene, and contributor documentation.

## High-Value Findings

### 1. Oversized Orchestration Modules

Several modules are large enough that changes are likely to be slower to review,
harder to test in isolation, and more likely to create unrelated conflicts.

Largest Rust files observed outside ignored build/vendor areas:

- `codex-rs/tui/src/chatwidget.rs`: about 12,336 lines.
- `codex-rs/app-server/src/codex_message_processor.rs`: about 11,495 lines.
- `codex-rs/app-server-protocol/src/protocol/v2.rs`: about 10,703 lines.
- `codex-rs/tui/src/bottom_pane/chat_composer.rs`: about 9,176 lines.
- `codex-rs/core/src/config/config_tests.rs`: about 8,013 lines.
- `codex-rs/core/src/session/tests.rs`: about 7,987 lines.
- `codex-rs/protocol/src/protocol.rs`: about 5,237 lines.
- `codex-rs/tui/src/history_cell.rs`: about 5,224 lines.
- `codex-rs/tui/src/app/tests.rs`: about 5,096 lines.
- `codex-rs/app-server/src/bespoke_event_handling.rs`: about 4,964 lines.

Recommendation: treat these as gradual extraction targets. Do not begin with
large rewrites. Extract cohesive behavior in small slices, move adjacent tests
with the behavior when possible, and keep public API shape stable.

### 2. Core Crate Pressure

`codex-rs/core` has the largest Rust file count by a wide margin. This matches
the repo guidance to resist adding new concepts to `codex-core` unless no focused
crate or module fits.

Recommendation: make "does this need to live in core?" an explicit review
checkpoint for new features. Prefer existing focused crates such as protocol,
config, shell-command, exec, plugin, app-server, or small utility crates where
the ownership boundary is already clearer.

### 3. App-Server And Protocol Contract Size

The v2 protocol file and generated schema tree are large. This is expected for a
broad API surface, but it increases drift risk between Rust types, generated
JSON schema, generated SDK artifacts, and documentation.

Recommendation: document the app-server contract lifecycle in one place:

- Where v2 Rust API types live.
- Which commands regenerate schema fixtures.
- Which SDK artifacts depend on those schemas.
- Which tests catch drift.
- When experimental API flags must be updated.

This does not require changing wire shape. It is mainly a contributor workflow
and review-safety improvement.

### 4. Documentation Depth Is Uneven

Some docs are substantial and useful, especially TUI-specific docs. Others are
very small and appear to be redirect/stub-style files, including:

- `docs/license.md`
- `docs/skills.md`
- `docs/authentication.md`
- `docs/example-config.md`
- `docs/execpolicy.md`
- `docs/slash_commands.md`
- `docs/exec.md`
- `docs/sandbox.md`
- `docs/getting-started.md`

Recommendation: either expand these files into real contributor/user docs or
make them explicit redirect/index pages. Tiny docs are not automatically bad,
but they should not make contributors guess whether important material is
missing.

### 5. CI And Release Workflows Are Large

The workflow directory already explains the intended split:

- `bazel.yml` is the main PR-time Rust verification path.
- `rust-ci.yml` keeps Cargo-native PR checks small.
- `rust-ci-full.yml` runs heavier Cargo-native coverage after merge.

That strategy is sound. The maintenance risk is file size and repeated release
setup across workflow files. Notable workflow sizes observed:

- `rust-release.yml`: about 792 lines.
- `rust-ci-full.yml`: about 770 lines.
- `issue-deduplicator.yml`: about 402 lines.
- `bazel.yml`: about 298 lines.
- `rust-release-windows.yml`: about 286 lines.

Recommendation: preserve the fast-PR/full-post-merge split, but review repeated
setup and packaging steps for reusable actions or small scripts. Favor reducing
duplication without hiding platform-specific release details.

### 6. TODO Clusters Point To Active Risk Areas

TODOs are clustered around:

- Analytics and tool-call accounting.
- App-server compatibility fallbacks and capability scoping.
- Plugin sync and install/enabled state separation.
- Protocol cleanup and optional fields planned to become required.
- Compaction behavior and context-update diffing.
- Delegated agents and approval/elicitation behavior.
- Windows shell, exec policy, and unified exec behavior.
- Ignored or flaky tests.

Recommendation: do not treat TODO count as a cleanup metric. Instead, create a
triage list grouped by risk and ownership:

- Compatibility TODOs with removal criteria.
- Ignored/flaky tests with reproduction notes.
- Contract TODOs that require schema or SDK changes.
- Telemetry/accounting TODOs that affect product observability.
- Refactor TODOs that can wait for nearby feature work.

## Prioritized Roadmap

### Phase 1: Documentation And Ownership Cleanup

This phase has low behavioral risk and improves contributor throughput.

Actions:

- Add or expand a central contributor note for generated artifacts: config
  schema, app-server schema, SDK generation, and TUI snapshots.
- Convert tiny docs into either meaningful pages or explicit redirects to the
  canonical source.
- Add short ownership notes for large subsystems: TUI, app-server, protocol,
  core session/config, SDK generation, release workflows.
- Create a TODO triage document or issue list grouped by the categories above.

Acceptance criteria:

- A contributor can tell which command regenerates each generated surface.
- Stub-like docs either contain useful content or clearly point elsewhere.
- TODO cleanup work has categories and removal criteria instead of a flat list.

### Phase 2: Large Module Extraction In Safe Slices

This phase should be done incrementally and verified after each slice.

Recommended order:

1. `codex-rs/tui/src/chatwidget.rs`
   - Extract narrowly scoped rendering, history/status layout, or event adapter
     behavior.
   - Keep orchestration in `chatwidget.rs`; move cohesive leaf behavior out.
   - Preserve or move snapshot tests with visible UI changes.

2. `codex-rs/tui/src/bottom_pane/chat_composer.rs`
   - Continue splitting attachment handling, large-paste placeholders, wrapping,
     and history recall into focused modules.
   - Keep text wrapping behavior aligned with existing `tui/src/wrapping.rs`
     helpers.

3. `codex-rs/app-server/src/codex_message_processor.rs`
   - Split RPC routing, session/thread lifecycle, auth/account handling, and
     event adaptation.
   - Keep v2 API behavior stable and test through app-server suite coverage.

4. `codex-rs/app-server-protocol/src/protocol/v2.rs`
   - Group definitions by resource where possible while preserving generated
     schema output.
   - Avoid wire-shape changes unless the task is explicitly an API change.

Acceptance criteria:

- Each extraction reduces a large file by moving cohesive behavior, not by
  spreading unrelated helpers.
- Tests move closer to owned behavior where practical.
- Snapshot or schema changes are intentional and reviewed when user-visible or
  contract-visible output changes.

### Phase 3: Contract And Generated Artifact Hygiene

This phase reduces drift across Rust protocol types, schema JSON, SDKs, and docs.

Actions:

- Add one high-level "contract surfaces" doc covering app-server schema, config
  schema, generated TypeScript/Rust/Python artifacts, and snapshot expectations.
- Make regeneration commands easy to discover from both root and `codex-rs`.
- Add small drift checks where current CI does not already cover them.
- Ensure experimental API changes update both Rust metadata and generated schema
  fixtures.

Acceptance criteria:

- API changes have a clear checklist.
- Generated artifact diffs are expected, explainable, and locally reproducible.
- SDK contributors can trace generated files back to source schema.

### Phase 4: Build And CI Maintenance

This phase should optimize maintenance cost without weakening current signal.

Actions:

- Identify repeated setup blocks in release workflows and decide whether they
  belong in reusable actions or scripts.
- Preserve platform-specific clarity for Windows, macOS, Linux, and Bazel/RBE
  paths.
- Keep PR checks fast; push expensive cross-platform Cargo-native checks to the
  existing full workflow unless a check is essential for pre-merge safety.
- Audit dependency and lockfile update guidance so Cargo, Bazel, npm, pnpm, and
  SDK lockfiles have clear ownership.

Acceptance criteria:

- CI remains easy to reason about from `.github/workflows/README.md`.
- Reused setup reduces duplication without obscuring failure causes.
- Contributors know which lockfiles must be updated for each dependency class.

## Optimization Opportunities

- Build time: keep relying on targeted crate tests and scoped `just fix -p`
  invocations for local work. Avoid defaulting to full workspace tests unless a
  shared crate or protocol contract changed.
- Review time: prioritize smaller modules and clearer ownership boundaries over
  style-only churn.
- Runtime observability: continue adding lightweight accounting around token
  usage, compaction, tool output truncation, and deferred tool loading before
  changing policy.
- Test reliability: convert ignored flaky tests into tracked work with owner,
  reproduction command, and expected unblock condition.
- Generated artifacts: make drift visible with explicit check commands and docs,
  not ad hoc reviewer memory.

## Concrete Next Actions

1. Write a generated-artifact lifecycle note that covers schema, SDK, snapshot,
   and config regeneration.
2. Expand or redirect the smallest docs so the docs index does not contain
   ambiguous stubs.
3. Create a TODO triage issue or Markdown note grouped by compatibility,
   telemetry, flaky tests, protocol cleanup, and refactor debt.
4. Pick one TUI extraction from `chatwidget.rs` that can move with tests and no
   behavior change.
5. Pick one app-server message-processor extraction that only moves code and
   preserves v2 behavior.
6. Review release workflow duplication after the codebase ownership docs are in
   place.

## Guardrails

- Do not combine large refactors with behavioral feature work.
- Do not add new public API surface while extracting modules unless the task is
  explicitly an API change.
- Do not add feature code to `codex-core` by default; prove the ownership fit.
- Do not update generated schemas, SDK artifacts, or snapshots without the
  corresponding source change and regeneration command.
- Do not weaken the existing fast PR versus full post-merge verification split.
