# dirty_tree_ownership_mapper_worker Handoff

Status: complete read-only ownership map. No code review, build, test, schema,
commit, or subagent work was done. The only intended repo output is this
handoff.

## Current Dirty Shape

- Snapshot from `git status --porcelain=v1 --untracked-files=all`: 273 paths.
- Modified: workflow 1, app-server 15, app-server-protocol 1, codex-api 4,
  config/config-types/connectors 12, core 80, docs 1, ext 13, tools 6, plus
  smaller protocol/mcp-server/otel/feature/permission/sandbox/sample paths.
- Deleted: `codex-rs/core/tests/all.rs`,
  `codex-rs/core/tests/suite/mod.rs`, `codex-rs/state/migrations/0032_*`,
  and old `codex-rs/thread-store/**`.
- Untracked: two worker handoffs, `codex-rs/app-server/src/app_catalog_protocol.rs`,
  many new boundary crates under `app/`, `context-domain/`, `core-domain/`,
  `mcp/`, `runtime-domain/`, `session/`, `thread/`, `tools-domain/`, `turn/`,
  plus `docs/SELF_REVIEW_2026-05-20.md` and
  `docs/current-project-architecture.md`.

## Grouped Dirty Path Ownership

- Root/workflow:
  - `.codex/workflow/solid-refactor-handoff.md` is root orchestration state.
  - `.codex/workflow/agents/codex_otel_compile_followup_worker.handoff.md`
    owns the `codex-rs/otel/src/events/session_telemetry.rs` compile blocker
    follow-up note.
  - `.codex/workflow/agents/config_connectors_boundary_worker.handoff.md`
    owns the connector/config slice handoff.

- App-server, app catalog, and app-server protocol:
  - Explained by `app_server_boundary_rescue`,
    `app_server_boundary_scout`, `app_catalog_followup`,
    `thread_projection_boundary`, and `thread_store_integration_rescue`.
  - Current app-server dirty paths span processor routing, thread read/unarchive
    tests, config/external-agent processors, MCP refresh/message processor, and
    new `app_catalog_protocol.rs`; these are overlapping lanes, not one clean
    owner.
  - `codex-rs/app/app-catalog-api/**` and
    `codex-rs/app/app-catalog-types/**` belong with the app-catalog boundary
    lane and manifest integration, not generic app-server cleanup.

- Config/connectors:
  - Explained by `config_connectors_boundary_worker` and
    `config_provenance_boundary_worker`.
  - Main owned paths: `codex-rs/config/src/state.rs`,
    `codex-rs/config/src/loader/mod.rs`, `codex-rs/config/src/lib.rs`,
    `codex-rs/connectors/src/lib.rs`, plus related config/core call sites.
  - Manifest/import changes are shared with the root manifest boundary owner.

- Core compile and tool-router lanes:
  - Explained by `core_compile_tools_worker`,
    `core_compile_session_thread_worker`, `plugin_tool_compile_worker`,
    `skill_dependency_compile_worker`, and older boundary scouts.
  - Dirty `codex-rs/core/src/tools/**` is especially overlapped between
    tool-router, plugin install, skill dependency, and extension/tool
    contributor work. Do not hand this whole tree to one reconciliation worker.
  - Session/thread files under `codex-rs/core/src/session/**`,
    `thread_manager*`, `codex_thread.rs`, and `prompt_debug.rs` tie into the
    thread-store relocation lane.

- Core test split:
  - Explained by the `core_tests_*_lane_worker` handoffs and
    `core_tests_residual_router_worker`.
  - Deleted `codex-rs/core/tests/all.rs` and
    `codex-rs/core/tests/suite/mod.rs` are part of the split-router cleanup,
    not orphan deletes.

- Thread store/thread projection:
  - Explained by `thread_store_boundary`,
    `thread_store_api_recording_repair_worker`,
    `thread_store_integration_scout`, `thread_store_integration_rescue`, and
    `thread_projection_boundary`.
  - Old `codex-rs/thread-store/**` deletes pair with new
    `codex-rs/thread/thread-store-api/**` and
    `codex-rs/thread/thread-store/**` adds. Treat as a move/relocation group.
  - Thread projection API and app-server thread-data paths overlap; commit only
    after manifest/Bazel/root wiring is reconciled.

- Boundary/domain crate scaffolding:
  - Mostly explained at the strategy level by `manifest_wiring_scout`,
    `boundary_dependency_manifest_worker`, `bazel_lock_*`, `auth_boundary`,
    `mcp_elicitation_boundary`, and dependency-map scouts.
  - New crates under `context-domain/**`, `runtime-domain/**`, `session/**`,
    `turn/**`, `tools-domain/**`, and `core-domain/types/**` do not all have a
    fresh per-crate writer handoff in the current wave. Treat ownership as stale
    until root assigns exact crate families.

- Docs:
  - `docs/current-project-architecture-solid-refactor-plan.md`,
    `docs/current-project-architecture.md`, and
    `docs/SELF_REVIEW_2026-05-20.md` are architecture/review artifacts. Keep
    separate from source commits unless root intentionally creates a docs
    checkpoint.

## Orphaned Or Stale Ownership

- No obvious orphan in the old `thread-store/**` deletes; they line up with the
  new nested `thread/thread-store*` crates.
- Stale/no-current-wave owner: broad untracked domain crate scaffolding
  (`context-domain`, most `runtime-domain`, most `session`, `turn`,
  `tools-domain`, `adapters/README.md`) should not be swept into a source
  reconciliation commit without root marking an owner.
- Stale scout-only ownership: `commit_group_scout` predates the current 273-path
  tree and should be treated as guidance only, not current staging authority.

## Ignored Scratch Cleanup Notes

- Ignored prompt/marker/report scratch is present under
  `.codex/workflow/agents/`: `*.prompt.md`, `*.exec.marker.txt`, and
  `*.reads.report.json`.
- Keep ignored during mapping. Root can delete later if doing scratch cleanup.
- Untracked `*.handoff.md` files are not scratch; they should be folded into
  orchestration before cleanup.

## Overlap Risks

- Block a generic reconciliation worker over `codex-rs/core/src/**` or
  `codex-rs/app-server/src/**`; both are multi-owner areas with live overlap.
- Block broad staging or `git add .`; the tree mixes committed handoff lanes,
  stale scouts, new crate scaffolds, moves/deletes, and docs.
- Block thread-store reconciliation until root handles manifest/Bazel lock
  ownership and pairs old deletes with new `thread/thread-store*` adds.
- Block app-server reconciliation until app-catalog, thread projection, config,
  and thread-store app-server hunks are separated or explicitly co-owned.

## Suggested Next Worker Sequencing

1. Root folds the two untracked handoffs: `codex_otel_compile_followup_worker`
   and `config_connectors_boundary_worker`.
2. Assign narrow reconcilers by family, not by broad directory:
   thread-store relocation, app-catalog/app-server boundary, config/connectors,
   core tools/plugin/skill, core test split, then remaining domain crate
   scaffolding.
3. After endgame verification, consider coherent commits in this order:
   workflow/handoffs, codex-otel compile unblocker, config/connectors,
   app-catalog/app-server boundary, thread-store relocation/projection, core
   tools/plugin/skill, core test split, domain crate/manifest wiring, docs.
