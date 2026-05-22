# SOLID Refactor Handoff

Date: 2026-05-22
Status: active refactor-first orchestration; compact-safe current state; wave19 preparing/launching.

This handoff is the current source of truth. It intentionally omits old launch
history except where it affects active process ownership.

## Director Checkpoint - 2026-05-22 02:23 Europe/Kyiv

- Director reread the overseer memo, this handoff, the SOLID plan/review docs,
  and fresh handoffs. Director did not edit product/refactor source, manifests,
  lockfiles, Bazel files, generated schemas, commits, deploy, or activation
  files.
- Source-only checks run from director:
  - `git diff --check` exited 0 with line-ending warnings only.
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-cargo-dependency-boundaries.ps1 -SolidRefactor -Json`
    exited 0 with `violation_count = 0`.
  - Targeted `rg` over the core/tools boundary still shows:
    - `codex-rs/core/src/client.rs` imports
      `codex_tools::create_tools_json_for_responses_api`.
    - `codex-rs/core/src/goals.rs` still references
      `codex_tools::UPDATE_GOAL_TOOL_NAME`.
    - `codex-rs/core/Cargo.toml` still has `codex-tools = { workspace = true }`
      and lacks the direct `codex-tool-registry-api` dependency needed by the
      wave 14 `client_common.rs` import.
- Fresh worker handoff classifications:
  - `solid_refactor_wave14_core_tools_source_boundary_worker.handoff.md`:
    `root-wiring-needed`. Source slice accepted: `ToolSpec` and `ToolName`
    API-type imports moved to the narrow API crates. Remaining work is manifest
    wiring plus separate helper/constant boundary slices.
  - `solid_refactor_wave13_tools_domain_manifest_wiring_worker.handoff.md`:
    `accepted`. `codex-tools` now depends on the extracted tool API crates;
    lock/Bazel refresh remains intentionally deferred.
  - `solid_refactor_wave12_core_tools_dependency_recovery_worker.handoff.md`:
    `root-wiring-needed`, partly superseded by waves 13/14. The current
    unresolved core concrete dependency is now narrowed to the serializer helper,
    the goal-tool constant, and manifest cleanup.
  - `solid_refactor_wave12_manifest_wiring_worker.handoff.md`: `accepted`.
    It added explicit core test targets for wave 11 split suites; release
    verification remains deferred until architecture boundaries are stable.
  - `solid_refactor_fix_session_workspace_roots_worker.handoff.md`: source slice
    accepted, verification deferred. It supersedes the earlier
    `blocked-moving-tree` read-only workspace-roots review; do not run release
    tests from director while architecture is still moving.
  - Wave 11 split handoffs for client websocket, hooks, client residual,
    search/OTEL/model, permissions, and unified exec are accepted source/test
    split slices; wave 12 manifest wiring picked up their explicit targets.
- Current high-value remaining gaps:
  - Close `codex-core -> codex-tools` by moving/removing the remaining
    `codex_tools` helper and constant references, then remove the direct
    `codex-tools` manifest dependency when no core source references remain.
  - Repair or isolate the core test-support dependency blocker around
    `codex-rs/core/tests/common/test_codex.rs` thread-store imports without
    reintroducing broad concrete dependencies.
  - Continue splitting large `codex-rs/core/tests/suite/*.rs` families by topic
    into independently wired Cargo test binaries with narrow dependencies.
  - Keep mixed lock/Bazel/generated-schema fallout uncommitted and do not refresh
    generated metadata until the architecture source boundary is closed.
- Next wave prompt files prepared for visible workers:
  - `solid_refactor_wave15_core_tools_manifest_worker.prompt.md`
  - `solid_refactor_wave15_tool_serializer_boundary_worker.prompt.md`
  - `solid_refactor_wave15_goal_tool_constant_boundary_worker.prompt.md`
  - `solid_refactor_wave15_core_test_support_boundary_worker.prompt.md`
  - `solid_refactor_wave15_session_workspace_roots_static_review_worker.prompt.md`
  - `solid_refactor_wave15_remaining_core_suite_split_scout_worker.prompt.md`
- Wave 15 launched with `codex-workers -Pattern "solid_refactor_wave15*.prompt.md"`
  at 2026-05-22 02:29 Europe/Kyiv. Visible worker processes:
  - `solid_refactor_wave15_core_test_support_boundary_worker` PID 19244.
  - `solid_refactor_wave15_core_tools_manifest_worker` PID 12916.
  - `solid_refactor_wave15_goal_tool_constant_boundary_worker` PID 7160.
  - `solid_refactor_wave15_remaining_core_suite_split_scout_worker` PID 4544.
  - `solid_refactor_wave15_session_workspace_roots_static_review_worker` PID 19764.
  - `solid_refactor_wave15_tool_serializer_boundary_worker` PID 14892.
  Monitor matching `.exec.marker.txt`, `.exec.visible.log`, and `.handoff.md`
  files under `.codex/workflow/agents/`.
- Wave 15 manifest result:
  - `solid_refactor_wave15_core_tools_manifest_worker.handoff.md` is
    `root-wiring-needed`. It added `codex-tool-registry-api` to
    `codex-rs/core/Cargo.toml` and correctly left `codex-tools` because
    `client.rs` and `goals.rs` still reference `codex_tools`.
  - Director reran the allowed boundary checker and it now fails with one
    expected policy violation:
    `unclassified_core_dependency` for
    `codex-core -> codex-tool-registry-api` at `codex-rs\core\Cargo.toml:506`.
  - Repair prompt prepared:
    `solid_refactor_wave15_boundary_policy_repair_worker.prompt.md`.
  - Repair worker launched visible with PID 15224 at 2026-05-22 02:33
    Europe/Kyiv.
- Wave 15 source/policy results as of 2026-05-22 02:42 Europe/Kyiv:
  - `solid_refactor_wave15_tool_serializer_boundary_worker.handoff.md`:
    `root-wiring-needed`, source slice accepted. The Responses API serializer
    now lives behind `codex-tool-registry-api`, `codex-tools` keeps a
    compatibility wrapper, and `codex-rs/core/src/client.rs` imports the narrow
    API. Remaining root wiring was the boundary-policy classification below.
  - `solid_refactor_wave15_goal_tool_constant_boundary_worker.handoff.md`:
    `root-wiring-needed`, source slice accepted. `codex-rs/core/src/goals.rs`
    now uses the core-local goal spec constant instead of `codex_tools`.
  - `solid_refactor_wave15_core_test_support_boundary_worker.handoff.md`:
    `accepted`. `codex-rs/core/tests/common/test_codex.rs` now uses
    `codex_thread_store_api` abstractions for the test harness path; broader
    core application-wiring imports remain a later design decision.
  - `solid_refactor_wave15_boundary_policy_repair_worker.handoff.md`:
    `accepted`. `scripts/check-cargo-dependency-boundaries.ps1` now classifies
    `codex-core -> codex-tool-registry-api` as an allowed narrow API edge.
  - Director reran
    `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-cargo-dependency-boundaries.ps1 -SolidRefactor -Json`;
    it exited 0 with `violation_count = 0`.
  - Pending handoffs: `solid_refactor_wave15_session_workspace_roots_static_review_worker`
    and `solid_refactor_wave15_remaining_core_suite_split_scout_worker`.
- `solid_refactor_wave15_remaining_core_suite_split_scout_worker.handoff.md`
  later landed as `accepted` and is the source for the next test-split worker
  wave. The original `solid_refactor_wave15_session_workspace_roots_static_review_worker`
  process is still alive but had not produced a handoff by 2026-05-22 02:47
  Europe/Kyiv, so a tighter read-only retry prompt was prepared:
  `solid_refactor_wave15b_session_workspace_roots_static_review_retry_worker.prompt.md`.
  The retry worker launched visible with PID 14900.
- `solid_refactor_wave15b_session_workspace_roots_static_review_retry_worker.handoff.md`
  landed as `conflict/blocked`: the prompt named
  `codex-rs/app-server/src/codex_manager.rs` and
  `codex-rs/app-server/src/app_event_processor.rs`, but those files are not
  present at current `HEAD`. A wave 16 path scout should locate the current
  owning app-server modules before requesting an accepted static review.
- Director source-only boundary scan after wave 15 found many remaining
  `codex_tools` references under `codex-rs/core/src` and tests. The direct
  `client.rs` and `goals.rs` references are closed, but broader core surfaces
  still depend on concrete `codex-tools` types/helpers. Wave 16 should split
  this by ownership instead of trying to remove `codex-tools` from
  `codex-rs/core/Cargo.toml` immediately.
- Wave 16 prompt files prepared:
  - `solid_refactor_wave16_core_tools_surface_scout_worker.prompt.md`
  - `solid_refactor_wave16_core_tool_handlers_api_worker.prompt.md`
  - `solid_refactor_wave16_core_tools_config_boundary_worker.prompt.md`
  - `solid_refactor_wave16_connectors_discoverable_boundary_worker.prompt.md`
  - `solid_refactor_wave16_function_image_boundary_worker.prompt.md`
  - `solid_refactor_wave16_workspace_roots_path_scout_worker.prompt.md`
- Wave 16 launched with `codex-workers -Pattern "solid_refactor_wave16*.prompt.md"`
  at 2026-05-22 03:02 Europe/Kyiv. Visible worker processes:
  - `solid_refactor_wave16_connectors_discoverable_boundary_worker` PID 21068.
  - `solid_refactor_wave16_core_tool_handlers_api_worker` PID 1168.
  - `solid_refactor_wave16_core_tools_config_boundary_worker` PID 19732.
  - `solid_refactor_wave16_core_tools_surface_scout_worker` PID 19332.
  - `solid_refactor_wave16_function_image_boundary_worker` PID 16136.
  - `solid_refactor_wave16_workspace_roots_path_scout_worker` PID 19084.
- Compact checkpoint - 2026-05-22 03:08 Europe/Kyiv:
  - User requested handoff update and compact. Stop at this safe boundary after
    writing this checkpoint.
  - Wave 16 handoffs present: only
    `solid_refactor_wave16_core_tools_surface_scout_worker.handoff.md`.
  - Wave 16 surface scout classification: `accepted`. It was read-only and
    found 63 current `codex_tools` references across 27 files. It grouped the
    remaining concrete `codex-core -> codex-tools` surface into:
    tool spec/name/registry references, tool config/environment references,
    connector/plugin discoverable metadata, function-call/original-image helper
    facades, session/runtime wiring, and tests.
  - Wave 16 worker processes still alive at checkpoint:
    - PID 21068 `solid_refactor_wave16_connectors_discoverable_boundary_worker`
    - PID 1168 `solid_refactor_wave16_core_tool_handlers_api_worker`
    - PID 19732 `solid_refactor_wave16_core_tools_config_boundary_worker`
    - PID 19332 `solid_refactor_wave16_core_tools_surface_scout_worker`
    - PID 16136 `solid_refactor_wave16_function_image_boundary_worker`
    - PID 19084 `solid_refactor_wave16_workspace_roots_path_scout_worker`
  - On resume, reread this file, the overseer memo, the SOLID docs, and fresh
    `.codex/workflow/agents/solid_refactor_wave16*.handoff.md` files. Then
    classify wave 16 handoffs before launching any new workers.
  - Do not run broad builds/tests/schema/Bazel/lock/release. Allowed director
    checks remain `git diff --check`, targeted `rg`, PowerShell parser checks
    for changed `.ps1` files, and
    `scripts/check-cargo-dependency-boundaries.ps1 -SolidRefactor -Json`.

## Compact Checkpoint - 2026-05-21 13:52 Europe/Kyiv

- Branch is synced with origin: `git rev-list --left-right --count
  HEAD...origin/slow-context-budget-mode` returned `0 0`.
- Latest pushed checkpoint remains `e0654303c9 Document SOLID fix verification
  state`; this section is the next docs/report update after that commit.
- No Cargo/rustc/link/cl process is active.
- New read-only reports now landed:
  - `solid_refactor_readonly_agent_depth_review_worker.handoff.md`: depth
    policy leak appears fixed. The reviewer marked `status: finding` because
    this is a broader agent graph-store/policy adapter slice, not a depth-only
    patch; commit only with required untracked graph-store module files,
    manifest/lock updates, and focused core verification after the
    test-support dependency blocker is repaired.
  - `solid_refactor_commit_ready_app_server_bazel_audit_worker.handoff.md`:
    app-server permission/profile/schema is `blocked-by-mixed-diff`; no safe
    path-level add list. Bazel BUILD scaffolds are `commit-ready` as a separate
    path-only slice independent of dirty `Cargo.lock`.
  - `solid_refactor_readonly_workspace_roots_review_worker.handoff.md`: current
    source snapshot appears to fix the P1 workspace-root drop, but status is
    `blocked-moving-tree` until the owner
    `solid_refactor_fix_session_workspace_roots_worker` handoff lands.
  - `solid_refactor_readonly_dependency_lock_review_worker.handoff.md`: current
    dependency/lock state is mixed. Do not file-level commit `codex-rs/Cargo.lock`,
    `codex-rs/core/Cargo.toml`, `codex-rs/core-api/Cargo.toml`, or
    `codex-rs/core-domain/types/Cargo.toml` until source owners settle and
    staging is split by owner.
- Still missing handoff:
  - `.codex/workflow/agents/solid_refactor_fix_session_workspace_roots_worker.handoff.md`.
- Active implementation session:
  - `solid_refactor_fix_session_workspace_roots_worker`: PowerShell `26904`,
    `pwsh` `18832`, Python `5604`/`6768`, Codex `3664`.
  - Other read-only review/audit wrappers may remain open after writing
    reports; treat them as complete unless their handoff timestamp changes.
- Current completion estimate: about 78% complete. Remaining repo-controlled
  work is roughly 2-3 hours if verification remains incremental:
  1. Wait for `solid_refactor_fix_session_workspace_roots_worker.handoff.md`.
  2. Update findings based on that owner handoff.
  3. Repair `codex-rs/core/tests/common/Cargo.toml` thread-store deps so
     focused core `multi_agent_v2` release verification can run.
  4. Commit/push docs/report checkpoint, then integrate clean product slices:
     Bazel BUILD scaffolds first, agent graph-store/policy adapter after core
     verification, workspace-root settings after owner handoff/tests, then
     core-api/domain and app-server schema/source only after mixed diffs split.

## Compact Checkpoint - 2026-05-21 13:46 Europe/Kyiv

- Branch is synced with origin: `git rev-list --left-right --count
  HEAD...origin/slow-context-budget-mode` returned `0 0`.
- Latest pushed checkpoint: `e0654303c9 Document SOLID fix verification state`.
- No Cargo/rustc/link/cl process is active. It is safe to run the next
  targeted release verification after worker handoffs are integrated, but do
  not start broad build lanes by default.
- New read-only reports landed after `e0654303c9`:
  - `.codex/workflow/agents/solid_refactor_readonly_agent_depth_review_worker.handoff.md`
    reports `status: finding`: the depth policy leak appears fixed, but the
    diff is a broader agent graph-store/policy adapter slice, not a depth-only
    patch. Commit it only with the required untracked graph-store module files,
    manifest/lock updates, and focused core verification after the separate
    test-support dependency blocker is repaired.
  - `.codex/workflow/agents/solid_refactor_commit_ready_app_server_bazel_audit_worker.handoff.md`
    reports app-server permission/profile schema is `blocked-by-mixed-diff`
    and cannot be path-added safely. It says the Bazel BUILD scaffold slice is
    `commit-ready` with a path-only add list and should stay separate from
    dirty `Cargo.lock` and source changes.
- Still missing handoffs:
  - `.codex/workflow/agents/solid_refactor_fix_session_workspace_roots_worker.handoff.md`.
  - `.codex/workflow/agents/solid_refactor_readonly_workspace_roots_review_worker.handoff.md`.
  - `.codex/workflow/agents/solid_refactor_readonly_dependency_lock_review_worker.handoff.md`.
- Live external worker/session snapshot:
  - `solid_refactor_fix_session_workspace_roots_worker`: PowerShell `26904`,
    Codex `3664`.
  - `solid_refactor_readonly_workspace_roots_review_worker`: PowerShell `2524`,
    Codex `17628`.
  - `solid_refactor_readonly_dependency_lock_review_worker`: PowerShell
    `30696`, Codex `11308`.
  - Completed-report wrapper windows may remain open for
    `solid_refactor_commit_ready_app_server_bazel_audit_worker` and
    `solid_refactor_readonly_agent_depth_review_worker`; treat them as
    complete unless a new handoff timestamp appears.
- Current completion estimate: about 75% complete. Remaining repo-controlled
  work is roughly 2-4 hours if verification remains incremental: integrate the
  workspace-roots handoff, repair core test-support deps, wait for dependency
  lock review, then commit/push clean slices in this order where possible:
  Bazel BUILD scaffolds, agent graph-store/policy adapter, workspace-root
  settings fix, core-api/domain lock/Bazel follow-up, and finally app-server
  schema/source once mixed diffs are separated.

## Continuation Checkpoint - 2026-05-21 13:18 Europe/Kyiv

- Branch was still synced before this handoff update:
  `git rev-list --left-right --count HEAD...origin/slow-context-budget-mode`
  returned `0 0`.
- `solid_refactor_fix_replacement_shadow_dep_worker.handoff.md` landed and
  verified the replacement-shadow dependency cleanup:
  - `just bazel-lock-update`: passed.
  - `just bazel-lock-check`: passed.
  - `cargo check -p codex-core --release --locked`: passed; log
    `logs/solid-refactor-codex-core-release-check-20260521-125117.log`.
  - Treat the stale `codex-replacement-shadow` core dependency finding as fixed
    at current branch `HEAD`.
- Follow-up at 2026-05-21 13:28 Europe/Kyiv: the release lane exited; no
  `cargo.exe`, `rustc.exe`, `link.exe`, or `cl.exe` rows were active.
- `solid_refactor_fix_agent_depth_policy_worker.handoff.md` landed:
  - Source fix: recursive persisted-descendant resume now calls the
    `codex-agent-policy` depth helper instead of doing local `parent_depth + 1`
    arithmetic in core.
  - Verification passed: `just fmt` and
    `scripts\test-local-codex-release.ps1 -Package codex-agent-policy`.
  - Core verification is still blocked outside that worker's ownership:
    `codex-rs/core/tests/common/test_codex.rs` imports
    `codex_thread_store` / `codex_thread_store_api`, but
    `codex-rs/core/tests/common/Cargo.toml` does not declare those
    dependencies.
- Active visible implementation workers still missing handoffs:
  - `solid_refactor_fix_session_workspace_roots_worker`: PowerShell `26904`,
    Codex `3664`.
  - `solid_refactor_fix_agent_depth_policy_worker`: handoff landed; wrapper
    windows may remain open, but root should treat this worker as complete
    unless its read-only reviewer finds a concrete regression.
- Active read-only review/audit workers launched or still running; all are
  command-banned from Cargo/Bazel/tests/schema generation and may write only
  their assigned handoff:
  - `solid_refactor_commit_ready_app_server_bazel_audit_worker`: app-server
    permission/schema and Bazel scaffold commit readiness.
  - `solid_refactor_readonly_agent_depth_review_worker`: source sanity review
    for the agent-depth policy fix.
  - `solid_refactor_readonly_workspace_roots_review_worker`: current
    workspace-root settings data-flow review; expected to report
    `blocked-moving-tree` if the owner worker is still active.
  - `solid_refactor_readonly_dependency_lock_review_worker`: dependency,
    lock, and core test-support boundary audit.
- Current completion estimate: about 70-75% of the refactor orchestration is
  complete. The remaining critical path is the workspace-roots worker handoff,
  core test-support dependency repair, focused release verification, and scoped
  commit/push of ready source groups. Expected repo-controlled time is roughly
  2-4 hours if verification stays incremental; longer if core/app-server checks
  force rebuilds or uncover source regressions.

## Compaction Checkpoint - 2026-05-21 12:45 Europe/Kyiv

- Branch is synced with origin: `git rev-list --left-right --count HEAD...origin/slow-context-budget-mode` returned `0 0`.
- Latest pushed orchestration commits:
  - `7917c50e52 Add core API retry review handoff`.
  - `88c98ca0b6 Document SOLID core API review fallback`.
  - `21fccba985 Fix SOLID review handoff prompts`.
  - Earlier docs/review setup commits: `39a414106b`, `55cbc90c48`.
- Self-review correction: `7917c50e52` was not docs-only. It also included
  the replacement-shadow dependency cleanup from already-staged worker files:
  `codex-rs/core/Cargo.toml` and `codex-rs/Cargo.lock` each removed the dead
  `codex-replacement-shadow` entry. Do not assume that source slice is verified
  until the active worker/verification lane reports success.
- Active build/verification lane exists now. Do not start competing Cargo/Bazel/build work until it finishes:
  - Process IDs are volatile; recheck before acting.
  - Latest refresh during self-review saw `cargo.exe:27964` and `rustc.exe:18424`.
- Active visible orchestration/fix sessions:
  - `solid_refactor_area_review_retry_core_api_worker`: PowerShell `34688`, Codex `25128`; no handoff seen yet.
  - `solid_refactor_commit_grouping_worker`: PowerShell `33944`, Codex `12432`; no handoff seen yet.
  - `solid_refactor_fix_session_workspace_roots_worker`: PowerShell `26904`, Codex `3664`; likely owns the active verification lane.
  - `solid_refactor_fix_agent_depth_policy_worker`: PowerShell `25236`, Codex `15848`.
  - `solid_refactor_fix_replacement_shadow_dep_worker`: PowerShell `23324`, Codex `5144`.
- New useful handoffs landed after the last commit and should be committed with this handoff update:
  - `.codex/workflow/agents/solid_refactor_area_review_core_api_quick_worker.handoff.md`
  - `.codex/workflow/agents/solid_refactor_area_review_retry_core_api_worker.handoff.md`
  - `.codex/workflow/agents/solid_refactor_area_review_retry_session_settings_worker.handoff.md`
  - `.codex/workflow/agents/solid_refactor_area_review_retry_tests_schema_worker.handoff.md`
- Review state:
  - Core-api quick review found no concrete import/API regression from the identifier move; keep core-api/domain source plus `Cargo.lock` separate from app-server schema JSON and run the named release/Bazel lock checks before committing that source slice.
  - Core-api retry review adds a commit blocker: `codex-rs/Cargo.lock` is stale/mixed for the `codex-core-api -> codex-core-domain-types` dependency move and must be refreshed with the required Bazel lock flow before the core-api slice is committed.
  - Replacement-shadow dependency cleanup is now committed in `7917c50e52`, but
    the active verification lane is still running; wait for
    `solid_refactor_fix_replacement_shadow_dep_worker.handoff.md` or process
    completion before treating it as green.
  - Retry session-settings review reconfirmed the P1 workspace-root data-loss blocker.
  - Retry tests/schema review reconfirmed two blockers before schema/test commits: workspace-root data loss and stale permission-shape schema/test drift; schema JSON must stay with its owning DTO/source change.
- Main next action after compaction:
  1. Monitor active fix workers and read their `.handoff.md` files as they land.
  2. Do not launch more verification while `cargo.exe`/`cl.exe` are active.
  3. Commit/push this compact checkpoint plus the three new review handoffs as a docs-only orchestration slice.
  4. Once implementation workers finish, integrate only verified source slices by ownership.

## Objective

Refactor `codex-core` and its tests so core code does not depend directly or
transitively on concrete implementation crates where a small boundary
abstraction, domain type, policy object, or split support crate is the right
owner.

Work order:

1. Finish source/test boundary refactors.
2. Reconcile manifests/Bazel/schema/lock updates after source ownership is
   clear.
3. Repair compile fallout caused by the refactor and recent `main` merge.
4. Run formatting, scoped release checks/tests, then broader verification only
   after boundaries are stable.

## Operating Rules

- Root is director/integrator: spawn visible workers, read handoffs, assign
  follow-ups, integrate by ownership, and run final verification.
- Prefer external visible Codex worker windows for parallel subtasks. Do not
  use in-thread `spawn_agent` workers for this refactor.
- Do not start broad builds/tests, `just fix`, Bazel, schema generation, or
  deploy scripts while source boundaries are still moving.
- Worker commit cadence:
  - Editable workers own their coherent slice through commit and push after
    their allowed verification is green and the remote is not ahead.
  - Read-only, review-only, or command-banned workers may write only their
    assigned `.handoff.md`; they must not commit, and the handoff must mark
    commit-ready files, missing verification, and the exact root commit
    boundary.
  - Root should not let useful verified work sit in the dirty tree while
    starting unrelated slices. Group dirty work by ownership, verify the
    narrowest safe lane, commit, and push before widening the next wave.
- Unless explicitly assigned verification/commit ownership, workers must not
  run `cargo`, `rustc`, `just`, Bazel, build/test scripts, schema generation,
  deploy scripts, staging, or commits.
- Do not repair fallout by adding broad compatibility imports, catch-all
  re-exports, or new broad dependencies back into `codex-core`.
- Preserve real data flow through proper models/APIs; do not replace available
  data with `None` or placeholders just to compile.

## Worker Launcher

- Preferred PATH command: `codex-workers`.
- Repo script: `.codex/workflow/agents/start-codex-workers.ps1`.
- Prompt runner: `.codex/workflow/agents/launch-solid-refactor-worker.ps1`.
- `codex-workers -List` lists matching prompts and handoffs.
- `codex-workers -DryRun` shows the windows/logs it would create.
- `codex-workers -Pattern "solid_refactor_wave4_*.prompt.md"` launches one
  visible Codex window per matching prompt file.
- `codex-workers -WorkerNames name1,name2` launches an explicit subset.

Default in this repo script is still `solid_refactor_wave3_*.prompt.md`;
always pass `-Pattern` when launching a new wave.

## Completed Results

Wave 3 handoffs ready:

- `.codex/workflow/agents/solid_refactor_wave3_session_thread_boundary_worker.handoff.md`
  - No new changes. Prior session/thread boundary pass appears complete.
- `.codex/workflow/agents/solid_refactor_wave3_protocol_domain_tests_worker.handoff.md`
  - Split a protocol/domain test family into a smaller `rollout_list_find`
    target.
- `.codex/workflow/agents/solid_refactor_wave3_tools_boundary_worker.handoff.md`
  - Added a tools-owned telemetry preview policy boundary in `codex-tools` and
    adjusted core tool call sites.
- `.codex/workflow/agents/solid_refactor_wave3_agent_boundary_worker.handoff.md`
  - Moved agent policy/summary behavior toward `codex-agent-policy` ownership.
- `.codex/workflow/agents/solid_refactor_wave3_test_support_worker.handoff.md`
  - Split core test support so protocol/domain fixtures are separated from
    runtime-instantiating helpers.
- `.codex/workflow/agents/solid_refactor_wave3_core_api_boundary_worker.handoff.md`
  - Moved core API identifier exports toward `codex-core-domain-types` /
    `codex-core-api` ownership.
- `.codex/workflow/agents/solid_refactor_wave3_compact_tests_worker.handoff.md`
  - Split `compact_remote_parity` into a focused compact test target.
- `.codex/workflow/agents/solid_refactor_wave3_dependency_scout_worker.handoff.md`
  - Read-only dependency scout completed. It found remaining source-boundary
    candidates; follow-up workers should use this handoff as their map.

Still pending from wave 3:

- `.codex/workflow/agents/solid_refactor_wave3_manifest_planner_worker.handoff.md`
  - Not present yet at the last root check.
  - Legacy hidden process was still running; do not duplicate manifest planning
    unless that process exits without a handoff or is intentionally stopped and
    relaunched visibly.

## Current Worker State

- No `cargo`, `rustc`, `link`, or `cl` processes are currently running.
- `solid_refactor_review_handoffs_worker` was stopped by root before handoff
  because it over-expanded a read-only review into broad source scanning. Treat
  its visible log as partial evidence only, not as a completed review result.
- Prompt contract correction on 2026-05-21:
  - The five `solid_refactor_area_review_*_worker.prompt.md` files and commit
    grouping prompt now state that review workers may write only their assigned
    `.handoff.md`.
  - The original scoped review prompts were contradictory: they asked for a
    handoff but also said no file edits. Treat missing original handoffs as
    prompt fallout, not reviewer failure.
- Completed scoped review handoffs now present:
  - `.codex/workflow/agents/solid_refactor_area_review_agent_tools_worker.handoff.md`.
  - `.codex/workflow/agents/solid_refactor_area_review_context_ops_worker.handoff.md`.
- Retry scoped review workers were launched visibly for missing areas:
  - `solid_refactor_area_review_retry_core_api_worker.prompt.md` ->
    `.codex/workflow/agents/solid_refactor_area_review_retry_core_api_worker.handoff.md`.
  - `solid_refactor_area_review_retry_session_settings_worker.prompt.md` ->
    `.codex/workflow/agents/solid_refactor_area_review_retry_session_settings_worker.handoff.md`.
  - `solid_refactor_area_review_retry_tests_schema_worker.prompt.md` ->
    `.codex/workflow/agents/solid_refactor_area_review_retry_tests_schema_worker.handoff.md`.
- The broader core-api retry was still running without a handoff, so a narrower
  visible fallback was launched:
  - `solid_refactor_area_review_core_api_quick_worker.prompt.md` ->
    `.codex/workflow/agents/solid_refactor_area_review_core_api_quick_worker.handoff.md`.
- Core-api root fallback review was completed while visible core-api workers
  were still running:
  - `.codex/workflow/agents/solid_refactor_area_review_core_api_root_review.handoff.md`.
  - Result: no direct source consumer fallout found; keep core-api/domain source
    plus `Cargo.lock` separate from app-server schema JSON and run required
    release/Bazel lock verification after source blockers are fixed.
- Visible implementation workers launched for scoped fixes:
  - `solid_refactor_fix_session_workspace_roots_worker.prompt.md`, PowerShell
    PID `26904`; owns workspace-root/session settings propagation.
  - `solid_refactor_fix_agent_depth_policy_worker.prompt.md`, PowerShell PID
    `25236`; owns resume-descendant depth policy routing.
  - `solid_refactor_fix_replacement_shadow_dep_worker.prompt.md`, PowerShell
    PID `23324`; owns dead `codex-replacement-shadow` dependency cleanup.
- `solid_refactor_commit_grouping_worker.prompt.md` exists locally as a possible
  read-only grouping prompt, but it has not been launched. Do not launch it
  until the blocking source-review questions below are handled.
- Durable orchestration/docs slice was committed and pushed:
  `55cbc90c48 Document SOLID refactor handoff rules`.
- Review findings doc: `.codex/workflow/solid-refactor-review-findings.md`.
- Active scoped visible review workers launched with
  `codex-workers -Pattern "solid_refactor_area_review_*.prompt.md"`:
  - `solid_refactor_area_review_agent_tools_worker`: PowerShell PID `31384`;
    handoff target
    `.codex/workflow/agents/solid_refactor_area_review_agent_tools_worker.handoff.md`.
  - `solid_refactor_area_review_context_ops_worker`: PowerShell PID `5040`,
    Codex PID `16288`; handoff target
    `.codex/workflow/agents/solid_refactor_area_review_context_ops_worker.handoff.md`.
  - `solid_refactor_area_review_core_api_worker`: PowerShell PID `11100`;
    handoff target
    `.codex/workflow/agents/solid_refactor_area_review_core_api_worker.handoff.md`.
  - `solid_refactor_area_review_session_settings_worker`: PowerShell PID
    `20848`, Codex PID `22108`; handoff target
    `.codex/workflow/agents/solid_refactor_area_review_session_settings_worker.handoff.md`.
  - `solid_refactor_area_review_tests_schema_worker`: PowerShell PID `31500`,
    Codex PID `21512`; handoff target
    `.codex/workflow/agents/solid_refactor_area_review_tests_schema_worker.handoff.md`.

Blocking source-review questions before committing Rust/schema groups:

- `CodexThreadSettingsOverrides.workspace_roots` and
  `profile_workspace_roots` are still public override fields, but the current
  dirty `CodexThread::thread_settings_update` destructures and drops them while
  `SessionSettingsUpdate` no longer carries them. Confirm whether this is an
  intentional API removal or repair the data flow before committing the
  session/thread slice.
- Verify `replacement_shadow.rs` deletion against module/manifests and later
  remove any now-dead `codex-context-ops-impl` /
  `codex-replacement-shadow` dependencies only after source references are
  clean.

## Historical Wave 4 Launch Record (Superseded)

Launched with:

```powershell
codex-workers -Pattern "solid_refactor_wave4_*.prompt.md"
```

Visible worker prompts:

- `.codex/workflow/agents/solid_refactor_wave4_context_ops_boundary_worker.prompt.md`
  - Handoff target: `.codex/workflow/agents/solid_refactor_wave4_context_ops_boundary_worker.handoff.md`
  - Owns the dependency-scout context-ops/replacement-shadow boundary finding.
- `.codex/workflow/agents/solid_refactor_wave4_core_api_consumer_worker.prompt.md`
  - Handoff target: `.codex/workflow/agents/solid_refactor_wave4_core_api_consumer_worker.handoff.md`
  - Owns direct consumer/import fallout from the core-api identifier boundary.
- `.codex/workflow/agents/solid_refactor_wave4_stale_test_api_repair_worker.prompt.md`
  - Handoff target: `.codex/workflow/agents/solid_refactor_wave4_stale_test_api_repair_worker.handoff.md`
  - Owns stale test API repair from the compile-repair queue after wave-3 test
    splits.

Last observed wave-4 launch:

- `solid_refactor_wave4_context_ops_boundary_worker`: visible PowerShell PID
  `2808`, Codex PID `30116`.
- `solid_refactor_wave4_core_api_consumer_worker`: visible PowerShell PID
  `20068`, Codex PID `9292`.
- `solid_refactor_wave4_stale_test_api_repair_worker`: visible PowerShell PID
  `10532`, Codex PID `1124`.

Last observed status:

- No `cargo`, `rustc`, `link`, or `cl` processes were running.
- Wave-4 visible logs exist as
  `.codex/workflow/agents/solid_refactor_wave4_*.exec.visible.log`.

## Root Next Actions

1. Check no build/link processes are running before verification:
   - `Get-Process cargo,rustc,link,cl -ErrorAction SilentlyContinue`
2. Resolve or explicitly classify the workspace-root override drop before
   committing the session/thread settings slice.
3. Integrate completed source slices by ownership.
4. Use dependency-scout and manifest-planner handoffs to decide the smallest
   manifest/Bazel/lock/schema follow-up.
5. Then run `just fmt`, focused release checks/tests, and scoped `just fix -p`
   only for changed crates/targets.
6. Group the dirty tree into useful, verified commits by ownership and push
   those commits before opening unrelated refactor work. If a worker could not
   commit because it was read-only or command-banned, root owns that
   verification/commit/push handoff.

Update this handoff after each integration pass and before the next compaction.

## Compact Checkpoint - 2026-05-22 01:50 Europe/Kyiv

- This checkpoint was added after the Wave 11-13 director pass. It preserves the
  earlier handoff content above and records only the new state from the latest
  visible worker waves.
- Wave 11 handoffs classified:
  - `solid_refactor_wave11_permissions_split_worker.handoff.md`: accepted / root-wired.
  - `solid_refactor_wave11_unified_exec_split_worker.handoff.md`: conflict / blocked; do not revive duplicate unregistered unified-exec wrappers without fresh ownership review.
  - `solid_refactor_wave11_client_websockets_split_worker.handoff.md`: root-wiring-needed before Wave 12.
  - `solid_refactor_wave11_hooks_split_worker.handoff.md`: root-wiring-needed before Wave 12.
  - `solid_refactor_wave11_client_residual_split_worker.handoff.md`: root-wiring-needed before Wave 12.
  - `solid_refactor_wave11_search_otel_model_split_worker.handoff.md`: root-wiring-needed before Wave 12.
  - `solid_refactor_wave11_core_tools_dependency_worker`: repair-needed / missing handoff before Wave 12 recovery.
- Wave 12 results:
  - `solid_refactor_wave12_manifest_wiring_worker.handoff.md`: accepted. It added missing explicit `codex-rs/core/Cargo.toml` integration-test targets for existing Wave 11 wrapper files while preserving compatibility aggregates.
  - `solid_refactor_wave12_core_tools_dependency_recovery_worker.handoff.md`: root-wiring-needed. It confirmed the Wave 11 core/tools source extraction is coherent, but remaining `codex_core -> codex_tools` imports and manifest/dependency ownership are not closed.
- Wave 13 result:
  - `solid_refactor_wave13_tools_domain_manifest_wiring_worker.handoff.md`: accepted. It added `codex-tool-registry-api = { workspace = true }` to `codex-rs/tools/Cargo.toml`, confirmed the existing workspace/tool-execution wiring, and deferred `Cargo.lock`, Bazel metadata, builds, and tests per source-static constraints.
- Checkpoint static checks:
  - `git diff --check -- .codex\workflow\solid-refactor-handoff.md .codex\workflow\agents\solid_refactor_wave13_tools_domain_manifest_wiring_worker.handoff.md .codex\workflow\agents\solid_refactor_wave13_tools_domain_manifest_wiring_worker.prompt.md codex-rs\tools\Cargo.toml codex-rs\tools-domain\tool-registry-api\Cargo.toml codex-rs\Cargo.toml` passed with line-ending warnings only.
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-cargo-dependency-boundaries.ps1 -SolidRefactor -Json` passed with `violation_count = 0`.
- Overseer requested a stop-and-compact boundary here. After compaction or resume, reread:
  - `.codex/workflow/solid-refactor-overseer-memo.md`
  - `.codex/workflow/solid-refactor-handoff.md`
  - `docs/current-project-architecture-solid-refactor-plan.md`
  - `docs/current-project-architecture-solid-review.md`
  - fresh worker handoffs under `.codex/workflow/agents/`
- Continue using real separate visible worker windows via `codex-workers`. Do not run broad builds/tests or broad self-review; delegate reviews/source work to workers.
- Best next worker candidates:
  - `solid_refactor_wave14_core_tools_source_boundary_worker`: inspect and repair remaining source-level `codex_core` / `codex_tools` coupling through extracted `codex-tool-registry-api` and `codex-tool-execution-api` boundaries.
  - `solid_refactor_wave14_core_manifest_followup_worker`: only if the source-boundary worker proves `codex-rs/core/Cargo.toml` requires direct dependency cleanup or additions; report lock/Bazel fallout without refreshing generated metadata.

## Live Wave 14 - 2026-05-22 02:00 Europe/Kyiv

- Self-review finding fixed before launch: the prior checkpoint rewrite had
  replaced too much of this source-of-truth handoff. The previous handoff body
  is preserved above, with the Wave 11-13 checkpoint appended.
- Self-review finding fixed before launch: next-worker names now use Wave 14,
  not the already-completed Wave 13.
- `solid_refactor_wave14_core_tools_source_boundary_worker.prompt.md` prepared
  and launched through `codex-workers`.
  - Visible worker PID: `11336`.
  - Owns only `codex-rs/core/src/client.rs`,
    `codex-rs/core/src/client_common.rs`, `codex-rs/core/src/goals.rs`, and
    `codex-rs/core/src/tools/code_mode/mod.rs`, plus its handoff.
  - Must not edit manifests, lockfiles, Bazel files, generated schemas,
    activation/deploy files, commits, staging, or unrelated Rust modules.
- Pre-launch checks:
  - `git diff --check -- .codex/workflow/solid-refactor-handoff.md .codex/workflow/agents/solid_refactor_wave14_core_tools_source_boundary_worker.prompt.md` passed with line-ending warnings only.
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-cargo-dependency-boundaries.ps1 -SolidRefactor -Json` passed with `violation_count = 0`.

## Director Checkpoint - 2026-05-22 03:14 Europe/Kyiv

- Post-self-review continuation state:
  - Commit anchor named by the overseer as `b4195b2` was not resolvable in this checkout during the prior evidence pass; current HEAD observed then was `f46542d3de`.
  - Recent director workflow script slice was source-only / workflow-only; PowerShell parser checks were run for the changed director scripts before this checkpoint.
  - No product/refactor source was edited by the director.
- Wave 16 handoffs now present and classified:
  - `solid_refactor_wave16_core_tools_surface_scout_worker.handoff.md`: `accepted`, read-only scout. It found the remaining `codex_tools` surface is broader than the earlier two-source-file slice and grouped the next work into tool spec/name/registry, session/runtime tool config, connector/plugin discovery metadata, function/original-image helper facades, and tests.
  - `solid_refactor_wave16_function_image_boundary_worker.handoff.md`: `accepted`, source slice complete. Worker moved the function-call error facade and original-image-detail facade behind narrow APIs; no handler/config/session/plugin/manifest/lock/Bazel/generated-schema files were edited.
  - `solid_refactor_wave16_core_tools_config_boundary_worker.handoff.md`: `accepted`, source slice complete. Worker added `McpToolExposureConfig` to `codex-tool-registry-api` and updated core callers so MCP exposure config no longer imports `codex_tools` directly.
  - `solid_refactor_wave16_connectors_discoverable_boundary_worker.handoff.md`: `accepted`, source slice complete and dependency-boundary check passed. Worker moved discoverable tool/plugin config projection behind the tool-registry API; handler/config references remain outside that worker ownership.
  - `solid_refactor_wave16_core_tool_handlers_api_worker.handoff.md`: `accepted`, source slice complete for straightforward handler/spec API cleanup. It left plugin/discoverable-adjacent target-type references for separate ownership.
  - `solid_refactor_wave16_workspace_roots_path_scout_worker.handoff.md`: `stale-review-prompt`, no source changes. Prompt targeted old app-server paths; handoff identifies current owner files and recommends a follow-up worker against the current session/app-server ownership surface.
- Remaining refactor direction:
  - Root should rerun only source/static checks after integrating worker handoffs: `git diff --check`, targeted `rg`, changed `.ps1` parser checks if applicable, and `scripts\check-cargo-dependency-boundaries.ps1 -SolidRefactor -Json`.
  - Do not start broad builds/tests/schema/Bazel/lock/release/activation/deploy/commit work until architecture docs and this handoff show the target boundaries are closed.
  - Next source worker wave should use wave16 handoffs as the authoritative surface map and avoid reusing stale app-server paths from the blocked workspace-roots prompt.

## Context Budget Protocol - 2026-05-22 03:14 Europe/Kyiv

Problem:

- The visible director session grows context quickly while rereading handoffs, classifying workers, updating this file, and monitoring visible worker windows.
- Manual compaction has been needed near 50 percent because `/compact` appears unreliable after roughly 60 percent in the current launcher path.
- The director must stay compact and must not spend root context on broad self-review or repeated historical reconstruction.

Systematic operating rule:

1. Keep root/director reads narrow:
   - Reread only the overseer memo, this handoff, the two architecture docs, and fresh worker handoffs needed for the current wave.
   - Do not reread broad source or broad diffs in the director session; spawn scouts/workers for that.
2. Delegate context hygiene to a persistent visible sidekick:
   - Prompt file: `.codex/workflow/agents/solid_refactor_context_budget_sidekick.prompt.md`.
   - Launch with `codex-workers -Pattern "solid_refactor_context_budget_sidekick.prompt.md"` when no active sidekick handoff/marker is present.
   - Sidekick is read-only except for its own handoff under `.codex/workflow/agents/`.
   - Sidekick watches handoff staleness, context-growth causes, duplicate director work, and compact-boundary timing, then writes concise recommendations only.
3. Compact earlier:
   - Treat 45-50 percent context as the normal safe boundary.
   - Before compaction, root updates this handoff with current wave state, visible worker PIDs/markers, checks run, and the exact next prompt pattern to launch.
   - After compaction, root rereads the overseer memo, this handoff, architecture plan/review, and fresh worker handoffs before any launch or classification.
4. Push repeated director work outward:
   - Broad source scans -> scout workers.
   - Source edits -> narrow owner workers.
   - Broad review -> review workers.
   - Handoff freshness / token-budget review -> context-budget sidekick.
5. Root retains only integration decisions:
   - Classify worker handoffs.
   - Update this handoff.
   - Choose/prepare/launch the next visible `codex-workers` wave.
   - Run allowed source/static checks only after workers finish.

Sidekick first assignment:

- Inspect recent director checkpoints and worker-handoff flow only under `.codex/workflow/`.
- Produce `.codex/workflow/agents/solid_refactor_context_budget_sidekick.handoff.md` with:
  - Estimated context-growth drivers.
  - Concrete compact trigger recommendation.
  - Which director duties can be delegated in the next two waves.
  - Any stale handoff claims or missing worker-state entries.
  - No product/refactor source edits and no broad builds/tests.

Verification for this workflow slice:

- `git diff --check -- .codex/workflow/solid-refactor-handoff.md .codex/workflow/agents/solid_refactor_context_budget_sidekick.prompt.md`
- `codex-workers -DryRun -Pattern "solid_refactor_context_budget_sidekick.prompt.md"`
- Sidekick handoff verification must include a direct trailing-whitespace check if the handoff is still untracked:
  `$path = '.codex\workflow\agents\solid_refactor_context_budget_sidekick.handoff.md'; $bad = Select-String -Path $path -Pattern '[ \t]+$'; if ($bad) { $bad | ForEach-Object { "$($_.Path):$($_.LineNumber): trailing whitespace" }; exit 1 }`

Launch:

- Prompt created: `.codex/workflow/agents/solid_refactor_context_budget_sidekick.prompt.md`.
- Dry run passed, then launched with:
  `codex-workers -Pattern "solid_refactor_context_budget_sidekick.prompt.md"`.
- Visible worker PID: `21464`.
- Expected handoff: `.codex/workflow/agents/solid_refactor_context_budget_sidekick.handoff.md`.
- Self-review note: the original sidekick prompt relied only on `git diff --check` for the sidekick handoff; that does not check newly created untracked files. The prompt and this handoff now include the direct trailing-whitespace check. Because PID `21464` was already launched with the older prompt, root must apply the corrected sidekick-handoff verification when that handoff appears.
- Self-review launcher fix: `codex-workers -DryRun -Pattern "solid_refactor_context_budget_sidekick.prompt.md"` failed while PID `21464` was active because `start-codex-workers.ps1` tried to delete the active `.exec.visible.log` before honoring `-DryRun`. The launcher now skips log cleanup in dry-run mode; parser-check this script before compacting.

## Self-Review Follow-Up - 2026-05-22 03:22 Europe/Kyiv

- Review finding fixed:
  - `solid_refactor_context_budget_sidekick.prompt.md` originally told the sidekick to rely on `git diff --check` for its own new handoff. That is insufficient for untracked files. The prompt and this handoff now require a direct trailing-whitespace check for the sidekick handoff.
- Launcher finding fixed:
  - `codex-workers -DryRun -Pattern "solid_refactor_context_budget_sidekick.prompt.md"` failed while PID `21464` held the visible log open. `.codex/workflow/agents/start-codex-workers.ps1` now skips log deletion when `-DryRun` is set.
- Sidekick handoff:
  - `.codex/workflow/agents/solid_refactor_context_budget_sidekick.handoff.md` landed with classification `stale-handoff-found`.
  - Corrected direct whitespace verification passed for the sidekick handoff.
  - Sidekick noted root should keep worker-state/PID/marker monitoring out of main context where possible and compact at 45-50 percent.
- Compact worker-state table:

| Worker | PID | Process | Handoff |
| --- | ---: | --- | --- |
| `solid_refactor_wave16_connectors_discoverable_boundary_worker` | 21068 | exited | yes |
| `solid_refactor_wave16_core_tool_handlers_api_worker` | 1168 | exited | yes |
| `solid_refactor_wave16_core_tools_config_boundary_worker` | 19732 | exited | yes |
| `solid_refactor_wave16_core_tools_surface_scout_worker` | 19332 | exited | yes |
| `solid_refactor_wave16_function_image_boundary_worker` | 16136 | exited | yes |
| `solid_refactor_wave16_workspace_roots_path_scout_worker` | 19084 | exited | yes |
| `solid_refactor_context_budget_sidekick` | 21464 | alive at check | yes |

- Remaining stale state:
  - `solid_refactor_wave16_workspace_roots_path_scout_worker.handoff.md` remains `stale-review-prompt`; next worker for that area must target current owner files, not the stale prompt paths.
- Current workflow-only verification to rerun after this section:
  - PowerShell parser check for `.codex/workflow/agents/start-codex-workers.ps1`.
  - `git diff --check -- .codex/workflow/solid-refactor-handoff.md .codex/workflow/agents/solid_refactor_context_budget_sidekick.prompt.md .codex/workflow/agents/start-codex-workers.ps1`.
  - Direct trailing-whitespace check for `.codex/workflow/solid-refactor-handoff.md`, `.codex/workflow/agents/solid_refactor_context_budget_sidekick.prompt.md`, `.codex/workflow/agents/solid_refactor_context_budget_sidekick.handoff.md`, and `.codex/workflow/agents/start-codex-workers.ps1`.
  - `codex-workers -DryRun -Pattern "solid_refactor_context_budget_sidekick.prompt.md"`.

## Wave 17 Prepared - 2026-05-22 03:26 Europe/Kyiv

- Prepared prompt files:
  - `.codex/workflow/agents/solid_refactor_wave17_remaining_core_tools_surface_scout_worker.prompt.md`
  - `.codex/workflow/agents/solid_refactor_wave17_plugin_discoverable_target_boundary_worker.prompt.md`
  - `.codex/workflow/agents/solid_refactor_wave17_workspace_roots_current_static_review_worker.prompt.md`
  - `.codex/workflow/agents/solid_refactor_wave17_handoff_freshness_monitor.prompt.md`
- Delegation intent:
  - Root keeps only classification/integration.
  - Source scan goes to the read-only remaining core tools surface scout.
  - Source edit goes to the plugin/discoverable target boundary worker with ownership limited to `codex-rs/core/src/tools/**` plus narrow tools-domain API files if needed.
  - Stale workspace-roots prompt repair goes to a read-only current-path static review worker.
  - Prompt/marker/handoff freshness monitoring goes to a workflow-only monitor to keep root context smaller.
- Launch command after preflight:
  - `codex-workers -Pattern "solid_refactor_wave17*.prompt.md"`
- Preflight to run first:
  - `git diff --check -- .codex/workflow/solid-refactor-handoff.md .codex/workflow/agents/solid_refactor_wave17*.prompt.md .codex/workflow/agents/start-codex-workers.ps1`
  - Direct trailing-whitespace check for the four wave17 prompt files.
  - `codex-workers -DryRun -Pattern "solid_refactor_wave17*.prompt.md"`
- Preflight passed:
  - `git diff --check` passed for the handoff, wave17 prompts, and `start-codex-workers.ps1`.
  - Direct trailing-whitespace check passed for the handoff, wave17 prompts, and `start-codex-workers.ps1`.
  - PowerShell parser check passed for `start-codex-workers.ps1`.
  - `codex-workers -DryRun -Pattern "solid_refactor_wave17*.prompt.md"` passed after the dry-run log-lock fix.
- Launched with:
  - `codex-workers -Pattern "solid_refactor_wave17*.prompt.md"`
- Visible worker PIDs:
  - `solid_refactor_wave17_handoff_freshness_monitor`: PID `12424`
  - `solid_refactor_wave17_plugin_discoverable_target_boundary_worker`: PID `21096`
  - `solid_refactor_wave17_remaining_core_tools_surface_scout_worker`: PID `19780`
  - `solid_refactor_wave17_workspace_roots_current_static_review_worker`: PID `13520`
- Expected handoffs:
  - `.codex/workflow/agents/solid_refactor_wave17_handoff_freshness_monitor.handoff.md`
  - `.codex/workflow/agents/solid_refactor_wave17_plugin_discoverable_target_boundary_worker.handoff.md`
  - `.codex/workflow/agents/solid_refactor_wave17_remaining_core_tools_surface_scout_worker.handoff.md`
  - `.codex/workflow/agents/solid_refactor_wave17_workspace_roots_current_static_review_worker.handoff.md`

## Context Budget Direction Update - 2026-05-22 03:31 Europe/Kyiv

- Overseer clarification:
  - Do not keep a persistent sidekick/overseer for compaction or token-budget logic.
  - Another agent owns the compaction-script work around the earlier boundary, so the director should not add complicated compaction automation here.
  - The useful adjustment is simpler: delegate more of the director's ordinary repeat work to bounded workers.
- Updated operating rule:
  - Use workflow helpers only as short-lived, one-off workers when they replace root busywork.
  - Close or let helper sessions end after their handoff is captured; if a helper itself grows large, compact/stop it around 30 percent rather than keeping it alive for the whole refactor loop.
  - Do not respawn `solid_refactor_context_budget_sidekick` as a persistent service.
  - Keep future context savings focused on delegation of concrete work: source scouts, source owner edits, stale-path reviews, broad review, and handoff freshness checks.
- Current helper state:
  - `solid_refactor_context_budget_sidekick` produced `.codex/workflow/agents/solid_refactor_context_budget_sidekick.handoff.md` and PID `21464` is no longer running.
  - `solid_refactor_wave17_handoff_freshness_monitor` is a one-off wave17 workflow worker, not a persistent sidekick. Do not keep it alive or restart it after its handoff unless a later wave has a concrete freshness-check need.
- Current wave17 process state at this checkpoint:
  - `solid_refactor_wave17_handoff_freshness_monitor`: PID `12424`, alive.
  - `solid_refactor_wave17_plugin_discoverable_target_boundary_worker`: PID `21096`, alive.
  - `solid_refactor_wave17_remaining_core_tools_surface_scout_worker`: PID `19780`, alive.
  - `solid_refactor_wave17_workspace_roots_current_static_review_worker`: PID `13520`, alive.
  - No wave17 handoffs had landed yet at this check.

## Wave 17 Partial Results - 2026-05-22 03:36 Europe/Kyiv

- Completed handoffs:
  - `solid_refactor_wave17_remaining_core_tools_surface_scout_worker.handoff.md`: `accepted`, read-only. Current scan found 256 `rg -n` matching lines, 257 literal `codex_tools` occurrences, and 62 affected Rust files under `codex-rs/core/src`; `codex-rs/core/tests` had 0 affected files. Remaining ownership areas include code-mode freeform tools, shell/unified-exec runtime config, handler/spec surface, connector/discoverable/plugin/search shapes, session/runtime tool config, and source-local tests.
  - `solid_refactor_wave17_handoff_freshness_monitor.handoff.md`: `accepted`, workflow-only. Prompt quality was acceptable and it confirmed the wave17 worker-state/checkpoint shape; this was a one-off helper and should not be kept alive or restarted as an overseer.
  - `solid_refactor_wave17_workspace_roots_current_static_review_worker.handoff.md`: `accepted`, read-only static review. The stale workspace-roots concern is accepted as already repaired in the current layout; old app-server paths are absent, live ownership is in session configuration plus app-server thread/turn processors, and no concrete bug or missing-test finding was found. No repair worker is recommended for this concern.
- Still pending:
  - `solid_refactor_wave17_plugin_discoverable_target_boundary_worker`: PID `21096`, still alive at check.
- Completed worker processes:
  - `solid_refactor_wave17_handoff_freshness_monitor`: PID `12424`, exited.
  - `solid_refactor_wave17_remaining_core_tools_surface_scout_worker`: PID `19780`, exited.
- Completed handoff but process still visible:
  - `solid_refactor_wave17_workspace_roots_current_static_review_worker`: PID `13520`, handoff landed and direct handoff verification passed.
- Verification:
  - `git diff --check` passed for the two completed wave17 handoffs.
  - Direct trailing-whitespace checks passed for the two completed wave17 handoffs.
  - `git diff --check` and direct trailing-whitespace check passed for `solid_refactor_wave17_workspace_roots_current_static_review_worker.handoff.md`.
- Next director action:
  - Wait for the pending plugin/discoverable target boundary handoff, then classify and update this handoff.
  - Do not spawn a new sidekick or monitor unless a concrete later-wave freshness check replaces root work.

## Wave 18 Broad Parallel Wave Prepared - 2026-05-22 03:45 Europe/Kyiv

- Overseer direction: be more aggressive, not conservative. Goal is to finish refactor work as much as possible and get to a new working build with faster tests after the architecture refactor, without broad builds/tests before boundaries are closed.
- Current active worker to avoid colliding with:
  - `solid_refactor_wave17_plugin_discoverable_target_boundary_worker`, PID `21096`, still running and owning plugin/discoverable-adjacent `codex-rs/core/src/tools/**`.
- Prepared broader wave18 prompt files:
  - `solid_refactor_wave18_core_api_app_server_protocol_boundary_scout_worker.prompt.md`: source-capable boundary worker for reducing `codex-core-api` dependency on app-server protocol; edits only its narrow owner files if clear.
  - `solid_refactor_wave18_code_mode_freeform_boundary_worker.prompt.md`: source worker for `codex-rs/core/src/tools/code_mode/**`.
  - `solid_refactor_wave18_shell_unified_exec_boundary_scout_worker.prompt.md`: read-only scout for shell/unified-exec config ownership.
  - `solid_refactor_wave18_apply_patch_test_binary_split_worker.prompt.md`: source worker for apply-patch test binary splits and the relevant `codex-rs/core/Cargo.toml` test entries.
  - `solid_refactor_wave18_compact_test_binary_split_worker.prompt.md`: source worker for compact test binary splits and the relevant `codex-rs/core/Cargo.toml` test entries.
  - `solid_refactor_wave18_responses_headers_test_split_worker.prompt.md`: source worker for responses-header test split if a clean split exists.
  - `solid_refactor_wave18_core_tests_agents_split_scout_worker.prompt.md`: read-only scout for agent test split planning.
  - `solid_refactor_wave18_core_tests_mcp_tooling_split_scout_worker.prompt.md`: read-only scout for MCP/tooling test splits.
  - `solid_refactor_wave18_core_tests_session_runtime_split_scout_worker.prompt.md`: read-only scout for session/runtime test splits.
  - `solid_refactor_wave18_core_tests_support_dependency_scout_worker.prompt.md`: read-only scout for high-fan-in test support dependency reduction.
- Parallelism policy for wave18:
  - More sessions are intentional here. The source workers have bounded owners; the scouts prepare the next source wave without consuming root context.
  - Multiple test split workers may need `codex-rs/core/Cargo.toml`; their prompts require `root-wiring-needed` rather than overwriting concurrent manifest edits if they collide.
  - No worker may run broad builds/tests/schema/Bazel/lock/release/deploy/activation. The old blanket commit ban is stale: workers with explicit commit stewardship or source ownership may commit coherent verified slices, stage only owned files, and must report commit hashes, checks, skipped dirty work, and fallout.
- Launch command after preflight:
  - `codex-workers -Pattern "solid_refactor_wave18*.prompt.md"`
- Preflight:
  - `git diff --check -- .codex/workflow/solid-refactor-handoff.md .codex/workflow/agents/solid_refactor_wave18*.prompt.md`
  - Direct trailing-whitespace check for all wave18 prompt files and this handoff.
  - `codex-workers -DryRun -Pattern "solid_refactor_wave18*.prompt.md"`

## Continuation Checkpoint - 2026-05-22 04:32 Europe/Kyiv

- Director reread `.codex/workflow/solid-refactor-overseer-memo.md`, this handoff, `docs/current-project-architecture-solid-refactor-plan.md`, `docs/current-project-architecture-solid-review.md`, and fresh wave18 handoffs.
- Fresh handoff classification:
  - accepted: `solid_refactor_wave18_apply_patch_test_binary_split_worker.handoff.md` split six apply-patch binaries and reported no root wiring needed.
  - accepted: `solid_refactor_wave18_compact_test_binary_split_worker.handoff.md` split compact binaries and manifest entries.
  - accepted: `solid_refactor_wave18_responses_headers_test_split_worker.handoff.md` split response/header binaries.
  - accepted: `solid_refactor_wave18_code_mode_freeform_boundary_worker.handoff.md` removed `codex_tools` from the code-mode freeform ownership area.
  - accepted/read-only: `solid_refactor_wave18_core_api_app_server_protocol_boundary_scout_worker.handoff.md` found no direct current `core-api` app-server-protocol dependency.
  - accepted/read-only: `solid_refactor_wave18_core_tests_agents_split_scout_worker.handoff.md` recommends splitting `agents_runtime` into `agents_jobs`, `agents_delegate`, `agents_hierarchy`, and `agents_tool_parallelism`.
  - accepted/read-only: `solid_refactor_wave18_shell_unified_exec_boundary_scout_worker.handoff.md` recommends moving shell/unified-exec config/types out of the `codex-tools` dependency path.
  - accepted/read-only: `solid_refactor_wave18_core_tests_session_runtime_split_scout_worker.handoff.md` identified more session/runtime split slices, including compact/resume/fork, streaming errors, unified exec sessions, realtime conversation, state conversation, and review history.
  - accepted/read-only: `solid_refactor_wave18_core_tests_mcp_tooling_split_scout_worker.handoff.md` identified MCP/tooling split families including code-mode, RMCP client, and search-tool binaries.
  - accepted/read-only: `solid_refactor_wave18_core_tests_support_dependency_scout_worker.handoff.md` identified high-fan-in test-support dependency reduction opportunities.
- Commit policy correction:
  - Root/director still does not commit product/source changes directly.
  - Workers should commit coherent useful verified slices when their prompt grants commit stewardship or source ownership and their source/static checks pass.
  - Commits must be focused: stage only owned files, avoid unrelated dirty work, and do not include generated/schema/Bazel/lock/deploy artifacts unless explicitly assigned.
  - Worker handoffs must report commit hashes, files included, checks run, skipped/conflicting dirty work, and remaining fallout.
- Current process note:
  - A process check showed newer `codex`/`pwsh` processes still alive. Treat the tree as moving. New wave prompts must read fresh status and must not overwrite unrelated edits.
  - Worker launcher marker metadata was corrected so it no longer advertises a blanket git staging/commit ban; commit permission is prompt-specific.
- Next visible wave prepared:
  - `solid_refactor_wave19_commit_integrity_worker.prompt.md`: commit steward for already-completed accepted worker slices; no broad builds/tests.
  - `solid_refactor_wave19_core_tools_client_goals_boundary_worker.prompt.md`: remove remaining `codex_tools` use from `client.rs`/`goals.rs` path by depending on narrow abstractions/domain crates.
  - `solid_refactor_wave19_shell_unified_exec_boundary_worker.prompt.md`: implement the shell/unified-exec config boundary recommended by the wave18 scout.
  - `solid_refactor_wave19_agents_runtime_split_worker.prompt.md`: implement the agents runtime test split recommended by the wave18 scout.
  - `solid_refactor_wave19_code_mode_tests_split_worker.prompt.md`: split code-mode MCP/tooling tests by topic.
  - `solid_refactor_wave19_rmcp_client_tests_split_worker.prompt.md`: split RMCP client tests by transport/response topic.
  - `solid_refactor_wave19_search_tool_tests_split_worker.prompt.md`: split search-tool tests by matching/deferred/dynamic/MCP topic.
  - `solid_refactor_wave19_core_tests_support_dependency_worker.prompt.md`: shrink test-support dependency fan-in using the wave18 scout.
- Source/static checks for this director slice:
  - `git diff --check -- .codex/workflow/solid-refactor-handoff.md .codex/workflow/agents/solid_refactor_wave19*.prompt.md`
  - `codex-workers -DryRun -Pattern "solid_refactor_wave19*.prompt.md"`
- Launch command:
  - `codex-workers -Pattern "solid_refactor_wave19*.prompt.md"`

## Wave 19 Launched - 2026-05-22 04:39 Europe/Kyiv

- Director-only changes made:
  - Updated this handoff with current wave18 classifications, prompt-specific commit policy, and wave19 plan.
  - Added eight wave19 prompt files under `.codex/workflow/agents/`.
  - Updated `.codex/workflow/agents/start-codex-workers.ps1` marker metadata so it no longer advertises a blanket git staging/commit ban.
  - Mechanically corrected current wave19 marker metadata after launch; this does not change worker prompts.
- Checks run by director:
  - `git diff --check -- .codex/workflow/solid-refactor-handoff.md .codex/workflow/agents/solid_refactor_wave19*.prompt.md .codex/workflow/agents/start-codex-workers.ps1 .codex/workflow/agents/solid_refactor_wave19*.exec.marker.txt`
    - Exit 0; only existing CRLF warning for this handoff.
  - PowerShell parser check on `.codex/workflow/agents/start-codex-workers.ps1`
    - Exit 0.
  - `codex-workers -DryRun -Pattern "solid_refactor_wave19*.prompt.md"`
    - Exit 0.
- Visible worker launch:
  - `solid_refactor_wave19_agents_runtime_split_worker` PID 6168.
  - `solid_refactor_wave19_code_mode_tests_split_worker` PID 19700.
  - `solid_refactor_wave19_commit_integrity_worker` PID 22368.
  - `solid_refactor_wave19_core_tests_support_dependency_worker` PID 3808.
  - `solid_refactor_wave19_core_tools_client_goals_boundary_worker` PID 3860.
  - `solid_refactor_wave19_rmcp_client_tests_split_worker` PID 24456.
  - `solid_refactor_wave19_search_tool_tests_split_worker` PID 20488.
  - `solid_refactor_wave19_shell_unified_exec_boundary_worker` PID 11580.
- Initial monitor:
  - After roughly 35 seconds, no `solid_refactor_wave19*.handoff.md` files existed yet.
  - All eight launcher PIDs were still running.
  - Log tail sampling showed workers had started; do not infer acceptance until handoffs are written.
- Next director action:
  - Monitor wave19 handoffs.
  - If many visible workers are still running and no new handoff/actionable failure exists, sleep between monitor passes instead of polling rapidly. A reasonable cadence is 60-180 seconds during active worker execution, with longer sleeps acceptable when logs show normal progress. Do not run `git status`, process scans, or log tails every few seconds just to fill time.
  - Classify each as `accepted`, `root-wiring-needed`, `repair-needed`, or `conflict/blocked`.
  - Do not repair source in root. Spawn repair workers for source fallout.

## Self-Review / Wave 19 Monitor Update - 2026-05-22 04:58 Europe/Kyiv

- Trigger: automatic post-self-review loop continuation after the wave19 launch and monitor-cadence handoff update.
- Anchor note: the overseer supplied anchor `a3dd3ab804a3f977ef5ae41d2474d363d3838e26`, but the object was not present in this checkout when checked. The tree is moving under active workers; rely on current handoffs and current `HEAD`, not that anchor.
- Review finding fixed in root:
  - This handoff was stale after new wave19 handoffs arrived. Root updated it instead of doing broad source review.
- Fresh wave19 classifications observed:
  - accepted: `solid_refactor_wave19_agents_runtime_split_worker.handoff.md`.
    - Added standalone agents runtime wrapper binaries and manifest entries.
    - Worker left the slice unstaged because `Cargo.toml` also has unrelated concurrent edits.
  - root-wiring-needed: `solid_refactor_wave19_rmcp_client_tests_split_worker.handoff.md`.
    - Worker reported wrapper files/manifest entries and a passed `git diff --check`.
    - Needs root integration/commit sequencing because `codex-rs/core/Cargo.toml` is shared with other split workers.
  - root-wiring-needed: `solid_refactor_wave19_search_tool_tests_split_worker.handoff.md`.
    - Worker reported search split wrappers/manifest entries and a passed `git diff --check`.
    - Needs root integration/commit sequencing because `codex-rs/core/Cargo.toml` is shared with other split workers.
  - root-wiring-needed: `solid_refactor_wave19_core_tests_support_dependency_worker.handoff.md`.
    - Worker reported support dependency changes and remaining decision around moving/keeping `core_test_support::responses`.
    - Needs source repair/integration worker, not root source edits.
- Wave19 handoffs not yet present at this checkpoint:
  - `solid_refactor_wave19_core_tools_client_goals_boundary_worker.handoff.md`
  - `solid_refactor_wave19_shell_unified_exec_boundary_worker.handoff.md`
  - `solid_refactor_wave19_commit_integrity_worker.handoff.md`
- Director constraints for next resume:
  - Do not fix source in root.
  - Do not run broad builds/tests/schema/Bazel/lock/release/deploy/activation.
  - Wait/sleep between monitor passes while workers are active and no handoff changed; 60-180 seconds is acceptable.
  - Next useful action is to reread the four present wave19 handoffs, then wait for the missing four or spawn narrow repair/integration workers only after the current workers are done or blocked.

## Wave 19 Monitor Update - 2026-05-22 05:02 Europe/Kyiv

- New handoff after a slow monitor wait:
  - accepted: `solid_refactor_wave19_code_mode_tests_split_worker.handoff.md`.
    - Worker reported six code-mode split wrapper binaries and a passed `git diff --check`.
    - It also shares `codex-rs/core/Cargo.toml`; commit sequencing should be handled by the commit/integration path, not root source edits.
- Wave19 handoffs still missing at this checkpoint:
  - `solid_refactor_wave19_core_tools_client_goals_boundary_worker.handoff.md`
  - `solid_refactor_wave19_commit_integrity_worker.handoff.md`
- Continue slow monitor cadence. If no new handoff appears, sleep rather than polling rapidly.

## Late Handoff Update - 2026-05-22 05:52 Europe/Kyiv

- New handoff now present after the earlier missing-handoff note:
  - repair-needed:
    `solid_refactor_wave19_core_tools_client_goals_boundary_worker.handoff.md`.
    - Direct `codex_tools` references are now gone from the originally owned
      `client.rs`, `client_common.rs`, and `goals.rs` boundary.
    - `client.rs` now uses
      `codex_tool_registry_api::create_tools_json_for_responses_api`.
    - `goals.rs` now uses `codex_tool_registry_api::UPDATE_GOAL_TOOL_NAME`.
    - The handoff classifies the slice as repair-needed because
      `cargo check --release -p codex-core --lib` still fails in cross-worker
      fallout outside the owned client/goals files, reported around
      incompatible `ToolOutput` method signatures in multi-agent handler
      modules.
    - `just fmt` failed on Windows with path length error 206; direct rustfmt
      on the touched files passed, while `cargo fmt --check` hit unrelated
      parse fallout in `core/tests/suite/items_message_events.rs`.
- This supersedes the earlier line that listed the wave19 client/goals handoff
  as missing. The only older wave19 handoff still listed as missing here is
  `solid_refactor_wave19_commit_integrity_worker.handoff.md`; recheck before
  acting because handoffs are still landing asynchronously.
- Active worker state at this checkpoint:
  - `solid_refactor_wave20_non_shell_handler_specs_boundary_worker` still has
    no handoff.
  - Marker PID `23356` was still alive and the visible log size was
    `2469341` bytes at the 05:53 verification check.
  - Continue slow monitor cadence. Do not launch an overlapping source worker
    for non-shell handler/spec imports or multi-agent `ToolOutput` fallout
    while this worker is still active.
- Next-wave adjustment:
  - Defer `solid_refactor_wave21_client_common_toolspec_boundary_worker` as a
    first action. The latest wave19 client/goals handoff removed the direct
    client/goals `codex_tools` edge; remaining acceptance is blocked by
    cross-worker fallout, not by those files.
  - Keep `solid_refactor_wave21_stream_error_duplicate_repair_worker` as the
    first independent source repair candidate if it does not overlap active
    worker files.
  - Keep the non-shell follow-up and core-tools symbol-family cleanup queued
    behind the active wave20 non-shell handler handoff.

## Wave 20 Handoff Classification Update - 2026-05-22 05:50 Europe/Kyiv

- Fresh wave20 handoffs read and classified:
  - accepted/read-only:
    `solid_refactor_wave20_client_goals_boundary_status_scout.handoff.md`.
    - Wave19 client/goals worker still had no handoff at the scout's
      checkpoint.
    - The scout found `client.rs` and `goals.rs` clear of the scoped
      `codex_tools`, `ToolSpec`, `ToolName`, and
      `ResponsesApiNamespaceTool` references.
    - Remaining scoped follow-up is `codex-rs/core/src/client_common.rs`,
      where `ToolSpec` is still imported and stored in `ClientCommonArgs`.
  - accepted/completed:
    `solid_refactor_wave20_commit_steward_followup_worker.handoff.md`.
    - Recorded handoff-only commits:
      `6cb2dddfed06`, `2f510a537180`, and `5f95b495bfc0`.
    - It left mixed source, schema, lockfile, generated, deploy, and
      activation work unstaged.
  - accepted:
    `solid_refactor_wave20_core_tests_support_responses_repair_worker.handoff.md`.
    - Keeps response helpers owned by `codex-test-support-responses`.
    - Restores thin `core_test_support::{responses, streaming_sse}` adapters
      because non-core topic adapters still import them.
    - Do not treat these compatibility adapters as the final architectural
      target; they are a repair step to keep split test wrappers compiling
      while import owners are migrated.
  - accepted/read-only:
    `solid_refactor_wave20_dependency_boundary_checker_scout.handoff.md`.
    - Boundary checker passed with exit 0, but the scout explicitly notes that
      this does not prove `codex-core -> codex-tools` is clean.
    - Fresh `rg -n "codex_tools" codex-rs/core/src codex-rs/core/tests`
      still found 176 matching lines across 17 files while the non-shell
      handler worker was still running.
  - accepted/commit-ready:
    `solid_refactor_wave20_wave19_split_integration_commit_worker.handoff.md`.
    - Integrated safe Wave19 split binaries for agents runtime, RMCP client,
      and search-tool slices.
    - Left `agents_runtime`, `tools_search`, `tools`, and broader dirty
      manifest/source work unstaged by design.
  - repair-needed:
    `solid_refactor_wave20_compact_resume_streaming_split_worker.handoff.md`.
    - It added focused compact/resume/fork and stream-error wrappers.
    - `codex-rs/core/tests/client_stream.rs` still includes
      `suite/stream_error_allows_next_turn.rs`, so a source-owner repair
      worker should remove the duplicate include before those tests are
      considered cleanly split.
  - root-wiring-needed:
    `solid_refactor_wave20_remaining_mcp_tooling_split_worker.handoff.md`.
    - Added focused wrappers for `tools_mcp_openai_file`,
      `tools_mcp_plugins`, and `tools_mcp_turn_metadata`.
    - Final root integration/staging must reconcile shared
      `codex-rs/core/Cargo.toml`.
  - root-wiring-needed:
    `solid_refactor_wave20_unified_exec_sessions_test_split_worker.handoff.md`.
    - Split unified-exec session coverage into lifecycle, terminal, modes,
      and reuse wrappers.
    - Final root integration/staging must reconcile shared
      `codex-rs/core/Cargo.toml`.
  - root-wiring-needed:
    `solid_refactor_wave20_realtime_state_review_split_worker.handoff.md`.
    - Added `realtime_conversation_startup_context`, focused state
      conversation wrappers, and focused review-history wrappers.
    - Final root integration/staging must reconcile shared
      `codex-rs/core/Cargo.toml`.
- Wave20 worker still running with no handoff yet:
  - `solid_refactor_wave20_non_shell_handler_specs_boundary_worker`.
    - Marker PID `23356` was still a responsive `powershell.exe` process at
      this checkpoint.
    - Handoff file was still absent.
    - Visible log was non-empty and recently written, so classify as
      `still-running/no-handoff`, not failed.
    - Continue slow monitor cadence; do not launch overlapping source workers
      over non-shell handler/spec `codex_tools` imports until this handoff
      lands or the process is proven dead.
- Current boundary snapshot:
  - `scripts/check-cargo-dependency-boundaries.ps1 -SolidRefactor -Json`
    still exits 0, but checker coverage is incomplete for the current
    `codex-core -> codex-tools` edge.
  - `codex-rs/core/Cargo.toml` still has
    `codex-tools = { workspace = true }`.
  - `codex-rs/core/Cargo.toml` already has direct
    `codex-tool-execution-api` and `codex-tool-registry-api` dependencies,
    so many remaining source imports can move without manifest expansion.
  - A direct scan of `codex-rs/core-api` did not find current
    `codex-app-server-protocol` imports or manifest dependencies in this
    checkout. Treat older notes claiming a direct current `core-api` protocol
    edge as stale unless a fresh dependency-graph check proves otherwise.
- Next wave candidates after the active non-shell worker finishes or hands off:
  - `solid_refactor_wave21_non_shell_handler_specs_followup_worker`
    - Launch only if wave20 non-shell handoff is missing, repair-needed, or
      leaves a clearly bounded remaining symbol family.
    - Own the exact handler/spec files named by the wave20 handoff or boundary
      scout; do not touch client/goals or test split wrappers.
  - `solid_refactor_wave21_client_common_toolspec_boundary_worker`
    - Own `codex-rs/core/src/client_common.rs` plus the smallest required
      boundary adapter/type.
    - Remove the direct `codex_tool_registry_api::ToolSpec` import from
      `client_common.rs`, preserve the existing `apply_patch` freeform
      detection behavior, and rerun the scoped reference scan.
  - `solid_refactor_wave21_stream_error_duplicate_repair_worker`
    - Own `codex-rs/core/tests/client_stream.rs` and the focused
      `stream_error_allows_next_turn` wrapper/suite only.
    - Remove the duplicate suite include, then rerun the compact/resume/stream
      reference scan and `git diff --check` for those files.
  - `solid_refactor_wave21_core_tools_symbol_family_worker`
    - After non-shell handler/spec work, use the dependency scout's remaining
      symbol list to move one concrete family at a time out of direct
      `codex_tools` imports: shell/execution config leftovers, plugin request
      entries/constants, or registry spec types.
    - Keep each family in a narrow API crate already present where possible;
      do not hide the edge through compatibility re-exports.
  - `solid_refactor_wave21_large_suite_split_worker`
    - Target remaining large suite wrappers with clear topic boundaries,
      starting with `suite/hooks.rs` (3303 lines), `suite/client.rs` (2482
      lines), and `suite/compact.rs` (1622 lines).
    - Create separate Cargo test binaries by topic so focused release tests can
      compile and run smaller units.
  - `solid_refactor_wave21_split_manifest_commit_steward`
    - Root-owned or root-supervised staging pass only after the source-owner
      handoffs above are classified.
    - Commit coherent split-test manifest/source slices without staging
      schema, lockfile, generated, deploy, or unrelated active source work.

## 2026-05-22 05:20 EET Wave19 Core Tools Client/Goals Boundary Worker

- Handoff written: `.codex/workflow/agents/solid_refactor_wave19_core_tools_client_goals_boundary_worker.handoff.md`.
- Classification: repair-needed.
- Green scope: owned core client/goals files no longer reference `codex_tools`; registry/execution API boundary scripts passed; `cargo check --release -p codex-tool-registry-api -p codex-tool-execution-api -p codex-tools` passed.
- Red scope: `cargo check --release -p codex-core --lib` still fails outside this worker's owned files. Saved log: `logs/wave19-codex-core-lib-check.log`.

## Wave 20 Broad Worker Wave Launched - 2026-05-22 05:22 Europe/Kyiv

- Director reread this handoff, the architecture plan/review docs, and fresh wave19 handoffs before spawning wave20.
- Fresh wave19 classification before wave20:
  - accepted: `solid_refactor_wave19_agents_runtime_split_worker.handoff.md`.
  - accepted: `solid_refactor_wave19_code_mode_tests_split_worker.handoff.md`.
  - accepted: `solid_refactor_wave19_shell_unified_exec_boundary_worker.handoff.md`.
  - root-wiring-needed: `solid_refactor_wave19_rmcp_client_tests_split_worker.handoff.md`.
  - root-wiring-needed: `solid_refactor_wave19_search_tool_tests_split_worker.handoff.md`.
  - root-wiring-needed: `solid_refactor_wave19_core_tests_support_dependency_worker.handoff.md`.
  - partial: `solid_refactor_wave19_commit_integrity_worker.handoff.md`.
    - It created commit `e3df2e25d4e177d759e9f7feb3e87027afab9d01` for session-maintenance workflow tools.
    - It skipped mixed `codex-rs/core/Cargo.toml` slices and noted a separate code-mode split commit existed outside its created-commit set.
  - still missing: `solid_refactor_wave19_core_tools_client_goals_boundary_worker.handoff.md`.
- Wave20 prompt files created:
  - `solid_refactor_wave20_wave19_split_integration_commit_worker.prompt.md`
  - `solid_refactor_wave20_dependency_boundary_checker_scout.prompt.md`
  - `solid_refactor_wave20_core_tests_support_responses_repair_worker.prompt.md`
  - `solid_refactor_wave20_non_shell_handler_specs_boundary_worker.prompt.md`
  - `solid_refactor_wave20_client_goals_boundary_status_scout.prompt.md`
  - `solid_refactor_wave20_compact_resume_streaming_split_worker.prompt.md`
  - `solid_refactor_wave20_unified_exec_sessions_test_split_worker.prompt.md`
  - `solid_refactor_wave20_realtime_state_review_split_worker.prompt.md`
  - `solid_refactor_wave20_remaining_mcp_tooling_split_worker.prompt.md`
  - `solid_refactor_wave20_commit_steward_followup_worker.prompt.md`
- Checks before launch:
  - `git diff --check -- .codex/workflow/agents/solid_refactor_wave20*.prompt.md`
    - Exit 0.
  - `codex-workers -DryRun -Pattern "solid_refactor_wave20*.prompt.md"`
    - Exit 0.
- Launch command:
  - `codex-workers -Pattern "solid_refactor_wave20*.prompt.md"`
- Visible wave20 worker PIDs:
  - `solid_refactor_wave20_client_goals_boundary_status_scout` PID 20704.
  - `solid_refactor_wave20_commit_steward_followup_worker` PID 16588.
  - `solid_refactor_wave20_compact_resume_streaming_split_worker` PID 25400.
  - `solid_refactor_wave20_core_tests_support_responses_repair_worker` PID 13952.
  - `solid_refactor_wave20_dependency_boundary_checker_scout` PID 24148.
  - `solid_refactor_wave20_non_shell_handler_specs_boundary_worker` PID 23356.
  - `solid_refactor_wave20_realtime_state_review_split_worker` PID 9464.
  - `solid_refactor_wave20_remaining_mcp_tooling_split_worker` PID 9076.
  - `solid_refactor_wave20_unified_exec_sessions_test_split_worker` PID 18624.
  - `solid_refactor_wave20_wave19_split_integration_commit_worker` PID 11372.
- Resume after compact:
  - First reread `.codex/workflow/solid-refactor-overseer-memo.md`, this handoff, `docs/current-project-architecture-solid-refactor-plan.md`, `docs/current-project-architecture-solid-review.md`, and fresh wave19/wave20 handoffs.
  - Classify wave20 handoffs as `accepted`, `root-wiring-needed`, `repair-needed`, or `conflict/blocked`.
  - Do not repair source in root; spawn repair workers for source fallout.
  - Use slow monitor cadence. If many visible workers are running and no new handoff/actionable failure exists, sleep 60-180 seconds rather than polling rapidly.
  - Continue using `Kyiv`, not `Kiev`, in new user-facing notes.

## Wave 19 Monitor Update - 2026-05-22 05:04 Europe/Kyiv

- New handoff after the next slow monitor wait:
  - accepted: `solid_refactor_wave19_shell_unified_exec_boundary_worker.handoff.md`.
    - Worker reported moving shell/unified-exec config types into `codex-tool-execution-api` and updating shell/unified-exec call sites.
    - Worker reported `rg` still finds `codex_tools` imports in non-shell handler specs outside its boundary.
    - Commit remains for root/clean staging pass because touched files contain unrelated dirty hunks; root must not stage/commit product files directly.
- Wave19 handoffs still missing at this checkpoint:
  - `solid_refactor_wave19_core_tools_client_goals_boundary_worker.handoff.md`
  - `solid_refactor_wave19_commit_integrity_worker.handoff.md`
- Continue slow monitor cadence. If no new handoff appears, sleep rather than polling rapidly.

## Current Root Checkpoint - 2026-05-22 06:06 Europe/Kyiv

This checkpoint supersedes older duplicate "missing handoff" notes above.

- Wave20 handoffs are classified in this handoff under
  "Wave 20 Handoff Classification Update".
- Late wave19 client/goals handoff is now present and classified
  `repair-needed`:
  `solid_refactor_wave19_core_tools_client_goals_boundary_worker.handoff.md`.
- Wave20 non-shell handler/spec worker still has no handoff and remains active
  at the latest local check, with PID `23356` and a growing visible log.
  - 06:06 verification: handoff missing, PID alive, log length `5739656`
    bytes.
- Latest cheap boundary checks:
  - `scripts/check-cargo-dependency-boundaries.ps1 -SolidRefactor -Json`
    returned `violation_count = 0`.
  - `rg -n "codex_tools" codex-rs/core/src codex-rs/core/tests` still
    returned 93 matching lines while the non-shell worker was active.
- Do not launch overlapping source workers for non-shell handler/spec imports,
  multi-agent `ToolOutput` fallout, or remaining `codex_tools` handler/spec
  references until that worker hands off or is proven dead.
- The next independent source repair candidate is the stream-error duplicate
  include in `codex-rs/core/tests/client_stream.rs`, owned by the proposed
  `solid_refactor_wave21_stream_error_duplicate_repair_worker`.
- The next source-boundary cleanup wave should be selected from the active
  non-shell handoff, then followed by a root-supervised manifest/staging pass
  for root-wiring-needed split-test slices.

## Wave 20 Non-Shell Handoff Update - 2026-05-22 06:11 Europe/Kyiv

- New handoff present:
  - accepted/root-wiring-needed:
    `solid_refactor_wave20_non_shell_handler_specs_boundary_worker.handoff.md`.
    - Worker removed direct `codex_tools` imports from the movable non-shell
      handler/spec surface and added narrow registry-domain exports.
    - Worker reports scoped `rg` now leaves only `plan_spec.rs`
      (`root-wiring-needed`) and `shell_spec.rs` (`wave19-owned`) in that
      handler/spec surface.
    - Solid refactor dependency boundary check passed with
      `violation_count = 0`.
    - Source files and handoff are intentionally left unstaged for a later
      source-owner or integration-steward pass.
- Current `rg -n "codex_tools" codex-rs/core/src codex-rs/core/tests` still
  reports 93 matching lines across session code/tests and tool spec tests.
- Queued next worker:
  - `solid_refactor_wave21_stream_error_duplicate_repair_worker.prompt.md`
    owns only the duplicate `stream_error_allows_next_turn` include repair in
    `codex-rs/core/tests/client_stream.rs` plus its focused wrapper/suite
    references.
- Launch check:
  - `codex-workers -List -Pattern "solid_refactor_wave21_stream_error_duplicate_repair_worker.prompt.md"`
    found the prompt.
  - `git diff --check -- .codex/workflow/agents/solid_refactor_wave21_stream_error_duplicate_repair_worker.prompt.md`
    passed.
  - `codex-workers -DryRun -Pattern "solid_refactor_wave21_stream_error_duplicate_repair_worker.prompt.md"`
    exited 0.
  - Launched visible worker with PID `10164`.
- Self-review fallout routed:
  - Root added direct `codex-file-system` and `codex-session-state`
    dependencies to `codex-rs/core/Cargo.toml` because current source imports
    `codex_file_system::LOCAL_FS` and
    `codex_session_state::PreviousTurnSettings`.
  - `solid_refactor_wave21_core_lib_fallout_repair_worker.prompt.md` owns the
    remaining source fallout from `logs/wave19-codex-core-lib-check.log`:
    missing `ToolsConfig` / `ToolsConfigParams` imports and stale
    multi-agent `ToolOutput` method signatures.
  - `codex-workers -DryRun -Pattern "solid_refactor_wave21_core_lib_fallout_repair_worker.prompt.md"`
    exited 0.
  - Launched visible worker with PID `21812`.
