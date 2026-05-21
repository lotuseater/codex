slice | status | include | exclude | verification still needed
--- | --- | --- | --- | ---
app-server permission profile/schema | blocked-by-mixed-diff | No safe path-level add list. Permission-owned tracked source/doc files are `codex-rs/app-server-protocol/src/protocol/v2/permissions.rs` and `codex-rs/app-server/README.md`; generated tracked files with permission hunks include `codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.schemas.json`, `codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.v2.schemas.json`, `codex-rs/app-server-protocol/schema/json/v2/ThreadForkResponse.json`, `codex-rs/app-server-protocol/schema/json/v2/ThreadResumeResponse.json`, `codex-rs/app-server-protocol/schema/json/v2/ThreadStartResponse.json`, `codex-rs/app-server-protocol/schema/json/v2/TurnStartParams.json`, `codex-rs/app-server-protocol/schema/typescript/v2/ActivePermissionProfile.ts`, `codex-rs/app-server-protocol/schema/typescript/v2/PermissionProfileModificationParams.ts`, `codex-rs/app-server-protocol/schema/typescript/v2/PermissionProfileSelectionParams.ts`, and `codex-rs/app-server-protocol/schema/typescript/v2/index.ts`. | Exclude `codex-rs/app-server-protocol/src/protocol/thread_history.rs` and `codex-rs/app-server-protocol/src/protocol/v2/tests.rs`; they are not permission-profile behavior. Do not path-add `codex-rs/app-server-protocol/schema/json` or `codex-rs/app-server-protocol/schema/typescript` because those directories contain unrelated tracked and untracked generated API changes. | Isolate permission schema hunks by patch-staging or regenerate/schema-audit after other API source slices are separated; then run the app-server protocol schema/test lane.
Bazel BUILD scaffolds for split crates | commit-ready | 37 new `BUILD.bazel` scaffold files listed below. | Exclude `codex-rs/Cargo.lock`, `codex-rs/Cargo.toml`, `MODULE.bazel`, `MODULE.bazel.lock`, Rust source files, and `.codex/workflow/**` artifacts. | None blocking this scaffold commit. Prior workers reported static scaffold audit and Bazel lock check success.

## App-Server Evidence

- Current tracked source/doc diff for the permission area is not self-contained by path. `git diff --stat -- codex-rs/app-server-protocol/src/protocol/thread_history.rs codex-rs/app-server-protocol/src/protocol/v2/permissions.rs codex-rs/app-server-protocol/src/protocol/v2/tests.rs codex-rs/app-server/README.md` reports 4 dirty files.
- `codex-rs/app-server-protocol/src/protocol/v2/permissions.rs` adds `ActivePermissionProfileModification`, carries `modifications` in `ActivePermissionProfile`, and maps `AdditionalWritableRoot` both ways. `codex-rs/app-server/README.md` documents `activePermissionProfile.modifications`, command `permissionProfile`, and permission entry access values.
- `app_server_protocol_review_worker.handoff.md` reports no app-server protocol review findings for that DTO/conversion/doc behavior, but it was read-only and did not run schema generation or tests.
- `codex-rs/app-server-protocol/src/protocol/thread_history.rs` only updates a `UserMessageEvent` fixture with `image_details` and `local_image_details`; that belongs to an image/user-message source slice, not the permission-profile commit.
- `codex-rs/app-server-protocol/src/protocol/v2/tests.rs` removes config-related fields such as `model_auto_compact_token_limit_scope`, `desktop`, and `computer_use`; that belongs to config/protocol drift, not the permission-profile commit.
- The schema output is mixed. Permission hunks are present, but aggregate files such as `codex-rs/app-server-protocol/schema/typescript/v2/index.ts` also add unrelated exports for other API areas. `git status --porcelain --untracked-files=all -- codex-rs/app-server-protocol/schema/typescript` also shows unrelated deletes/modifications and untracked process, realtime, and thread-turn generated files.

Exact next action for root: do not commit the app-server slice with a directory-level `git add`. Either patch-stage only the permission hunks, or first separate/regenerate the other app-server API schema changes and then re-audit this slice.

## Bazel Evidence

- `bazel_build_scaffold_worker.handoff.md` reports a completed source-only Bazel scaffold slice and no Cargo/Rust/Bazel/test/schema/format commands in that worker.
- `build_scaffold_audit_worker.handoff.md` reports `COUNT=37`, `MISMATCHES=0`, `ANOMALIES=0`; each `BUILD.bazel` was inspected with its adjacent `Cargo.toml` and matched the minimal `codex_rust_crate` wrapper shape.
- Spot checks of `codex-rs/thread/thread-store-api/BUILD.bazel`, `codex-rs/debug-client/BUILD.bazel`, and `codex-rs/app/app-catalog-api/BUILD.bazel` show only `load("//:defs.bzl", "codex_rust_crate")` plus `name` and `crate_name`.
- `bazel_lock_refresh_worker.handoff.md` reports `just bazel-lock-update` exit 0, no `MODULE.bazel.lock`/`MODULE.bazel` diff, and `just bazel-lock-check` exit 0.
- Current focused status shows the BUILD scaffold slice is independent of dirty lock/source work: the BUILD files are new untracked scaffold files, while the existing dirty `codex-rs/Cargo.lock` belongs outside this commit.

Safe add list for the Bazel scaffold commit:

```powershell
git add -- `
  codex-rs/app/app-catalog-api/BUILD.bazel `
  codex-rs/app/app-catalog-types/BUILD.bazel `
  codex-rs/context-domain/compaction-policy/BUILD.bazel `
  codex-rs/context-domain/context-budget/BUILD.bazel `
  codex-rs/context-domain/history-api/BUILD.bazel `
  codex-rs/context-domain/prompt-context/BUILD.bazel `
  codex-rs/core-domain/types/BUILD.bazel `
  codex-rs/debug-client/BUILD.bazel `
  codex-rs/mcp/elicitation-api/BUILD.bazel `
  codex-rs/runtime-domain/auth-api/BUILD.bazel `
  codex-rs/runtime-domain/model-client-api/BUILD.bazel `
  codex-rs/runtime-domain/runtime-ports/BUILD.bazel `
  codex-rs/runtime-domain/state-db-api/BUILD.bazel `
  codex-rs/runtime-domain/telemetry-api/BUILD.bazel `
  codex-rs/session/session-api/BUILD.bazel `
  codex-rs/session/session-events/BUILD.bazel `
  codex-rs/session/session-factory/BUILD.bazel `
  codex-rs/session/session-input/BUILD.bazel `
  codex-rs/session/session-policy/BUILD.bazel `
  codex-rs/session/session-runtime-api/BUILD.bazel `
  codex-rs/session/session-runtime/BUILD.bazel `
  codex-rs/session/session-state/BUILD.bazel `
  codex-rs/thread/thread-api/BUILD.bazel `
  codex-rs/thread/thread-handle-api/BUILD.bazel `
  codex-rs/thread/thread-manager-api/BUILD.bazel `
  codex-rs/thread/thread-projection-api/BUILD.bazel `
  codex-rs/thread/thread-store-api/BUILD.bazel `
  codex-rs/tools-domain/tool-execution-api/BUILD.bazel `
  codex-rs/tools-domain/tool-handler-api/BUILD.bazel `
  codex-rs/tools-domain/tool-registry-api/BUILD.bazel `
  codex-rs/turn/turn-api/BUILD.bazel `
  codex-rs/turn/turn-events/BUILD.bazel `
  codex-rs/turn/turn-loop-api/BUILD.bazel `
  codex-rs/turn/turn-loop/BUILD.bazel `
  codex-rs/turn/turn-policy/BUILD.bazel `
  codex-rs/turn/turn-state/BUILD.bazel `
  codex-rs/turn/turn-tool-bridge/BUILD.bazel
```

Exact next action for root: commit the Bazel scaffold slice with the add list above, then leave app-server schema/product source separation for a separate integration pass.
