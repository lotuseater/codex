status: blocked-moving-tree

Findings:
- The current source snapshot appears to fix the P1 workspace-root drop: the public override fields are still present and are no longer destructured away, and the session update/snapshot paths now carry both root vectors.
- I am not marking this `ok` because `solid_refactor_fix_session_workspace_roots_worker` has an exec marker/log but no owner handoff file in `.codex/workflow/agents/` yet. Treat this as an in-flight moving tree until that worker lands or root takes ownership.

Evidence:
- `codex-rs/core/src/codex_thread.rs:90-91` keeps `CodexThreadSettingsOverrides.workspace_roots` and `profile_workspace_roots` public.
- `codex-rs/core/src/codex_thread.rs:296-299` destructures `workspace_roots` and `profile_workspace_roots` by name, and `codex-rs/core/src/codex_thread.rs:324-327` passes them into `SessionSettingsUpdate`.
- `codex-rs/core/src/session/session.rs:381-382` adds `workspace_roots` and `profile_workspace_roots` to `SessionSettingsUpdate`.
- `codex-rs/core/src/session/session.rs:280-286` applies explicit root updates into `SessionConfiguration`; `codex-rs/core/src/session/session.rs:191-193` returns the stored roots in `ThreadConfigSnapshot` instead of collapsing to `cwd`/empty profile roots.
- `codex-rs/protocol/src/protocol.rs:363-369` now carries both root fields on `Op::UserInputWithTurnContext`, and `codex-rs/core/src/session/handlers.rs:166-169` / `codex-rs/core/src/session/handlers.rs:201-204` forward them to `SessionSettingsUpdate`.
- `codex-rs/app-server/src/request_processors/turn_processor.rs:459-462` validates `runtime_workspace_roots` through `CodexThreadTurnContextOverrides.workspace_roots`, and `codex-rs/app-server/src/request_processors/turn_processor.rs:488-490` submits them with the turn.
- `codex-rs/app-server/src/request_processors/thread_processor.rs:74-90` compares requested runtime roots against `config_snapshot.workspace_roots`, which now depends on the repaired snapshot preservation.
- `codex-rs/core/src/session/tests.rs:5140` adds `session_settings_update_preserves_workspace_roots_in_snapshot`; `codex-rs/core/src/session/tests.rs:5165-5166` asserts both runtime and profile roots survive into the snapshot.
- Owner state: `.codex/workflow/agents/solid_refactor_fix_session_workspace_roots_worker.exec.marker.txt` exists and points to the visible worker log, but `Get-ChildItem` found no `.codex/workflow/agents/solid_refactor_fix_session_workspace_roots_worker.handoff.md`.

Exact next action for root:
- Wait for or recover `solid_refactor_fix_session_workspace_roots_worker` handoff, then resnapshot the same files. If the source still matches the evidence above, this P1 can be treated as fixed pending verification.

Smallest targeted verification after owner handoff:
- `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Filter session_settings_update_preserves_workspace_roots_in_snapshot`
- `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-app-server -Filter turn_start_permission_profile_rebinds_runtime_workspace_roots_between_turns`

Notes:
- The app-server regression test is currently `#[cfg(unix)]` at `codex-rs/app-server/tests/suite/v2/turn_start.rs:2268-2270`; on this Windows machine root should expect the core snapshot test to be the local non-Unix coverage unless a new non-Unix app-server check is added.
- No tests, formatters, Cargo/Bazel/just commands, staging, commits, or source edits were run by this read-only review worker.
