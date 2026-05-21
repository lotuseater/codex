# solid_refactor_area_review_tests_schema_worker Handoff

Status: read-only review complete on 2026-05-21. I did not edit source, manifests, schemas, locks, staging, commits, or pushes. I only wrote this handoff as requested.

## Findings

1. **Do not commit the current `codex-core` manifest/lock state as a tests-only slice.**

   Evidence:
   - `codex-rs/core/Cargo.toml:71` starts production `[dependencies]`; `codex-rs/core/Cargo.toml:132` adds `codex-thread-store = { workspace = true }` before `codex-rs/core/Cargo.toml:206` starts `[dev-dependencies]`.
   - The scoped test-support/stale API work imports the concrete store in test paths: `codex-rs/core/tests/common/test_codex.rs:44-46`, `codex-rs/core/src/tools/handlers/multi_agents_tests.rs:55-57`, and `codex-rs/core/tests/suite/client.rs:50-52`.
   - The lockfile reflects this boundary expansion: `codex-rs/Cargo.lock` has a new `codex-thread-store` dependency under the `codex-core` package diff.
   - The manifest planner explicitly calls this out as unresolved source-boundary fallout: `.codex/workflow/agents/solid_refactor_wave3_manifest_planner_worker.handoff.md:413-414`.

   Root-owned next action: before committing the test-support/stale-test repairs, decide whether `codex-core` really owns the concrete `codex-thread-store` dependency. If it is only for tests/helpers, move it to the test-support/dev boundary and regenerate `codex-rs/Cargo.lock`; if production code still needs it, keep that as a separate core-boundary commit with its own verification, not folded into the tests/schema slice.

2. **Schema refresh is source-backed but not commit-ready until the generator lane is rerun and all generated outputs are grouped deliberately.**

   Evidence:
   - Protocol source changes exist in `codex-rs/app-server-protocol/src/protocol/v2/permissions.rs` (`ActivePermissionProfile` / `modifications` / conversion lines around `331-401`), `codex-rs/app-server-protocol/src/protocol/thread_history.rs` (`UserMessageEvent` fixture lines include `image_details` / `local_image_details`), and `codex-rs/app-server-protocol/src/protocol/v2/tests.rs:1714`.
   - Tracked schema JSON is dirty under `codex-rs/app-server-protocol/schema/json`, including `ClientRequest.json`, `ServerRequest.json`, `ServerNotification.json`, multiple `v2/Thread*.json` files, and aggregate schema files.
   - New untracked generated schema files also exist, for example `codex-rs/app-server-protocol/schema/json/FuzzyFileSearchSession*`, `codex-rs/app-server-protocol/schema/json/v2/CollaborationModeList*`, `EnvironmentAdd*`, `Memory*`, `Process*`, `ThreadGoal*`, `ThreadRealtime*`, and `ThreadTurns*` files.
   - Existing root review already warned that generated app-server schema files should be committed only with their DTO/source changes: `.codex/workflow/solid-refactor-review-findings.md:31`.

   Root-owned next action: rerun the schema generation lane from a stable source tree, review tracked and untracked JSON together, then commit the app-server protocol source, tests, and all intentional generated schema files in one schema/API commit. If any new JSON was produced only by a stale generator/configuration run, exclude it rather than mixing it into unrelated test commits.

3. **Bazel BUILD scaffolds and workflow handoffs are separate follow-up artifacts, not part of the tests/schema commits.**

   Evidence:
   - The manifest planner says it was read-only and did not edit manifests, lockfiles, BUILD files, schema fixtures, generated files, staging, or commits: `.codex/workflow/agents/solid_refactor_wave3_manifest_planner_worker.handoff.md:5-8`.
   - Current status includes many untracked `.codex/workflow/agents/*.handoff.md` and prompt files; these are workflow artifacts, not source/test/schema changes.
   - Current status also includes untracked generated/planned `BUILD.bazel` scaffolds, including representative files such as `codex-rs/app/app-catalog-api/BUILD.bazel`, `codex-rs/app/app-catalog-types/BUILD.bazel`, `codex-rs/core-domain/types/BUILD.bazel`, `codex-rs/thread/thread-store/BUILD.bazel`, and `codex-rs/turn/turn-loop-api/BUILD.bazel`.
   - The planner’s deferred verification includes `git status --short --untracked-files=all`, `git diff --check -- codex-rs MODULE.bazel MODULE.bazel.lock`, `just bazel-lock-update`, and `just bazel-lock-check`: `.codex/workflow/agents/solid_refactor_wave3_manifest_planner_worker.handoff.md:352`, `389`, `395-396`.

   Root-owned next action: keep `.codex/workflow/agents/*` handoff/prompt files out of product commits unless root intentionally makes a process-evidence commit. Keep the untracked `BUILD.bazel` scaffolds out of the tests/schema commits; include them only in a manifest/Bazel follow-up after root decides the complete scaffold set and runs the Bazel/lock verification lane.

4. **No source-level stale-test API defect found in the scoped `mcp_turn_metadata` repair, but it still needs the integration test target.**

   Evidence:
   - The stale repair worker only changed `codex-rs/core/tests/suite/mcp_turn_metadata.rs`: `.codex/workflow/agents/solid_refactor_wave4_stale_test_api_repair_worker.handoff.md:11`.
   - Current test file keeps both target tests: `approved_mcp_tool_call_metadata_records_prior_user_input_request` at `codex-rs/core/tests/suite/mcp_turn_metadata.rs:101` and `mcp_tool_call_metadata_records_prior_request_user_input_tool` at `codex-rs/core/tests/suite/mcp_turn_metadata.rs:196`.
   - Both still assert `x-codex-turn-metadata/user_input_requested_during_turn` is `true` at `codex-rs/core/tests/suite/mcp_turn_metadata.rs:186-189` and `312-315`.

   Root-owned next action: after source integration, run the exact `mcp_turn_metadata` release integration target before committing the stale API repair.

## Commit Boundaries

1. **Core test support and stale-test repair commit**

   Include only after fixing/accepting the core dependency boundary:
   - `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
   - `codex-rs/core/tests/common/test_codex.rs`
   - `codex-rs/core/tests/suite/client.rs`
   - `codex-rs/core/tests/suite/mcp_turn_metadata.rs`
   - required manifest/lock changes only if they are part of the accepted boundary decision

   Do not include app-server schema JSON, Bazel BUILD scaffolds, or workflow handoff/prompt files in this commit.

2. **App-server protocol/schema commit**

   Include:
   - `codex-rs/app-server-protocol/src/protocol/v2/permissions.rs`
   - `codex-rs/app-server-protocol/src/protocol/thread_history.rs`
   - `codex-rs/app-server-protocol/src/protocol/v2/tests.rs`
   - all intentional generated tracked and untracked files under `codex-rs/app-server-protocol/schema/json`

   Do not include core test target repairs or manifest/Bazel scaffold fallout in this commit.

3. **Manifest/Bazel follow-up commit**

   Include only after source boundaries are stable:
   - `codex-rs/core-api/Cargo.toml`
   - `codex-rs/core-domain/types/Cargo.toml`
   - `codex-rs/core/Cargo.toml` only if the dependency boundary is intentional
   - `codex-rs/Cargo.lock`
   - intentional `BUILD.bazel` scaffolds and any lockfile updates required by Bazel

   Do not include schema refreshes, core stale-test edits, or workflow notes.

## Generated / Temporary Files To Exclude From Product Commits

- Exclude `.codex/workflow/agents/*.handoff.md`, `.codex/workflow/agents/*.prompt.md`, and worker launcher scripts from source/test/schema commits unless root intentionally makes a workflow-meta commit.
- Exclude local prompt-reducer artifacts under `C:\Users\Oleh\AppData\Local\Temp\codex-prompt-reducer\...`; they are outside the repo but should not be copied into commits.
- Exclude unverified untracked `BUILD.bazel` scaffolds from tests/schema commits until the manifest/Bazel follow-up is verified.
- Do not blindly exclude generated app-server schema JSON. Commit it with the matching DTO/source commit only after rerunning and reviewing the schema generation lane.

## Exact Root-Owned Verification Commands

Core test-support / stale-test slice:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package core_test_support
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Filter prompt_debug_tests
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Filter quota_exceeded
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs "--test","compact_remote_parity"
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs "--test","compact"
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -AllowIntegrationTargets -ExtraCargoArgs "--test" "mcp_turn_metadata"
Push-Location codex-rs; just fmt; just fix -p codex-core; just fix -p core_test_support; Pop-Location
```

App-server protocol/schema slice:

```powershell
Push-Location codex-rs
just write-app-server-schema
just write-app-server-schema --experimental
cargo test -p codex-app-server-protocol --release
Pop-Location
```

Manifest/Bazel slice:

```powershell
Push-Location codex-rs
just bazel-lock-update
just bazel-lock-check
git diff --check -- codex-rs MODULE.bazel MODULE.bazel.lock
git status --short --untracked-files=all
Pop-Location
```

Final root gate before committing any slice:

```powershell
git status --short --untracked-files=all
git diff --check -- codex-rs .codex/workflow
```
