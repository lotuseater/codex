# SOLID Refactor Handoff

Date: 2026-05-21
Status: active refactor-first orchestration; compact-safe current state.

This handoff is the current source of truth. It intentionally omits old launch
history except where it affects active process ownership.

## Continuation Checkpoint - 2026-05-21 13:18 Europe/Kiev

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
- Follow-up at 2026-05-21 13:28 Europe/Kiev: the release lane exited; no
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

## Compaction Checkpoint - 2026-05-21 12:45 Europe/Kiev

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
