# test_surface_scout Handoff

Status: complete read-only verification surface scout.

## Scope

- Read the SOLID refactor handoff, verification strategy scout handoff, worker delegation commit protocol, and recent `logs/` release/check artifacts.
- Did not start Cargo, Just, formatters, schema generation, builds, tests, Git staging, or commits.
- Only this handoff was edited.
- Existing `log_scout` helper was still running when this handoff was written; these findings come from direct saved-log inspection.

## Sources Inspected

Workflow sources:

- `.codex/workflow/solid-refactor-handoff.md`
- `.codex/workflow/agents/verification_strategy_scout.handoff.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`

Recent log sources with current signal:

- `logs/codex-core-release-check-20260520-204221.log`
- `logs/thread-store-release-check-20260520-204110.log`
- `logs/test-local-release-codex-tools-desktop_automation_tools_respect_config-20260520-193528.log`
- `logs/test-local-release-codex-core-desktop_automation_tools_follow_desktop_automation_config-20260520-193346.log`
- `logs/test-local-release-codex-core-desktop_automation-20260520-193034.log`
- `logs/core-release-lib-test-20260520-174740.log`
- `logs/test-local-release-codex-core-all-20260520-174743.log`
- `logs/cargo-check-release-codex-app-server-20260520-172334.log`
- `logs/core-boundary-canary-latest.txt`
- `logs/local-codex-build-fastrelease-20260520-132139.log`
- `logs/test-local-release-codex-thread-store-all-20260520-155237.log`
- `logs/thread-store-release-test-20260520-155030.log`
- `logs/test-local-release-codex-context-reduction-all-20260520-123030.log`
- `logs/test-local-release-codex-context-reduction-all-20260520-112855.log`
- `logs/test-local-release-codex-guardian-all-20260520-164025.log`

## Current Observed Blockers By Log

- `logs/codex-core-release-check-20260520-204221.log`: current core compile blocker. `codex-core` fails before tests with `could not compile codex-core (lib) due to 95 previous errors; 5 warnings emitted`. The visible error families are:
  - missing hook runtime exports/imports: `PendingInputHookDisposition`, `run_user_prompt_submit_hooks` in `core/src/session/turn.rs` and `core/src/tasks/mod.rs`.
  - missing or moved permission helper: `codex_protocol::permissions::project_roots_glob_pattern` in `core/src/config/permissions.rs`.
  - missing skill dependency exports: `skills::collect_env_var_dependencies`, `skills::resolve_skill_dependencies_for_turn` from `core/src/lib.rs`.
  - missing plugin list/install symbols from `codex_tools`, including `LIST_AVAILABLE_PLUGINS_TO_INSTALL_TOOL_NAME` and `ListAvailablePluginsToInstallResult`.
  - tool registry/spec-plan drift: missing `ToolHandler`, `RegisteredTool`, `ToolRegistryBuilder`, `build_tool_router`, `spec_plan_types`, and `tools::spec` references.
  - forbidden/missing `codex_app_server_protocol` imports from core files such as `client.rs`, `realtime_conversation.rs`, `session/mod.rs`, and `compact_remote.rs`.
  - thread/session extraction drift: missing `CodexThreadTurnContextOverrides`, `TurnInput`, `SessionConfigured`, `ThreadEvent`, `StateDbHandle`, `LocalThreadStore`, `session.input_queue`, and `Op::UserInput.thread_settings`.
  - `ToolExecutor` migration incomplete across handlers: associated type `Output` missing or `handle` signature no longer matches the trait.
  - exec/unified shell shape drift: `RunExecLikeArgs` fields such as `freeform` and `unified_exec_shell_mode` are no longer present.
  - context fragment trait drift: `ContextualUserFragment` implementations still define removed constants such as `ROLE`, `START_MARKER`, and `END_MARKER`.

- `logs/test-local-release-codex-core-desktop_automation_tools_follow_desktop_automation_config-20260520-193346.log`: focused core DAB lane did not reach the DAB assertion. It fails at the same broad `codex-core` compile surface, ending with `could not compile codex-core (lib) due to 96 previous errors; 5 warnings emitted`.

- `logs/test-local-release-codex-core-desktop_automation-20260520-193034.log` and the older `191913`, `191239`, `185627` core DAB logs: superseded by the newer core check, but they also fail before running the intended tests because `codex-core` does not compile.

- `logs/core-release-lib-test-20260520-174740.log` and `logs/test-local-release-codex-core-all-20260520-174743.log`: same current core compile surface as the later logs, ending with `codex-core` compile failure and no useful test result.

- `logs/core-boundary-canary-latest.txt`: boundary canary still fails. The saved output has 23 lines, including 9 `LocalThreadStore` forbidden-pattern hits, 8 `codex_app_server_protocol` source hits, and a transitive dependency violation on `codex-app-server-protocol` from `codex-core`.

- `logs/cargo-check-release-codex-app-server-20260520-172334.log`: no app-server compile result. The log contains only `Blocking waiting for file lock on build directory`, so this lane is inconclusive and should be rerun only when no repo-local Cargo/rustc/link processes are active.

- `logs/local-codex-build-fastrelease-20260520-132139.log`: latest saved whole FastRelease build is not green. It fails in `codex-extension-api` with `ToolExecutor<ToolCall>` requiring the associated `Output` type. Older FastRelease logs show earlier failures in `codex-tools`, `codex-tui-render`, `codex-config`, `codex-app-server-protocol`, permissions, and feature toggles; those older blockers are partly superseded by later build progress, but no later FastRelease log proves the binary build green.

- `logs/thread-store-release-test-20260520-155030.log`: historical thread-store test failure with `37 passed; 31 failed`, superseded by later green thread-store logs.

- `logs/test-local-release-codex-context-reduction-all-20260520-112855.log`: historical context-reduction failure with `22 passed; 1 failed`, superseded by later green context-reduction logs.

## Green Or Useful Non-Blocking Logs

- `logs/thread-store-release-check-20260520-204110.log`: `codex-thread-store-api` and `codex-thread-store` checked successfully in the release profile.
- `logs/test-local-release-codex-thread-store-all-20260520-155237.log`: `codex-thread-store` release tests passed with `68 passed; 0 failed`.
- `logs/test-local-release-codex-context-reduction-all-20260520-123030.log`: `codex-context-reduction` release tests passed with `23 passed; 0 failed`, plus a 0-test target.
- `logs/test-local-release-codex-tools-desktop_automation_tools_respect_config-20260520-193528.log`: focused `codex-tools` release test passed with `1 passed; 0 failed`.
- `logs/test-local-release-codex-guardian-all-20260520-164025.log`: pattern scan found no compile/test errors; treat as useful green guardian signal, but lower value than logs with explicit test summaries.

## Smallest Recommended Release Verification Lanes

Run these only after the relevant source slice is edited. Commands are recommendations; none were run by this scout.

| Slice | Smallest useful lane | Current note |
| --- | --- | --- |
| Boundary canary | From repo root: `powershell -ExecutionPolicy Bypass -File .codex\prototypes\check-core-boundaries.ps1` | Can run before Cargo. Currently failing and should gate core commit readiness. |
| Thread store extraction | From repo root: `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-thread-store` | Latest check and test logs are green. |
| Thread API skeletons | From `codex-rs`: `cargo check --release -p codex-thread-api -p codex-thread-handle-api -p codex-thread-manager-api -p codex-thread-projection-api` | Useful before core integration; no saved focused log found. |
| Session crates | From `codex-rs`: `cargo check --release -p codex-session-api -p codex-session-events -p codex-session-input -p codex-session-policy -p codex-session-runtime-api -p codex-session-runtime -p codex-session-state -p codex-session-factory` | Defer core-dependent behavior tests until `codex-core` compiles. |
| Turn crates | From `codex-rs`: `cargo check --release -p codex-turn-api -p codex-turn-events -p codex-turn-loop-api -p codex-turn-loop -p codex-turn-policy -p codex-turn-state -p codex-turn-tool-bridge` | Defer core loop behavior until core compile blockers are fixed. |
| Context/domain crates | From `codex-rs`: `cargo check --release -p codex-core-domain-types -p codex-compaction-policy -p codex-context-budget -p codex-history-api -p codex-prompt-context` | Pair with `codex-context-reduction` tests if context behavior changed. |
| Tools-domain crates | From `codex-rs`: `cargo check --release -p codex-tool-execution-api -p codex-tool-handler-api -p codex-tool-registry-api` | Important before fixing `ToolExecutor` and registry drift in `codex-core`. |
| Runtime-domain crates | From `codex-rs`: `cargo check --release -p codex-auth-api -p codex-model-client-api -p codex-runtime-ports -p codex-state-db-api -p codex-telemetry-api` | Useful for the missing `StateDbHandle` and model/client port surface. |
| Context reduction | From repo root: `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-context-reduction` | Latest focused release tests are green. |
| Guardian | From repo root: `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-guardian` | Existing log is green enough for a scout signal. |
| `codex-tools` DAB config | From repo root: `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-tools -Filter desktop_automation_tools_respect_config` | Latest focused release test is green. |
| `codex-extension-api` ToolExecutor migration | From repo root: `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-extension-api` | Latest FastRelease fails here; verify before another whole binary build. |
| `codex-core` spec-plan/DAB integration | From repo root: `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter tools::spec_plan_tests -CleanCoreLibTestArtifactsOnSuccess`; then `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter desktop_automation_tools_follow_desktop_automation_config -AllowBroadCoreLibUnitTests` | Must wait until `codex-core` compiles. |
| App server | First rerun a release check only when build locks are clear. If app-server protocol shapes changed, use `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server-protocol`; otherwise from `codex-rs`, `cargo check --release -p codex-app-server`. | Current app-server check log is lock-only and inconclusive. |
| Whole local binary | From repo root: `powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode FastRelease`; use `-Mode LowMemRelease` only under pressure. | Defer until targeted extension-api/core blockers are fixed. Save and inspect the combined log. |

## Lanes To Defer Until Compile Blockers Are Fixed

- Any `codex-core` focused or broad release test, including DAB, spec-plan, core lib, and core all lanes, until the current 95-error core compile surface is resolved.
- Whole FastRelease binary build until `codex-extension-api` `ToolExecutor` output migration and `codex-core` compile blockers are fixed or intentionally narrowed.
- App-server release check until the Cargo build-directory lock issue is absent; if protocol/core boundary work is still dirty, run protocol-focused checks before app-server.
- Workspace-wide `cargo test --release`, broad `just test`, Bazel lock/check, and schema generation until the smaller release lanes are green and the relevant source/API changes actually require them.
- `just fmt` and `just fix -p <crate>` belong after Rust edits, not during this read-only scout. For a large `codex-core` slice, `just fix -p codex-core` remains the right pre-finalization lint lane after compile blockers are repaired.

## Commit Readiness Notes

- No commit should be made for the broad SOLID refactor state while `codex-core` fails to compile and the boundary canary fails.
- Thread-store has the strongest commit-readiness signal from saved logs: later release check/test logs are green. Actual commit scope still depends on root isolating the relevant files and handling any root-owned workspace manifest or lockfile changes explicitly.
- Context-reduction and `codex-tools` focused DAB config have useful green focused logs, but they should not be bundled with unresolved `codex-core` integration fixes unless the file ownership and verification story is coherent.
- `codex-core` DAB/spec-plan changes are not commit-ready: the focused tests do not run because compile fails first.
- The latest whole FastRelease log is still red at `codex-extension-api`, so no binary-build readiness claim is available.
- I did not run Git status, stage files, create commits, or push.
