# solid_refactor_area_review_retry_session_settings_worker

## Findings

1. Blocker: session/thread settings updates silently discard real runtime workspace-root data.

   Evidence:
   - `codex-rs/core/src/codex_thread.rs:88-91` still exposes `CodexThreadSettingsOverrides.workspace_roots` and `profile_workspace_roots` as public override inputs.
   - `codex-rs/core/src/codex_thread.rs:295-299` destructures those inputs as `workspace_roots: _` and `profile_workspace_roots: _`, so they are not propagated into the session update.
   - `codex-rs/core/src/session/session.rs:361-380` defines `SessionSettingsUpdate` without any workspace-root/profile-workspace-root fields, so there is no replacement path after the adapter drops them.
   - `codex-rs/core/src/session/session.rs:177-191` builds `ThreadConfigSnapshot` from the active session by forcing `workspace_roots = vec![self.cwd.clone()]` and `profile_workspace_roots = Vec::new()`, which loses non-cwd roots and all profile workspace roots.
   - The API surface still treats this data as real: `codex-rs/app-server-protocol/src/protocol/v2/turn.rs:73` and `codex-rs/app-server-protocol/src/protocol/v2/thread.rs:112` accept `runtime_workspace_roots`, and `codex-rs/app-server-protocol/src/protocol/v2/thread.rs:203`, `:332`, and `:449` return runtime workspace roots in thread responses.
   - App-server still maps incoming runtime roots into the core override path at `codex-rs/app-server/src/request_processors/thread_processor.rs:1272` and returns `config_snapshot.workspace_roots` at `codex-rs/app-server/src/request_processors/thread_processor.rs:1217`, `:2630`, and `:3278`. With the current core drop, those responses collapse to `cwd` rather than the caller-supplied runtime roots.
   - Existing app-server coverage expects this behavior to keep working: `codex-rs/app-server/tests/suite/v2/turn_start.rs:2270` defines `turn_start_permission_profile_rebinds_runtime_workspace_roots_between_turns`, with assertions at `:2405` and `:2407` that the second turn is rebound to the updated runtime root and no longer includes the old one.

   Impact: v2 clients can still send runtime workspace roots, but the settings update path no longer carries them. A turn/thread update that should rebind `:workspace_roots` instead snapshots only the session `cwd` and drops profile roots entirely, which is a user-visible permissions/config regression and a commit blocker for this slice.

   Root-owned next action: restore a real data path for both `workspace_roots` and `profile_workspace_roots` through `CodexThreadSettingsOverrides -> SessionSettingsUpdate -> SessionConfiguration::apply -> ThreadConfigSnapshot`, or deliberately remove/reject the API fields with protocol/schema/docs/test updates. If preserving the existing API behavior, rerun the focused release test for `turn_start_permission_profile_rebinds_runtime_workspace_roots_between_turns` and the related v2 thread start/resume workspace-root tests after the fix.

## Verification

Read-only review only. I did not run tests or edit source because this worker prompt forbids cargo/just/scripts and permits only this handoff write.
