# commit_group_scout Handoff

Status: completed read-only commit grouping scout on 2026-05-20.

## Scope

Read first:

- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/solid-refactor-handoff.md`

Inspected without staging, committing, resetting, checking out, or editing source:

- `git status --short --untracked-files=all`
- `git diff --stat`
- `git diff --name-status`
- `git diff --cached --name-status`
- `git ls-files -u`
- targeted diffs and existing worker handoffs

Only this handoff file was written.

## Git State Summary

- `git diff --cached --name-status`: empty.
- `git ls-files -u`: empty.
- `git diff --shortstat`: `119 files changed, 1105 insertions(+), 7917 deletions(-)` before writing this handoff.
- There are many untracked files, led by `.codex/workflow-batch/**`, `.codex/workflow/**`, and new `codex-rs/**` crate directories.
- Current code is not commit-ready as small worker slices. The existing handoffs still record release compile blockers and a failing boundary canary.

## Proposed Commit Groups

### 1. Workflow Handoffs And Refactor Notes

Purpose: preserve the coordination artifacts separately from code.

Exact pathspecs:

```text
.codex/workflow/worker-delegation-commit-protocol.md
.codex/workflow/solid-refactor-handoff.md
.codex/workflow/solid-refactor-subagent-contract.md
.codex/workflow/agents/README.md
.codex/workflow/agents/*.prompt.md
.codex/workflow/agents/*.handoff.md
.codex/workflow/agents/*.exec.marker.txt
.codex/workflow/agents/*.marker.txt
.codex/workflow/agents/*.reads.report.json
.codex/workflow/agents/baseline-*.txt
.codex/workflow/agents/codex-exec-help.txt
docs/current-project-architecture.md
docs/current-project-architecture-solid-refactor-plan.md
docs/SELF_REVIEW_2026-05-20.md
```

Verification state: commit-ready only as documentation/audit trail. No Rust verification applies, but review queued placeholder handoffs before committing. Do not include `.codex/workflow-batch/**` or `.codex/workflow/.tmp/**`.

### 2. Boundary Canary And Architecture Prototype

Purpose: keep the boundary measurement script and baseline independent from implementation.

Exact pathspecs:

```text
.codex/prototypes/check-core-boundaries.ps1
.codex/prototypes/check-core-boundaries.baseline.txt
```

Verification state: not green. `boundary_delta_scout` records the canary at exit code `1` with `23` violations. Commit only if root wants to preserve the current failing baseline as an explicit checkpoint; otherwise leave unstaged until the canary is rerun and intentionally accepted.

### 3. Root Workspace And New Boundary Crates

Purpose: introduce the SOLID boundary/API crate topology and shared workspace wiring.

Exact pathspecs:

```text
codex-rs/Cargo.toml
codex-rs/Cargo.lock
codex-rs/adapters/README.md
codex-rs/app/app-catalog-api/**
codex-rs/app/app-catalog-types/**
codex-rs/context-domain/**
codex-rs/core-api/**
codex-rs/core-domain/**
codex-rs/mcp/elicitation-api/**
codex-rs/runtime-domain/**
codex-rs/session/**
codex-rs/tools-domain/**
codex-rs/turn/**
```

Verification state: not commit-ready. This group is root-owned because it touches `codex-rs/Cargo.toml` and `codex-rs/Cargo.lock`. `manifest_wiring_scout` explicitly left manifest/protocol integration to root, and Bazel lock/schema refreshes were skipped. Do not commit until workspace wiring, release compile, and any required lock/schema refreshes are complete.

### 4. Thread Store Split And Thread Domain

Purpose: move the old concrete `thread-store` surface into the new thread API/implementation layout and migration numbering.

Exact pathspecs:

```text
codex-rs/thread-store/**
codex-rs/thread/**
codex-rs/thread-manager-sample/**
codex-rs/state/migrations/0032_stage1_outputs_metadata.sql
codex-rs/state/migrations/0035_stage1_outputs_metadata.sql
codex-rs/core/Cargo.toml
codex-rs/core/src/codex_thread.rs
codex-rs/core/src/session/**
codex-rs/app-server/Cargo.toml
codex-rs/app-server/src/request_processors/thread_processor.rs
codex-rs/app-server/src/request_processors/thread_processor_tests.rs
codex-rs/app-server/tests/suite/conversation_summary.rs
codex-rs/app-server/tests/suite/v2/thread_read.rs
codex-rs/app-server/tests/suite/v2/thread_unarchive.rs
```

Verification state: not commit-ready. `integration_order_scout` recommends this lane first, but current blockers still include `LocalThreadStore`, `thread_store_from_config`, and `InMemoryThreadStore` references plus a transitive `codex-core` dependency violation. Release compile and the boundary canary must pass before this can stand alone.

Path caveat: `codex-rs/core/src/session/**` and `codex-rs/app-server/Cargo.toml` are shared with other refactor lanes. If root wants a smaller thread-only commit, those files may require hunk-level separation after the compile blockers are resolved.

### 5. App Catalog, Connectors, And Extension Contributor Wiring

Purpose: move app catalog data through app-owned types/providers and update connector/extension surfaces.

Exact pathspecs:

```text
codex-rs/app/app-catalog-api/**
codex-rs/app/app-catalog-types/**
codex-rs/app-server/src/app_catalog_protocol.rs
codex-rs/app-server/src/request_processors/apps_processor.rs
codex-rs/app-server/src/request_processors.rs
codex-rs/app-server/src/lib.rs
codex-rs/connectors/**
codex-rs/core/src/apps/render.rs
codex-rs/core/src/connectors.rs
codex-rs/ext/extension-api/src/contributors.rs
codex-rs/ext/extension-api/src/contributors/tools.rs
codex-rs/ext/extension-api/src/lib.rs
codex-rs/ext/goal/src/extension.rs
codex-rs/ext/goal/src/tool.rs
codex-rs/ext/guardian/Cargo.toml
codex-rs/ext/guardian/src/lib.rs
codex-rs/ext/memories/src/extension.rs
codex-rs/ext/memories/src/tests.rs
codex-rs/ext/memories/src/tools/list.rs
codex-rs/ext/memories/src/tools/mod.rs
codex-rs/ext/memories/src/tools/read.rs
codex-rs/ext/memories/src/tools/search.rs
```

Verification state: not commit-ready. This depends on workspace manifest wiring and overlaps `codex-rs/core`/`codex-rs/app-server` files used by other lanes. No app-server or extension release tests are green for this state.

### 6. Tool Discovery, Plugin Install, And DAB Handler Surface

Purpose: group the plugin/tool symbol cleanup and internal tool handler changes.

Exact pathspecs:

```text
codex-rs/tools/**
codex-rs/tools-domain/**
codex-rs/core/src/tools/**
codex-rs/core/src/mcp_tool_call.rs
codex-rs/mcp-server/Cargo.toml
codex-rs/mcp-server/src/message_processor.rs
codex-rs/app-server/src/request_processors/plugins.rs
```

Verification state: not commit-ready. `compile_plugin_tool_scout` and `compile_hook_skill_scout` describe unresolved compile blockers around plugin install/list tool symbols, hook runtime, skill dependency exports, and protocol permission glob symbols. Keep this lane unstaged until those blockers are fixed and the focused release lane passes.

Path caveat: `codex-rs/core/src/tools/**` is broad and may include unrelated tool edits. Use the exact file set from `git diff --name-only -- codex-rs/core/src/tools/ codex-rs/tools/` before staging.

### 7. Protocol, Permission, Config, And Policy Support

Purpose: group supporting API/config/policy changes that feed the refactor and app-server protocol.

Exact pathspecs:

```text
codex-rs/app-server-protocol/src/protocol/v2/permissions.rs
codex-rs/config-types/src/lib.rs
codex-rs/config/src/lib.rs
codex-rs/config/src/state.rs
codex-rs/context-reduction/src/lib.rs
codex-rs/features/src/legacy.rs
codex-rs/permission-types/src/lib.rs
codex-rs/protocol/src/models.rs
codex-rs/protocol/src/protocol.rs
codex-rs/sandboxing/src/policy_transforms.rs
```

Verification state: not commit-ready. This includes app-server protocol shape changes and config changes; the corresponding schema generation/tests have not been completed, and release compile is still blocked by broader refactor issues.

### 8. Aggregate Source Integration Fallback

Purpose: if path overlap makes smaller commits unsafe, use one verified source commit for the current SOLID integration state.

Exact pathspecs:

```text
codex-rs/Cargo.toml
codex-rs/Cargo.lock
codex-rs/adapters/README.md
codex-rs/app/**
codex-rs/app-server-protocol/src/protocol/v2/permissions.rs
codex-rs/app-server/**
codex-rs/config-types/**
codex-rs/config/**
codex-rs/connectors/**
codex-rs/context-domain/**
codex-rs/context-reduction/**
codex-rs/core-api/**
codex-rs/core-domain/**
codex-rs/core/**
codex-rs/ext/**
codex-rs/features/src/legacy.rs
codex-rs/mcp/elicitation-api/**
codex-rs/mcp-server/**
codex-rs/permission-types/**
codex-rs/protocol/**
codex-rs/runtime-domain/**
codex-rs/sandboxing/**
codex-rs/session/**
codex-rs/state/migrations/0032_stage1_outputs_metadata.sql
codex-rs/state/migrations/0035_stage1_outputs_metadata.sql
codex-rs/thread/**
codex-rs/thread-manager-sample/**
codex-rs/thread-store/**
codex-rs/tools/**
codex-rs/tools-domain/**
codex-rs/turn/**
```

Verification state: not commit-ready now, but this is the safest path-scoped source grouping after the release build is green because `codex-rs/core/**`, `codex-rs/app-server/**`, and root manifests are currently shared by several lanes.

Minimum verification before committing this aggregate source group:

- `just fmt` in `codex-rs`.
- Focused release test/build lane from `verification_strategy_scout`.
- Boundary canary rerun with accepted expected result.
- App-server protocol schema regeneration/tests if the permission API change remains.
- `just fix -p codex-core` if the final source slice remains large.

## Files That Must Remain Unstaged

Always keep these unstaged for code commits:

```text
.codex/workflow-batch/**
.codex/workflow/.tmp/**
```

Keep these unstaged unless committing the workflow/docs group:

```text
.codex/workflow/**
.codex/prototypes/**
docs/**
```

Keep these unstaged for any non-root worker/lane commit:

```text
codex-rs/Cargo.toml
codex-rs/Cargo.lock
MODULE.bazel.lock
**/BUILD.bazel
```

When committing any one source group, leave every other source group unstaged. The current tree has overlapping files, so root should prefer explicit pathspec staging followed by `git diff --cached --name-only`; do not use `git add .`.
