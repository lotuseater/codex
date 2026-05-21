# orchestration_artifacts_review_worker

## Findings

### P2 - App-server resume prompt points workers at a non-existent v2 protocol file

Evidence:
- `.codex/workflow/agents/app_server_boundary_resume_worker.prompt.md:31-33` lists `codex-rs/app-server-protocol/src/protocol/v2.rs`.
- `Test-Path codex-rs/app-server-protocol/src/protocol/v2.rs` returned `False`.
- `codex-rs/app-server-protocol/src/protocol/v2/` exists and contains split v2 modules including `mod.rs`, `thread.rs`, `config.rs`, and other API files.

Impact:
- The resume worker can waste time on a missing first-read path or miss the actual split v2 API modules it is supposed to preserve while refactoring the app-server boundary.

Exact patch suggestion:

```diff
diff --git a/.codex/workflow/agents/app_server_boundary_resume_worker.prompt.md b/.codex/workflow/agents/app_server_boundary_resume_worker.prompt.md
@@
 - `codex-rs/app-server/src/lib.rs`
 - `codex-rs/app-server/src/request_processors.rs`
-- `codex-rs/app-server-protocol/src/protocol/v2.rs`
+- `codex-rs/app-server-protocol/src/protocol/v2/mod.rs`
+- relevant `codex-rs/app-server-protocol/src/protocol/v2/*.rs` modules for changed symbols
```

### P2 - Verification planner prompt points at a non-existent Codex Rust justfile

Evidence:
- `.codex/workflow/agents/verification_matrix_planner_worker.prompt.md:19-22` lists `codex-rs/justfile`.
- `Test-Path codex-rs/justfile` returned `False`.
- The repo root `justfile` exists.

Impact:
- The verification planner may fail a required read or plan commands from the wrong location, weakening the deferred verification matrix for the release-only local policy.

Exact patch suggestion:

```diff
diff --git a/.codex/workflow/agents/verification_matrix_planner_worker.prompt.md b/.codex/workflow/agents/verification_matrix_planner_worker.prompt.md
@@
 - `.cargo/config.toml`
 - `scripts/build-local-codex.ps1`
 - `scripts/test-local-codex-release.ps1`
-- `codex-rs/justfile`
+- `justfile`
```

## Notes

- No builds, tests, schema generation, lockfile generation, or broad repo scans were run by constraint.
- I did not find a high-confidence missing hard-stop rule in the scoped prompts inspected. The no-build/no-test/no-commit/no-subagent restrictions are explicit in the active worker prompts.
