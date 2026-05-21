# SOLID Refactor Handoff

Date: 2026-05-20

## Latest Update

### 2026-05-21T03:13+03:00 no-build resume wave launched

- Root verified the current orchestration state after interruption:
  - `codex_otel_compile_followup_worker` finished a source fix in
    `codex-rs/otel/src/events/session_telemetry.rs`; release verification was
    interrupted/deferred and no commit was created.
  - `config_connectors_boundary_worker` finished its source refactor slice in
    `codex-rs/config/**` and `codex-rs/connectors/**`; no commit was created
    because verification and ownership grouping are still deferred.
  - `app_server_boundary_finish_worker` was interrupted during an app-server
    release check/build and wrote no handoff. Treat it as stale, not active.
  - `core_protocol_dependency_followup_worker` has a prompt/marker but no live
    process and no handoff. Treat it as stale, not active.
- Root launched the next refactor-first/no-build wave:
  `app_server_boundary_resume_worker`,
  `core_protocol_dependency_resume_worker`, and
  `dirty_tree_ownership_mapper_worker`.
- All next-wave workers are instructed not to run `cargo`, `rustc`, `just`,
  build scripts, tests, schema generation, or subagents. If they need
  verification they must record exact commands for root/endgame and stop.
- Root should monitor for accidental build processes and stop only that build
  tree; useful refactor workers should continue when possible.
- Additional safe parallelism after launch:
  `scratch_cleanup_inventory_worker`,
  `manifest_dependency_inventory_worker`, and
  `verification_matrix_planner_worker`.
  These are read-only/no-build workers. They should not edit code, manifests, or
  handoffs other than their own handoff files. They exist to prepare cleanup,
  dependency ownership, and endgame verification sequencing while the edit
  workers run.
- Delegated review follow-up:
  `orchestration_artifacts_review_worker` found two prompt path defects after
  launch. Root fixed them in the prompt files:
  - `app_server_boundary_resume_worker.prompt.md` now points at
    `codex-rs/app-server-protocol/src/protocol/v2/mod.rs` plus relevant split
    v2 modules instead of missing `protocol/v2.rs`.
  - `verification_matrix_planner_worker.prompt.md` now points at root
    `justfile` instead of missing `codex-rs/justfile`.
  The affected sessions were launched before these prompt-file fixes; do not
  duplicate the app-server edit worker unless it reports that the stale first
  read blocked progress.
- Completed handoffs received from this wave:
  - `app_server_boundary_resume_worker`: source-only resume completed in
    app-server/app-server-protocol ownership. It handled the stale missing
    `protocol/v2.rs` prompt path by using the split v2 module layout. No
    outside-owner blocker was found. Verification, schema refresh, and root
    lockfile updates remain deferred to endgame.
  - `dirty_tree_ownership_mapper_worker`: read-only dirty-tree ownership map
    completed. It groups the moving tree by family and recommends narrow
    reconcilers by ownership family, not broad directory. Suggested eventual
    commit order: workflow/handoffs, codex-otel compile unblocker,
    config/connectors, app-catalog/app-server boundary,
    thread-store relocation/projection, core tools/plugin/skill, core test
    split, domain crate/manifest wiring, docs.
  - `scratch_cleanup_inventory_worker`: read-only artifact inventory completed.
    `.gitignore` already covers active prompt/marker/report scratch patterns;
    root should keep durable `*.handoff.md` files and delete stale scratch only
    after corresponding handoffs are preserved.
  - `verification_matrix_planner_worker`: read-only deferred verification
    matrix completed. It did not run build/test/schema/lock commands. Use it
    after refactor source ownership is clean to stage release-only verification,
    schema/lock updates, final build/deploy, and compaction validation.
  - `manifest_dependency_inventory_worker`: read-only manifest/dependency
    ownership inventory completed. It reports root workspace manifests and
    lockfiles currently clean, identifies changed package manifests as owned by
    their source-family workers, and defers lock/Bazel refresh until the final
    manifest/BUILD set is stable. It also flags that source imports of
    app-server protocol DTOs from `codex-core` still need to be resolved rather
    than hidden by adding app-server-protocol as a core dependency.
- Still waiting for handoffs:
  `core_protocol_dependency_resume_worker`.
- Duplicate manifest retry cleanup: root briefly launched
  `manifest_dependency_inventory_retry_worker` after suspecting the original
  died. The original produced
  `.codex/workflow/agents/manifest_dependency_inventory_worker.handoff.md`, so
  root stopped the retry process tree, closed the completed original wrapper,
  and deleted only the retry marker scratch file. No build processes were
  active during the cleanup.
- Follow-up duplicate cleanup correction: an orphaned retry child chain later
  started an accidental no-build-phase Cargo lane:
  `cargo test -p codex-core --release -j 1 --test compac...`.
  Root traced it to the stale retry Codex process, stopped the Cargo/rustc
  build and the duplicate retry Codex process, and rechecked that no `cargo`,
  `rustc`, `just`, linker, or compiler processes remained.

### 2026-05-21T02:46+03:00 config/connectors boundary handoff integration

- Received `.codex/workflow/agents/config_connectors_boundary_worker.handoff.md`.
  The worker completed its owned source refactor slice but did not create a
  commit because focused verification was blocked by an active Cargo lane,
  existing unrelated dirty owned paths, and a manifest-owned connector
  dependency change.
- Worker-owned source changes are in `codex-rs/config/src/state.rs`,
  `codex-rs/config/src/loader/mod.rs`, `codex-rs/config/src/lib.rs`, and
  `codex-rs/connectors/src/lib.rs`: user config layer identity is now carried
  by `UserConfigLayerSource`, and connector directory fetching now has
  `ConnectorDirectoryFetchPolicy` plus
  `list_all_connectors_with_fetch_policy`.
- Verification reported by the handoff: `cargo fmt -p codex-config -p
  codex-connectors` ran, and `git diff --check -- codex-rs/config
  codex-rs/config-types codex-rs/connectors` passed with only CRLF warnings.
  Focused `codex-config` / `codex-connectors` release tests were not started
  because `cargo test -p codex-app-server-protocol --release -j 1` was still
  active.
- Next owners from the handoff: root/build owner should run focused release
  tests for `codex-config` and `codex-connectors` after the active lane
  finishes; manifest/root owner must decide whether the dirty connector
  manifest and `codex_app_catalog_types` import move belong in the same
  connector boundary slice; app-server/chatgpt owners should migrate remaining
  external `list_all_connectors_with_options(..., force_refetch, ...)`
  callsites to the enum-based API.

### 2026-05-21T02:43+03:00 request permissions gate integration

- Integrated `request_permissions_gate_worker`: handoff reports complete and
  commit `62663cc4ed` (`solid-refactor: restore request permissions platform
  gate`) restored the non-Windows cfg on
  `codex-rs/core/tests/permissions.rs` for `suite/request_permissions.rs`.
- Focused `codex-core` release verification remains deferred as reported by the
  worker handoff: the test was not run because another live build was active and
  free disk was low.
- Remaining marker-only/active review follow-ups are
  `core_protocol_dependency_followup_worker` and
  `codex_otel_compile_followup_worker`; do not duplicate those lanes until
  their handoffs appear or root confirms they are stale.

### 2026-05-21T01:54+03:00 root orchestration checkpoint

- Integrated completed residual core test routing:
  `core_tests_residual_router_worker` finished and committed `d0a3390511`
  (`Route residual core integration tests`). `codex-rs/core/tests/all.rs`
  and `codex-rs/core/tests/suite/mod.rs` are deleted; residual modules are
  routed into split binaries including the new `telemetry` lane.
- Integrated completed manifest/dependency boundary repair:
  `boundary_dependency_manifest_worker` finished and committed
  `ed932df956` (`Wire boundary crates in workspace manifests`), then recorded
  handoff commit `a0ad874831` (`Record boundary dependency manifest handoff`).
- Integrated completed core config/permissions compile repair:
  `core_compile_config_permissions_worker` committed `72245564ff`
  (`Fix config permission project root glob helper`). Its focused release
  `cargo check -p codex-core` progressed past the owned config/permissions
  blocker and then stopped on a downstream `codex-otel` compile error recorded
  in `logs/core-config-cargo-check-release-20260521-015907.log`.
- `core_compile_session_thread_worker` wrote a handoff with uncommitted,
  formatted session/thread changes. It reports `git diff --check` clean, but no
  commit because the focused `cargo check -p codex-core --release --lib`
  stopped first in the out-of-lane `codex-otel`
  `ResponseEvent::Incomplete` exhaustive-match error
  (`logs/codex-core-lib-release-check-20260521-021630.log`).
- `core_compile_tools_worker` wrote a handoff with uncommitted tool-router and
  spec-plan changes. It reports `just fmt`, static `ToolExecutor` scan, and
  `git diff --check` clean, but no commit because the same `codex-otel`
  compile blocker stopped focused release verification
  (`logs/codex-core-tools-router-release-check-20260521-021708.log`).
- Existing external sessions with markers but no handoff files yet:
  `app_server_boundary_finish_worker`,
  `core_protocol_dependency_followup_worker`, and
  `codex_otel_compile_followup_worker`.
  Treat those lanes as active/unknown and do not duplicate their ownership until
  handoffs appear or root confirms they are stale.
- Root launched only disjoint follow-up edit lanes at
  2026-05-21T01:57+03:00:
  `app_server_boundary_finish_worker`,
  `config_connectors_boundary_worker`, and
  `compaction_output_plan_worker`.
- Workflow scratch cleanup: `.codex/workflow/agents/*.tmp.txt` is now
  gitignored after an active core-tools worker wrote
  `core_compile_tools_worker_missing_output.tmp.txt`; root left the file in
  place because that lane is still active/unknown.
- `compaction_output_plan_worker` finished quickly. It kept and normalized
  `.codex/workflow/compaction-max-output-plan.md` as a durable workflow plan,
  committed it as `556654f05d` (`solid-refactor: record compaction max output
  plan`), then recorded its handoff as `4998853d02`
  (`solid-refactor: record compaction output plan handoff`).
- `recent_worker_review_worker` completed delegated review and committed
  `0a465f27b3` (`Record recent worker review handoff`). It flagged three
  follow-ups: restore/resolve direct `codex-app-server-protocol` ownership for
  `codex-core`, restore the old non-Windows gate around
  `request_permissions.rs`, and remove stale residual-router queue language
  from this handoff.
- Root is launching those review follow-ups as disjoint external workers:
  `core_protocol_dependency_followup_worker`,
  `request_permissions_gate_worker`, and
  `codex_otel_compile_followup_worker` at 2026-05-21T02:18+03:00. They
  inherit the same no-build/test policy until their owned refactor is complete,
  then only focused prompt-authorized verification is allowed.
- No broad builds/tests should run from root. Workers must not build or run
  tests until their owned refactor is complete; after that, only focused
  prompt-authorized verification is allowed. Workers should commit coherent
  scoped changes when safe and write concise `*.handoff.md` files.
- Compact early: update this file again around 40-45% context and compact
  before 50%.

- `core_tests_agents_lane_worker` moved the agents/delegation integration-test
  modules from `codex-rs/core/tests/suite/` into
  `codex-rs/core/tests/agents/` and simplified `codex-rs/core/tests/agents.rs`
  to use normal sibling module declarations.
- Worker handoff:
  `.codex/workflow/agents/core_tests_agents_lane_worker.handoff.md`.
- Focused release verification for `codex-core --test agents` is blocked before
  the integration target compiles by unrelated shared `codex-core` library
  errors from other in-flight refactor lanes; see
  `logs/test-local-release-codex-core-all-20260521-003300.log`.

## Current User Intent

- Implement the SOLID/clean-architecture refactor in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.
- Split aggressively: many small crates, each with one entity or responsibility.
- Dependencies must point to abstractions/ports, not direct or transitive concrete implementations.
- Adjacent crates are allowed when they reduce coupling with adjacent areas, but the reason must be explicit and root should review/wire them.
- Commits are desired at logical implementation points, but Git currently refuses partial commits because a merge is in progress.
- User accepts temporary compilation breakage while boundaries are being established.
- Skip Bazel lock refresh and broad verification until later refactor stages.

## Important Constraints

- Root owns `codex-rs/Cargo.toml`, lockfiles, Bazel files, Git state, staging, commits, formatting, and broad verification.
- Workers may create/edit only assigned paths.
- Workers must not create nested workspaces, `Cargo.lock`, `target/`, or path dependencies between sibling workspace crates.
- Workers may create adjacent coupling-reduction crates only when they explain the architectural reason in their handoff.
- No compatibility re-export shims to hide old imports.
- Do not start the app-server protocol cleanup with MCP elicitation types, `ThreadHistoryBuilder`, or `TurnStatus`; those need a separate boundary review.

## Live Agents At Handoff

- `/root/config_provenance_worker`: focused on replacing `codex_app_server_protocol::ConfigLayerSource` imports in core with the domain owner (`codex_config_types::ConfigLayerSource`) where direct.
- `/root/solid_overseer`: read-only SOLID reviewer. It should flag concrete dependency leaks, nested workspaces, lockfiles, target dirs, root manifest edits by workers, path deps, overbroad crates, and adjacent crates without stated coupling-reduction rationale.

## Files And Slices Already Started

- Added `docs/current-project-architecture-solid-refactor-plan.md`.
- Added/updated `.codex/workflow/solid-refactor-subagent-contract.md`.
- Added this handoff file.
- Replaced `.codex/prototypes/check-core-boundaries.ps1` with a stronger static boundary canary:
  - source import checks;
  - direct and transitive local dependency checks from workspace manifests;
  - protected crate rules for core and new domain/API folders.
- Added `codex-rs/adapters/README.md`.
- Moved thread store crates structurally:
  - old `codex-rs/thread-store-api` -> `codex-rs/thread/thread-store-api`;
  - old `codex-rs/thread-store` -> `codex-rs/thread/thread-store`;
  - root `codex-rs/Cargo.toml` workspace paths were updated for these two.
- Added root-owned thread skeleton crates:
  - `codex-rs/thread/thread-api`;
  - `codex-rs/thread/thread-handle-api`;
  - `codex-rs/thread/thread-manager-api`;
  - root `codex-rs/Cargo.toml` is wired for these.
- Session worker created and root wired:
  - `session/session-api`;
  - `session/session-events`;
  - `session/session-factory`;
  - `session/session-input`;
  - `session/session-policy`;
  - `session/session-runtime`;
  - `session/session-runtime-api`;
  - `session/session-state`.
- Turn worker created and root wired:
  - `turn/turn-api`;
  - `turn/turn-events`;
  - `turn/turn-loop`;
  - `turn/turn-loop-api`;
  - `turn/turn-policy`;
  - `turn/turn-state`;
  - `turn/turn-tool-bridge`.
  - Root removed the worker-created nested `codex-rs/turn/Cargo.toml` and `codex-rs/turn/Cargo.lock`.
  - Root converted turn sibling dependencies from local `path = "../..."` to `{ workspace = true }`.
- Domain worker created and root wired:
  - `core-domain/types`;
  - `context-domain/compaction-policy`;
  - `context-domain/context-budget`;
  - `context-domain/history-api`;
  - `context-domain/prompt-context`;
  - `tools-domain/tool-execution-api`;
  - `tools-domain/tool-handler-api`;
  - `tools-domain/tool-registry-api`;
  - `runtime-domain/auth-api`;
  - `runtime-domain/model-client-api`;
  - `runtime-domain/runtime-ports`;
  - `runtime-domain/state-db-api`;
  - `runtime-domain/telemetry-api`.

## Current Known Issues

- Git commit attempts fail with `fatal: cannot do a partial commit during a merge`.
- There are many pre-existing dirty and unmerged files. Stage only files from the active slice when merge state permits.
- The boundary canary currently fails by design because core still has forbidden references:
  - concrete thread-store symbols in core tests/helpers;
  - `codex_app_server_protocol` imports in core;
  - `codex-core` still directly/transitively depends on forbidden outer crates.
- Domain crate workspace wiring was completed after the first failed patch by using the current manifest shape.
- The domain worker reported possible adjacent `worker-api`/`worker-context` crates, but they were not present on disk when root checked. Do not assume they exist.

## Current Crate Folder Inventory

- `codex-rs/core-domain/types`
- `codex-rs/context-domain/compaction-policy`
- `codex-rs/context-domain/context-budget`
- `codex-rs/context-domain/history-api`
- `codex-rs/context-domain/prompt-context`
- `codex-rs/tools-domain/tool-execution-api`
- `codex-rs/tools-domain/tool-handler-api`
- `codex-rs/tools-domain/tool-registry-api`
- `codex-rs/runtime-domain/auth-api`
- `codex-rs/runtime-domain/model-client-api`
- `codex-rs/runtime-domain/runtime-ports`
- `codex-rs/runtime-domain/state-db-api`
- `codex-rs/runtime-domain/telemetry-api`
- `codex-rs/session/session-*` as listed above
- `codex-rs/turn/turn-*` as listed above
- `codex-rs/thread/thread-api`
- `codex-rs/thread/thread-handle-api`
- `codex-rs/thread/thread-manager-api`
- `codex-rs/thread/thread-store-api`
- `codex-rs/thread/thread-store`

## Recommended Next Steps

1. Wait briefly for `/root/config_provenance_worker` and `/root/solid_overseer` handoffs.
2. Re-run cheap static scans:
   - forbidden import scan over new crates;
   - nested workspace/lockfile/target scan under new folder roots;
   - `.codex/prototypes/check-core-boundaries.ps1` to confirm expected violations only.
3. Apply the config provenance cleanup from the worker if it is clean:
   - replace core `ConfigLayerSource` imports from app-server protocol with `codex_config_types`;
   - avoid MCP elicitation, `ThreadHistoryBuilder`, `TurnStatus`, and app catalog metadata in that slice.
4. Integrate the core thread-store leak worker handoff if it is clean:
   - core must not import `codex_thread_store::`, `LocalThreadStore`, `InMemoryThreadStore`, or `thread_store_from_config`;
   - use API-only unsupported/fake stores where tests need placeholders.
5. Run `just fmt` only after Rust edit batches settle.
6. Do not run `just bazel-lock-update`, `just bazel-lock-check`, or broad Cargo checks yet.

## Compaction Note

The root agent cannot directly compact itself with the available tools. `/clear`
is a client-side reset and should be assumed unsafe for live subagent continuity
unless the client explicitly guarantees otherwise. This handoff is the durable
state to resume from after compaction, interruption, or a new session.

## 2026-05-20 Continuation: Director Plan Saved

- Saved the live delegation plan at
  `.codex/workflow/solid-refactor-delegation-director-plan.md`.
- The current root session is the director/integrator. Root owns manifests,
  lockfiles, Bazel files, Git state, merge-state repair, formatting, and broad
  verification.
- Built-in helper capacity is limited, so external Codex sessions in PowerShell
  tabs are allowed by the user for additional parallelism. They must communicate
  through `.codex/workflow/agents/` prompt and handoff files.
- Workers may create adjacent crates only when the crate has one clear
  responsibility and reduces coupling with the assigned adjacent area. They must
  not create unrelated neighboring crates or wire root-owned manifests unless
  root explicitly grants that work.
- Compaction is expected soon. On resume, first read this handoff and the
  delegation director plan, then inspect `.codex/workflow/agents/` for active
  worker notes before editing code.

## 2026-05-20 Continuation: External Codex Launcher Verified

Compaction is still expected soon. On resume, preserve this order:

1. Read this handoff.
2. Read `.codex/workflow/solid-refactor-delegation-director-plan.md`.
3. Inspect `.codex/workflow/agents/*.handoff.md` and `*.marker.txt`.
4. Use the verified launcher scripts instead of inline `wt.exe` prompt strings.

New launcher files:

- `.codex/workflow/scripts/Start-CodexWorker.ps1`
- `.codex/workflow/scripts/Invoke-CodexWorker.ps1`
- `.codex/workflow/agents/launcher_canary.prompt.md`
- `.codex/workflow/agents/dab_availability_worker.prompt.md`
- `.codex/workflow/agents/dab_availability_worker.handoff.md`

Verified launcher commands:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\scripts\Start-CodexWorker.ps1 -PromptFile .codex\workflow\agents\launcher_canary.prompt.md -Mode Version -Title codex-launcher-canary -MarkerFile .codex\workflow\agents\launcher_canary.tab.marker.txt
powershell -ExecutionPolicy Bypass -File .codex\workflow\scripts\Start-CodexWorker.ps1 -PromptFile .codex\workflow\agents\launcher_canary.prompt.md -Mode Exec -CurrentWindow -MarkerFile .codex\workflow\agents\launcher_canary.exec.marker.txt
powershell -ExecutionPolicy Bypass -File .codex\workflow\scripts\Start-CodexWorker.ps1 -PromptFile .codex\workflow\agents\dab_availability_worker.prompt.md -Mode Interactive -Title codex-dab-availability-worker -MarkerFile .codex\workflow\agents\dab_availability_worker.marker.txt
```

Verification results:

- New-tab launcher canary passed. Marker:
  `.codex/workflow/agents/launcher_canary.tab.marker.txt` contains
  `completed mode=Version exit=0`.
- `codex exec` canary passed in the current window and returned
  `LAUNCHER_CANARY_OK`. Marker:
  `.codex/workflow/agents/launcher_canary.exec.marker.txt`.
- DAB helper interactive tab launched. Marker:
  `.codex/workflow/agents/dab_availability_worker.marker.txt`.

Current DAB availability finding:

- Direct `dab_*` tools are not exposed in this root session even though a
  developer message says native desktop automation is available.
- `tool_search` did not reveal deferred DAB tools.
- Internal Codex DAB code exists in:
  - `codex-rs/desktop-automation/src/lib.rs`
  - `codex-rs/desktop-automation/src/windows.rs`
  - `codex-rs/tools/src/desktop_automation.rs`
  - `codex-rs/core/src/tools/handlers/desktop_automation.rs`
- `codex-rs/tools/src/tool_registry_plan.rs` appears to register
  model-visible DAB specs when `desktop_automation_enabled` is true.
- The likely missing edge is the core executor registry:
  `DesktopAutomationHandler` exists, but `codex-rs/core/src/tools/spec_plan.rs`
  did not appear to register it in `collect_tool_executors`, and
  `codex-rs/core/src/tools/handlers/mod.rs` did not appear to re-export it.
- The launched `dab_availability_worker` owns only the internal DAB tool
  exposure lane. Root should avoid editing the same DAB files until that worker
  writes its handoff or the session is declared stale.

Do not rely on external Wizard_Erasmus DAB for the fix. The user explicitly
asked to fix/use internal Codex DAB availability.

## 2026-05-20 Continuation: Expanded Worker Launch Plan

Root updated `.codex/workflow/worker-delegation-commit-protocol.md` and the
worker prompts so workers may delegate bounded helper tasks and commit verified
owned slices with path-scoped staging. Root still owns manifests, lockfiles,
Bazel, merge state, pushes, and final aggregate commits.

Launch set:

1. `canary_observer` read-only boundary scan.
2. `auth_boundary` auth API ownership lane.
3. `thread_store_boundary` thread store API ownership lane.
4. `thread_projection_boundary` thread projection type/API lane.
5. `mcp_elicitation_boundary` MCP elicitation type/API lane.
6. `app_catalog_followup` app catalog protocol leak follow-up lane.
7. `dab_availability_worker` internal DAB availability relaunch.

The original DAB interactive marker only recorded `starting`, so root marked it
stale and queued an exec relaunch with a canary-first requirement. Workers should
write their handoff files and, when safe, path-scope their own commits.

Launch verification at 2026-05-20T19:01+03:00:

- `app_catalog_followup.exec.marker.txt`: running.
- `auth_boundary.exec.marker.txt`: running.
- `canary_observer.exec.marker.txt`: running.
- `dab_availability_worker.exec.marker.txt`: running.
- `mcp_elicitation_boundary.exec.marker.txt`: running.
- `thread_projection_boundary.exec.marker.txt`: running.
- `thread_store_boundary.exec.marker.txt`: running.
- `git ls-files -u` returned no unmerged paths, so merge conflicts are
  currently resolved for path-scoped commits.

Canary observer result at 2026-05-20T19:15+03:00:

- `canary_observer.exec.marker.txt` completed with exit `0`.
- `.codex/workflow/agents/canary_observer.handoff.md` reports the boundary
  canary itself still exits `1` with 23 current violations: 0 direct forbidden
  crate dependencies, 1 transitive forbidden crate dependency, and 22
  source-pattern leaks.
- Highest-risk source patterns remain `codex_app_server_protocol::`,
  `LocalThreadStore`, `LocalThreadStoreConfig`, and
  `thread_store_from_config`.
- Other six worker markers are still running as of this note.

DAB oversight update:

- `.codex/workflow/agents/dab_availability_worker.handoff.md` now reports a
  scoped internal DAB fix in `codex-rs/core/src/tools/handlers/mod.rs`,
  `codex-rs/core/src/tools/spec_plan.rs`, and
  `codex-rs/core/src/tools/spec_plan_tests.rs`.
- Root ran `just fmt`, `git diff --check` for the DAB-touched files, and then
  attempted `scripts\test-local-codex-release.ps1 -Package codex-core -Filter
  desktop_automation -Lib -AllowBroadCoreLibUnitTests`.
- The release test still fails before reaching the DAB canary because of broad
  current `codex-core` compile blockers unrelated to the DAB patch:
  missing hook-runtime symbols, missing skill dependency exports, missing
  plugin-install tool symbols, `session.input_queue`,
  `Op::UserInput.thread_settings`, and `LocalThreadStore`.

Worker supervision update at 2026-05-20T19:46+03:00:

- Completed markers: `canary_observer`, `app_catalog_followup`,
  `auth_boundary`, `thread_projection_boundary`, `mcp_elicitation_boundary`,
  and `thread_store_boundary`.
- `dab_availability_worker.exec.marker.txt` later completed with exit `0` at
  2026-05-20T20:06:17+03:00; no Cargo/rustc/link process from that verification
  lane remains running.
- The worker prompts were corrected after launch: path-scoped Git
  staging/commits are now allowed under
  `.codex/workflow/worker-delegation-commit-protocol.md`; resets and checkouts
  remain forbidden.
- Completed SOLID worker handoffs mostly prepared owned crates or findings and
  left root-owned manifest/protocol wiring as the blocker. Root should integrate
  only one lane at a time, starting with the slice that removes the current
  compile blocker or boundary-canary violation with the smallest manifest
  surface.

Follow-up scout wave queued:

- `compile_hook_skill_scout`: read-only diagnosis for missing hook runtime and
  skill dependency symbols.
- `compile_plugin_tool_scout`: read-only diagnosis for missing plugin tool
  exports.
- `compile_session_store_scout`: read-only diagnosis for `session.input_queue`,
  `Op::UserInput.thread_settings`, and concrete thread-store blockers.

These are external `codex exec` sessions because built-in agent spawning is
blocked by stale registered helper threads. They should write handoff files only
and avoid Cargo/Just/Git.

Follow-up scout launch verification at 2026-05-20T20:06+03:00:

- `compile_hook_skill_scout.exec.marker.txt`: running.
- `compile_plugin_tool_scout.exec.marker.txt`: running.
- `compile_session_store_scout.exec.marker.txt`: running.
- `git ls-files -u` returned no unmerged paths.

Current rough progress estimate:

- Decomposition/orchestration: high; workers completed first-pass handoffs for
  auth, app catalog, MCP elicitation, thread projection, thread store, boundary
  canary, and DAB availability.
- Implementation/wiring: partial; multiple owned crates/slices are prepared,
  but root-owned manifest/protocol/core wiring remains.
- Verification: blocked; `codex-core` still has broad compile blockers and the
  boundary canary still reports 23 violations.
- Overall refactor estimate: roughly 30% complete for the full clean-architecture
  objective, with higher confidence in the map than in the final ETA.

## 2026-05-20 Compaction Prep: Additional Parallel Scouts

The first compile-scout wave is still running by marker as of the latest root
check, but no Cargo/rustc/link process is active. Root launched an additional
read-only coordination wave because the work is independent and writes only
per-scout handoff files:

- `integration_order_scout`: integrate-order plan from completed worker
  handoffs.
- `manifest_wiring_scout`: exact root manifest/workspace wiring required for
  prepared crates.
- `boundary_delta_scout`: current boundary-canary violation grouping and next
  highest-impact implementation slice.
- `commit_group_scout`: path-scoped commit grouping and files to keep unstaged.
- `verification_strategy_scout`: smallest release-only verification ladder after
  compile blockers are repaired.

All five are forbidden from editing source, running Cargo/Just/formatters, or
staging/committing. Their expected outputs are:

- `.codex/workflow/agents/integration_order_scout.handoff.md`
- `.codex/workflow/agents/manifest_wiring_scout.handoff.md`
- `.codex/workflow/agents/boundary_delta_scout.handoff.md`
- `.codex/workflow/agents/commit_group_scout.handoff.md`
- `.codex/workflow/agents/verification_strategy_scout.handoff.md`

## Additional Parallel Scout Batch

Updated: 2026-05-20T20:33:43+03:00

Root launched six more external `codex exec` sessions after the first scout
batch. These are deliberately read-only because the tree already has broad
dirty code changes and active compile-blocker scouts; they should not contend
with implementation ownership or commits. They may delegate focused read-only
questions internally if useful, but they must only write their own handoff file.

Launched sessions and markers:

- `core_dependency_map_scout`
  - prompt: `.codex/workflow/agents/core_dependency_map_scout.prompt.md`
  - handoff: `.codex/workflow/agents/core_dependency_map_scout.handoff.md`
  - marker: `.codex/workflow/agents/core_dependency_map_scout.exec.marker.txt`
- `app_server_boundary_scout`
  - prompt: `.codex/workflow/agents/app_server_boundary_scout.prompt.md`
  - handoff: `.codex/workflow/agents/app_server_boundary_scout.handoff.md`
  - marker: `.codex/workflow/agents/app_server_boundary_scout.exec.marker.txt`
- `protocol_schema_scout`
  - prompt: `.codex/workflow/agents/protocol_schema_scout.prompt.md`
  - handoff: `.codex/workflow/agents/protocol_schema_scout.handoff.md`
  - marker: `.codex/workflow/agents/protocol_schema_scout.exec.marker.txt`
- `bazel_lock_scout`
  - prompt: `.codex/workflow/agents/bazel_lock_scout.prompt.md`
  - handoff: `.codex/workflow/agents/bazel_lock_scout.handoff.md`
  - marker: `.codex/workflow/agents/bazel_lock_scout.exec.marker.txt`
- `test_surface_scout`
  - prompt: `.codex/workflow/agents/test_surface_scout.prompt.md`
  - handoff: `.codex/workflow/agents/test_surface_scout.handoff.md`
  - marker: `.codex/workflow/agents/test_surface_scout.exec.marker.txt`
- `thread_store_integration_scout`
  - prompt: `.codex/workflow/agents/thread_store_integration_scout.prompt.md`
  - handoff: `.codex/workflow/agents/thread_store_integration_scout.handoff.md`
  - marker: `.codex/workflow/agents/thread_store_integration_scout.exec.marker.txt`

Launch verification immediately after start:

- All six new marker files existed and contained `starting mode=Exec`.
- Process discovery found matching worker processes for all six prompts.

Compaction resume instruction:

1. Re-read this handoff.
2. Check all `.codex/workflow/agents/*.exec.marker.txt` statuses.
3. For any completed scouts, read their `.handoff.md` and integrate findings
   into the root plan before assigning more implementation ownership.
4. Only launch new implementation/editing sessions after the relevant scout
   handoffs are complete enough to define non-overlapping path ownership.
5. Commit coherent verified slices as they become green, following
   `.codex/workflow/worker-delegation-commit-protocol.md`.

## 2026-05-20 21:27 +03: Root Orchestration Update

### Marker Status Checked

Completed `exit=0` workers/scouts observed:

- `app_catalog_followup`
- `auth_boundary`
- `boundary_delta_scout`
- `canary_observer`
- `commit_group_scout`
- `compile_hook_skill_scout`
- `compile_plugin_tool_scout`
- `compile_session_store_scout`
- `core_dependency_map_scout`
- `dab_availability_worker`
- `integration_order_scout`
- `launcher_canary`
- `manifest_wiring_scout`
- `mcp_elicitation_boundary`
- `protocol_schema_scout`
- `test_surface_scout`
- `thread_projection_boundary`
- `thread_store_boundary`
- `verification_strategy_scout`

Hung or stale original scouts:

- `app_server_boundary_scout.exec.marker.txt`: started, no completion marker;
  handoff still only says queued.
- `bazel_lock_scout.exec.marker.txt`: started, no completion marker; handoff
  still only says queued.
- `thread_store_integration_scout.exec.marker.txt`: started, no completion
  marker; handoff still only says queued.

Those three have live/no-exit terminal wrapper processes, but no useful handoff
content. Treat them as stale and use the replacement sessions below.

### New Sessions Launched

All were launched via `.codex/workflow/scripts/Start-CodexWorker.ps1 -Mode Exec`
and had start markers written at about 21:25 +03. Check marker files first; if a
marker has no completed line, inspect the handoff timestamp/size before waiting.

- `app_server_boundary_rescue`
  - prompt: `.codex/workflow/agents/app_server_boundary_rescue.prompt.md`
  - handoff: `.codex/workflow/agents/app_server_boundary_rescue.handoff.md`
  - marker: `.codex/workflow/agents/app_server_boundary_rescue.exec.marker.txt`
  - read-only replacement for stale app-server boundary scout.
- `bazel_lock_rescue`
  - prompt: `.codex/workflow/agents/bazel_lock_rescue.prompt.md`
  - handoff: `.codex/workflow/agents/bazel_lock_rescue.handoff.md`
  - marker: `.codex/workflow/agents/bazel_lock_rescue.exec.marker.txt`
  - read-only replacement for stale Bazel/lock scout.
- `thread_store_integration_rescue`
  - prompt: `.codex/workflow/agents/thread_store_integration_rescue.prompt.md`
  - handoff: `.codex/workflow/agents/thread_store_integration_rescue.handoff.md`
  - marker: `.codex/workflow/agents/thread_store_integration_rescue.exec.marker.txt`
  - read-only replacement for stale thread-store integration scout.
- `plugin_tool_compile_worker`
  - prompt: `.codex/workflow/agents/plugin_tool_compile_worker.prompt.md`
  - handoff: `.codex/workflow/agents/plugin_tool_compile_worker.handoff.md`
  - marker: `.codex/workflow/agents/plugin_tool_compile_worker.exec.marker.txt`
  - edit-owned narrow compile blocker in plugin request-install handler/spec
    files and `core/src/tools/handlers/mod.rs`.
- `skill_dependency_compile_worker`
  - prompt: `.codex/workflow/agents/skill_dependency_compile_worker.prompt.md`
  - handoff: `.codex/workflow/agents/skill_dependency_compile_worker.handoff.md`
  - marker: `.codex/workflow/agents/skill_dependency_compile_worker.exec.marker.txt`
  - edit-owned narrow compile blocker for skill dependency exports/adapters.
- `hook_runtime_compile_scout`
  - prompt: `.codex/workflow/agents/hook_runtime_compile_scout.prompt.md`
  - handoff: `.codex/workflow/agents/hook_runtime_compile_scout.handoff.md`
  - marker: `.codex/workflow/agents/hook_runtime_compile_scout.exec.marker.txt`
  - read-only patch-plan scout for stale hook runtime callsites.
- `dab_internal_canary_scout`
  - prompt: `.codex/workflow/agents/dab_internal_canary_scout.prompt.md`
  - handoff: `.codex/workflow/agents/dab_internal_canary_scout.handoff.md`
  - marker: `.codex/workflow/agents/dab_internal_canary_scout.exec.marker.txt`
  - read-only review of the internal-DAB worker changes and later canary path.

### Current Practical State

- Estimated overall SOLID refactor progress remains about 30-35%. The design
  and crate slicing are now well mapped, but the tree is not compile-green.
- Main blockers are still compile blockers and static boundary leaks, not DAB
  design. DAB appears contained but cannot be verified until broader compile
  blockers clear.
- Do not start root manifest/lock/schema refresh until source compile blockers
  and replacement rescue handoffs clarify the exact crate wiring needed.
- Do not let multiple edit workers touch `codex-rs/core/src/session/turn.rs`,
  `codex-rs/core/src/session/mod.rs`, `codex-rs/core/src/thread_manager.rs`, or
  `codex-rs/core/tests/common/**` at the same time.

### Progress Estimate

- Overall progress: about 30-35% complete.
- Architecture discovery and decomposition: about 65-75% complete. The main
  boundaries, candidate crates, canary checks, and integration order are now
  documented by completed scouts.
- Source integration: about 20-30% complete. Several new crates and boundary
  adapters exist, but compile blockers still prevent a reliable release build.
- Verification/commit readiness: about 10-15% complete. Boundary canary still
  fails with 23 reported lines as of the latest root run, and no new coherent
  source slice is ready for root commit until the narrow compile blockers settle.

ETA depends heavily on whether the current dirty tree compiles after the narrow
plugin/skill/hook/session-store blockers are fixed:

- Best case: 1-2 focused work sessions to reach a first compile-green,
  partially verified checkpoint and commit the first coherent slices.
- Realistic case: 2-4 focused work sessions to reach compile-green, refresh
  manifest/schema/lock artifacts, run focused release tests, and commit several
  slices.
- Full SOLID refactor completion: likely 4-8 focused work sessions, because
  thread-store integration, app-server protocol projection, remaining static
  boundary leaks, DAB canary verification, and commit splitting still need
  sequential root oversight.

Parallelism status: seven useful sessions are currently launched and alive.
Do not launch more until at least one of the two edit-owned workers finishes or
fails, unless the new worker is strictly read-only and does not touch session,
thread-manager, handler, manifest, lock, or schema paths.

### Resume Steps After Compaction

1. Re-run marker summary:
   `Get-ChildItem .codex\workflow\agents -Filter *.exec.marker.txt | Sort-Object Name`
   and inspect completion lines for the seven new sessions.
2. Read new non-placeholder handoffs from:
   `plugin_tool_compile_worker`, `skill_dependency_compile_worker`,
   `app_server_boundary_rescue`, `bazel_lock_rescue`,
   `thread_store_integration_rescue`, `hook_runtime_compile_scout`, and
   `dab_internal_canary_scout`.
3. If either edit worker changed files, inspect only its owned paths first; do
   not run `just fmt` or release tests until both edit workers have finished or
   failed.
4. If the plugin/skill compile slices look coherent, root should run `just fmt`
   in `codex-rs`, then the smallest focused release test/check lane recommended
   by the handoffs. Do not run debug-profile Cargo.
5. Commit only verified coherent slices with explicit pathspec staging. Do not
   use `git add .`.

## 2026-05-20 Core Test Split Priority Update

User direction: splitting `codex-core` tests so they compile and run in smaller,
fast lanes has higher priority than continuing to run or wait on broad
`codex-core` builds/tests. Treat broad release builds as validation after the
split structure exists, not as the main way to make progress.

Operating rule for parallelism:

- At any moment, spawn as many Codex sessions as there are genuinely parallel,
  non-overlapping subtasks.
- Prefer many read-only scouts while path ownership is unclear.
- Assign at most one worker to central harness files such as
  `codex-rs/core/tests/all.rs`, `codex-rs/core/tests/suite/mod.rs`,
  `codex-rs/core/Cargo.toml`, and Bazel files until a split plan gives exact
  ownership.
- Encourage workers to delegate bounded read-only subquestions, but require
  each worker to keep its own handoff authoritative.
- Root keeps Git staging/commits, manifest/lock/Bazel ownership, and final
  verification lane selection.

New priority:

1. Map the `codex-core` test harness and split blockers.
2. Create or use a small read-only prototype to rank test modules and detect
   `super::`/shared-support dependencies.
3. Implement the first small split lane only after the scouts identify a
   mechanically safe module group.
4. Run only the smallest release-profile test command that validates the new
   split lane. Do not restart broad `codex-core` release tests while the harness
   is still monolithic.

Sessions launched for this lane:

- `core_test_split_topology_scout`
  - prompt: `.codex/workflow/agents/core_test_split_topology_scout.prompt.md`
  - handoff: `.codex/workflow/agents/core_test_split_topology_scout.handoff.md`
  - marker: `.codex/workflow/agents/core_test_split_topology_scout.exec.marker.txt`
- `core_test_split_cost_map_scout`
  - prompt: `.codex/workflow/agents/core_test_split_cost_map_scout.prompt.md`
  - handoff: `.codex/workflow/agents/core_test_split_cost_map_scout.handoff.md`
  - marker: `.codex/workflow/agents/core_test_split_cost_map_scout.exec.marker.txt`
- `core_test_split_common_support_scout`
  - prompt: `.codex/workflow/agents/core_test_split_common_support_scout.prompt.md`
  - handoff: `.codex/workflow/agents/core_test_split_common_support_scout.handoff.md`
  - marker: `.codex/workflow/agents/core_test_split_common_support_scout.exec.marker.txt`
- `core_test_split_cargo_bazel_scout`
  - prompt: `.codex/workflow/agents/core_test_split_cargo_bazel_scout.prompt.md`
  - handoff: `.codex/workflow/agents/core_test_split_cargo_bazel_scout.handoff.md`
  - marker: `.codex/workflow/agents/core_test_split_cargo_bazel_scout.exec.marker.txt`
- `core_test_split_lane_plan_scout`
  - prompt: `.codex/workflow/agents/core_test_split_lane_plan_scout.prompt.md`
  - handoff: `.codex/workflow/agents/core_test_split_lane_plan_scout.handoff.md`
  - marker: `.codex/workflow/agents/core_test_split_lane_plan_scout.exec.marker.txt`
- `core_test_split_analysis_proto`
  - prompt: `.codex/workflow/agents/core_test_split_analysis_proto.prompt.md`
  - handoff: `.codex/workflow/agents/core_test_split_analysis_proto.handoff.md`
  - marker: `.codex/workflow/agents/core_test_split_analysis_proto.exec.marker.txt`
  - owned edit path: `.codex/prototypes/plan-core-test-split.ps1`.

## 2026-05-20 Continuation: Execution Rules

- Compact early. Before root or any worker crosses roughly 50% of its token
  budget, update this handoff or the worker handoff first, then compact. Do not
  wait for a near-limit automatic compaction.
- Use external Codex worker sessions for breadth. Prefer
  `.codex/workflow/scripts/Start-CodexWorker.ps1 -Mode Exec` with prompt,
  marker, and handoff files under `.codex/workflow/agents/` when there are
  independent edit lanes; do not treat in-process agent thread limits as the
  team-size limit.
- Edit workers may proceed on path-owned non-test SOLID/refactor slices in
  parallel with the core test split. They must not run broad `codex-core` builds
  or tests until the split harness is verified, and root still owns manifests,
  Bazel wiring, lockfiles, aggregate verification, staging, and commits.
- The current test split is already partially implemented in the working tree:
  `codex-rs/core/tests/all.rs` and `codex-rs/core/tests/suite/mod.rs` are
  deleted, shared dispatch bootstrap lives in `codex-rs/core/tests/support/`,
  and the first split binaries are `agents`, `client`, `compact`, `config`,
  `exec`, `permissions`, `state`, and `tools`.

## 2026-05-20 23:52 +03: External Edit Worker Wave

Root correction after user reminder:

- No root build/test work should run while the split/refactor is still in
  progress. A prior root attempt to compile `--test compact --no-run` was
  stopped; do not repeat that lane until the worker refactors finish.
- Worker prompts now say: do not run `cargo`, `just`, `bazel`, build scripts, or
  test scripts while refactor edits are in progress; finish owned refactor
  edits first, then use only a narrow non-broad owned-slice check if appropriate.
- Worker prompts now also ask each worker to stage only owned changed paths and
  commit with a concise message when their edits are complete. If Git refuses
  because of merge/unmerged state, workers must record the exact blocker in
  their handoff.
- Compact early: before root or any worker crosses roughly 50% of token budget,
  update this handoff or the worker handoff first, then compact.

All launched via `.codex/workflow/scripts/Start-CodexWorker.ps1 -Mode Exec`.
Markers existed with `starting mode=Exec` immediately after launch.

- `core_tests_harness_manifest_worker`
  - prompt: `.codex/workflow/agents/core_tests_harness_manifest_worker.prompt.md`
  - handoff: `.codex/workflow/agents/core_tests_harness_manifest_worker.handoff.md`
  - marker: `.codex/workflow/agents/core_tests_harness_manifest_worker.exec.marker.txt`
  - owns core test manifest/Bazel/support wiring.
- `core_tests_compact_lane_worker`
  - prompt: `.codex/workflow/agents/core_tests_compact_lane_worker.prompt.md`
  - handoff: `.codex/workflow/agents/core_tests_compact_lane_worker.handoff.md`
  - marker: `.codex/workflow/agents/core_tests_compact_lane_worker.exec.marker.txt`
  - owns compact/context/resume suite lane.
- `core_tests_exec_permissions_lane_worker`
  - prompt: `.codex/workflow/agents/core_tests_exec_permissions_lane_worker.prompt.md`
  - handoff: `.codex/workflow/agents/core_tests_exec_permissions_lane_worker.handoff.md`
  - marker: `.codex/workflow/agents/core_tests_exec_permissions_lane_worker.exec.marker.txt`
  - owns exec/sandbox/permissions suite lanes.
- `core_tests_config_state_lane_worker`
  - prompt: `.codex/workflow/agents/core_tests_config_state_lane_worker.prompt.md`
  - handoff: `.codex/workflow/agents/core_tests_config_state_lane_worker.handoff.md`
  - marker: `.codex/workflow/agents/core_tests_config_state_lane_worker.exec.marker.txt`
  - owns config/state suite lanes.
- `core_tests_agents_lane_worker`
  - prompt: `.codex/workflow/agents/core_tests_agents_lane_worker.prompt.md`
  - handoff: `.codex/workflow/agents/core_tests_agents_lane_worker.handoff.md`
  - marker: `.codex/workflow/agents/core_tests_agents_lane_worker.exec.marker.txt`
  - owns agents/delegation suite lane.
- `core_tests_client_lane_worker`
  - prompt: `.codex/workflow/agents/core_tests_client_lane_worker.prompt.md`
  - handoff: `.codex/workflow/agents/core_tests_client_lane_worker.handoff.md`
  - marker: `.codex/workflow/agents/core_tests_client_lane_worker.exec.marker.txt`
  - owns client/realtime/websocket suite lane.
- `core_tests_tools_lane_worker`
  - prompt: `.codex/workflow/agents/core_tests_tools_lane_worker.prompt.md`
  - handoff: `.codex/workflow/agents/core_tests_tools_lane_worker.handoff.md`
  - marker: `.codex/workflow/agents/core_tests_tools_lane_worker.exec.marker.txt`
  - owns tools/MCP/plugins suite lane.
- `thread_store_api_recording_repair_worker`
  - prompt: `.codex/workflow/agents/thread_store_api_recording_repair_worker.prompt.md`
  - handoff: `.codex/workflow/agents/thread_store_api_recording_repair_worker.handoff.md`
  - marker: `.codex/workflow/agents/thread_store_api_recording_repair_worker.exec.marker.txt`
  - owns thread-store API recording utility repair.
- `config_provenance_boundary_worker`
  - prompt: `.codex/workflow/agents/config_provenance_boundary_worker.prompt.md`
  - handoff: `.codex/workflow/agents/config_provenance_boundary_worker.handoff.md`
  - marker: `.codex/workflow/agents/config_provenance_boundary_worker.exec.marker.txt`
  - owns config provenance boundary cleanup.
- `boundary_delta_edit_worker`
  - prompt: `.codex/workflow/agents/boundary_delta_edit_worker.prompt.md`
  - handoff: `.codex/workflow/agents/boundary_delta_edit_worker.handoff.md`
  - marker: `.codex/workflow/agents/boundary_delta_edit_worker.exec.marker.txt`
  - owns one non-overlapping boundary-delta edit slice.

## 2026-05-21 Root Resume Rollup

User directive for this resume:

- Root is an orchestrator, not the primary implementer.
- Use external Codex terminal sessions via
  `.codex/workflow/scripts/Start-CodexWorker.ps1`; do not use in-process agents
  for this lane.
- Workers may edit only explicitly owned paths and should commit coherent scoped
  changes when safe.
- Workers must not run builds/tests while their owned refactor is in progress.
  After the refactor is complete, only focused release verification is allowed;
  no broad `codex-core` or workspace verification until the split-test and
  compile-blocker lanes settle.
- Root should delegate review work to a spawned session rather than self-review.
- Compact early: update this handoff around 40-45% context and compact before
  50%; prompts and handoffs must ask for concise summaries, not raw transcripts.

Finished worker results folded in:

- `core_tests_tools_lane_worker`: tools/MCP/plugins test lane complete and
  committed as `2cfee083d0`.
- `core_tests_client_lane_worker`: client/realtime/websocket test lane complete
  and committed as `3db90f6110`.
- `core_tests_agents_lane_worker`: agents/delegation test lane complete and
  committed as `444b583f15`.
- `core_tests_harness_manifest_worker`: split test harness/manifest wiring
  complete and committed as `92039a4e32`.
- `core_tests_exec_permissions_lane_worker`: exec/permissions wrappers complete
  and committed as `5b68c4973b`; remaining removed permissions-related modules
  need residual routing.
- `core_tests_config_state_lane_worker`: config/state wrappers complete and
  committed as `a86d553882`.
- `core_tests_compact_lane_worker`: compact/context/resume lane complete and
  committed as `14727b4a61`.
- `thread_store_api_recording_repair_worker`: recording API utility repair
  complete, focused release check passed, committed as `3a00d81024`.
- `config_provenance_boundary_worker`: no config-provenance source edits were
  needed; handoff committed as `f3ea64c83e`.
- `boundary_delta_edit_worker`: core `AuthMode` protocol import cleanup
  complete and committed as `8939c9d0a7`.

Current test split shape:

- `codex-rs/core/tests/all.rs` and `codex-rs/core/tests/suite/mod.rs` are gone.
- Split integration binaries now include `agents.rs`, `client.rs`,
  `compact.rs`, `config.rs`, `exec.rs`, `permissions.rs`, `state.rs`, and
  `tools.rs`; `responses_headers.rs` remains standalone.
- Shared support moved to `codex-rs/core/tests/support/mod.rs`.
- Compact fixtures live in `codex-rs/core/tests/common/compact_fixtures.rs`
  and are exported from `common/lib.rs`.

Known blockers before broad verification:

- Shared `codex-core` library compile errors still block split-test release
  checks.
- Reported hotspots include `core/src/session/turn.rs`,
  `core/src/config/permissions.rs`, `core/src/tools/router.rs`,
  `core/src/tools/spec_plan.rs`, and thread-store callsites with changed
  `LocalThreadStore` / `LocalThreadStoreConfig` APIs.
- Residual suite routing is complete as of `d0a3390511`. Review follow-up:
  confirm/fix the `request_permissions.rs` platform gate in
  `codex-rs/core/tests/permissions.rs` before releasing that split lane.

Workflow cleanup performed by root:

- Deleted completed one-shot worker prompt files, exec/marker files,
  read-report JSON, help text, `.codex/workflow-batch/`, and
  `.codex/workflow/.tmp/`.
- Kept `*.handoff.md` files as orchestration history.
- Added scoped `.gitignore` rules for future workflow scratch, prompt, marker,
  and read-report files.

Next external worker queue status:

1. `core_compile_session_thread_worker`: handoff received; uncommitted changes
   await `codex-otel` blocker resolution and root integration.
2. `core_compile_tools_worker`: handoff received; uncommitted changes await
   `codex-otel` blocker resolution and root integration.
3. `app_server_boundary_finish_worker`: launched at 2026-05-21T01:57+03:00.
4. `config_connectors_boundary_worker`: handoff received; owned source slice is
   uncommitted pending focused release verification and manifest/root ownership
   decision.
5. `core_protocol_dependency_followup_worker`: launched at
   2026-05-21T02:18+03:00.
6. `request_permissions_gate_worker`: handoff received and code commit
   integrated; focused release verification deferred by live build contention
   and low free disk.
7. `codex_otel_compile_followup_worker`: launched at
   2026-05-21T02:18+03:00.

Launched at 2026-05-21 01:12 Europe/Kiev via
`.codex/workflow/scripts/Start-CodexWorker.ps1 -Mode Exec`:

- `core_tests_residual_router_worker`
- `core_compile_session_thread_worker`
- `core_compile_tools_worker`
- `core_compile_config_permissions_worker`
- `boundary_dependency_manifest_worker`
- `recent_worker_review_worker`

Additional launched at 2026-05-21 01:57 Europe/Kiev via the same script:

- `app_server_boundary_finish_worker`
- `compaction_output_plan_worker`

Completed/uncommitted handoff received from that launch group:

- `config_connectors_boundary_worker`

Additional review follow-ups launched after `recent_worker_review_worker`:

- `core_protocol_dependency_followup_worker`
- `codex_otel_compile_followup_worker`

Completed review follow-up integrated:

- `request_permissions_gate_worker` (`62663cc4ed`)

Ignored runtime prompt/marker paths for these sessions:

- `.codex/workflow/agents/<worker>.prompt.md`
- `.codex/workflow/agents/<worker>.exec.marker.txt`

Root should next read their `*.handoff.md` files when the tabs finish, fold the
results into this document, and compact early before 50% context.
