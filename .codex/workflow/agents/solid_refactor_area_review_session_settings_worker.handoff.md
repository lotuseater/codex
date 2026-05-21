# solid_refactor_area_review_session_settings_worker handoff

## Findings

### P1 - `turn/start.runtimeWorkspaceRoots` is still public API but no longer reaches the session settings update path

This looks like a real regression, not a fully propagated intentional removal.

Evidence:
- `codex-rs/app-server-protocol/src/protocol/v2/turn.rs:50` defines `TurnStartParams`; `codex-rs/app-server-protocol/src/protocol/v2/turn.rs:70-73` still exposes experimental `runtime_workspace_roots: Option<Vec<PathBuf>>` for turn start.
- `codex-rs/app-server/src/request_processors/turn_processor.rs:359-370` decides whether the turn has overrides, but the list does not include `params.runtime_workspace_roots`.
- `codex-rs/app-server/src/request_processors/turn_processor.rs:435-450` builds `CodexThreadTurnContextOverrides` without workspace-root fields, and `codex-rs/app-server/src/request_processors/turn_processor.rs:456-475` builds `Op::UserInputWithContext` without them. `rg -n "runtime_workspace_roots" codex-rs/app-server/src/request_processors/turn_processor.rs` returned no matches.
- `codex-rs/core/src/codex_thread.rs:88-91` still keeps `workspace_roots` and `profile_workspace_roots` as public fields on `CodexThreadSettingsOverrides`; `codex-rs/core/src/codex_thread.rs:107` aliases the same type as `CodexThreadTurnContextOverrides`.
- `codex-rs/core/src/codex_thread.rs:292-299` destructures those fields as `_`, so any caller-provided roots are dropped before `SessionSettingsUpdate` is built at `codex-rs/core/src/codex_thread.rs:324-326`.
- `codex-rs/core/src/session/session.rs:361-378` defines `SessionSettingsUpdate` without `workspace_roots` or `profile_workspace_roots`, leaving no downstream slot for a runtime root update.
- `codex-rs/core/src/session/session.rs:179-191` currently snapshots `workspace_roots` as `vec![self.cwd.clone()]` and `profile_workspace_roots` as `Vec::new()`, so the session snapshot cannot preserve a non-cwd runtime root set after updates.

Impact:
Clients can still send per-turn runtime workspace roots, and tests/schema still advertise that behavior, but the turn processor ignores the field and the core settings update path explicitly drops the corresponding override data. Permission-profile materialization for `:workspace_roots` can therefore continue using the old/default roots instead of rebinding to the roots supplied for the current and subsequent turns.

Exact root-owned next action:
Restore the runtime root data flow rather than deleting it silently: resolve `TurnStartParams.runtime_workspace_roots` in the app-server turn path, include it in the override gate, pass it through `CodexThreadTurnContextOverrides` / `Op::UserInputWithContext`, and restore storage/application of `workspace_roots` plus `profile_workspace_roots` in the core session settings/snapshot boundary. If product intent is actually to remove per-turn runtime root updates, then remove the API field, override fields, schema fixtures, TUI callers, and tests in one explicit compatibility decision; current code is halfway between the two states.

## Test Coverage Checked

Runtime-root behavior is still covered by tests, but the most important permission-profile rebind test is Unix-only.

Evidence:
- `codex-rs/app-server/tests/suite/v2/turn_start.rs:2268-2270` defines `turn_start_permission_profile_rebinds_runtime_workspace_roots_between_turns` behind `#[cfg(unix)]`.
- That test configures `:workspace_roots` at `codex-rs/app-server/tests/suite/v2/turn_start.rs:2318`, sends an old root at `codex-rs/app-server/tests/suite/v2/turn_start.rs:2347`, sends a new root at `codex-rs/app-server/tests/suite/v2/turn_start.rs:2370`, then asserts the second permissions instructions contain the new root and not the old root at `codex-rs/app-server/tests/suite/v2/turn_start.rs:2404-2408`.
- Initial thread-start runtime roots are covered by `codex-rs/app-server/tests/suite/v2/thread_start.rs:240`, `:256`, `:268`, and `:274`; profile-root exclusion from runtime roots is covered by `codex-rs/app-server/tests/suite/v2/thread_start.rs:282`, `:287`, `:310`, `:315`, and config fixture `:1095`.
- Loaded-thread runtime roots are covered by `codex-rs/app-server/tests/suite/v2/thread_resume.rs:252`, `:284`, and `:317-318`.

Exact root-owned test action:
After restoring propagation, run the focused app-server v2 runtime-root tests in a release lane. Because the key permission rebind assertion is Unix-only, add or preserve a lower-level non-Unix test around `CodexThread::thread_settings_update` / `SessionSettingsUpdate` so this regression is caught on Windows too, or make sure the Unix integration lane is required for this slice.

## Non-Findings / Scope Notes

- Thread start/resume/fork callers still thread `runtime_workspace_roots` into `ConfigOverrides`: see `codex-rs/app-server/src/request_processors/thread_processor.rs:844-846`, `:1252-1273`, `:2454-2456`, `:2488-2492`, `:2629-2630`, and `:3277-3278`. The gap I found is specifically the turn/session settings update path after a thread already exists.
- I did not edit source files, stage, commit, push, run tests, run cargo/rustc/just/Bazel, regenerate schema, or modify generated artifacts.

## Commands Used

Allowed inspection only: `rg`, `Get-Content`, `git diff`, `git show`, and `git status`.

## Verification Not Run

No tests or builds were run because this worker was explicitly limited to read-only review commands. The cited tests are the exact lanes root should run or repair after applying the fix.
