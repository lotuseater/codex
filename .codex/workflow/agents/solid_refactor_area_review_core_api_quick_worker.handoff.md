# solid_refactor_area_review_core_api_quick_worker handoff

## Findings

1. No concrete import/API regression from the core-api identifier move was found in the current dirty source tree.
   - Current core-api still exposes the protocol-specific escape hatch as `ProtocolThreadId`: `codex-rs/core-api/src/lib.rs:40`.
   - Current core-api exports the moved protocol-neutral root aliases: `codex-rs/core-api/src/lib.rs:62`, `codex-rs/core-api/src/lib.rs:63`, `codex-rs/core-api/src/lib.rs:64`, `codex-rs/core-api/src/lib.rs:65`.
   - Those aliases come from `codex-core-domain-types`: `codex-rs/core-api/src/identifiers.rs:3`, `codex-rs/core-api/src/identifiers.rs:4`, `codex-rs/core-api/src/identifiers.rs:5`, `codex-rs/core-api/src/identifiers.rs:6`.
   - The earlier worker recorded the intended move: `codex-core-api` no longer exposes root `ThreadId` from `codex-protocol` at `.codex/workflow/agents/solid_refactor_wave3_core_api_boundary_worker.handoff.md:17`, and the consumer pass found no direct import fallout at `.codex/workflow/agents/solid_refactor_wave4_core_api_consumer_worker.handoff.md:21`.
   - Fresh source search for stale `codex_core_api::ThreadId`, `codex_core_api::SessionId`, `codex_core_api::TurnId`, `codex_core_api::ToolCallId`, `codex_core_api::identifiers`, `ProtocolThreadId`, and `codex_core_api::protocol::*` consumers returned no matches outside the core-api export itself. Compile verification is still not proven because this worker was command-banned from `cargo`/`just`.

2. `Cargo.lock` is unsafe to commit as-is with the core-api source slice because it mixes at least two ownership groups.
   - The core-api/domain-types lock delta is expected: `codex-rs/core-api/Cargo.toml:21` adds `codex-core-domain-types`, `codex-rs/core-domain/types/Cargo.toml:16` adds `serde`, and `codex-rs/Cargo.lock:2797` through `codex-rs/Cargo.lock:2801` now records `codex-core-domain-types` depending on `serde`.
   - The same `Cargo.lock` also contains unrelated core/thread-store fallout: `codex-rs/core/Cargo.toml:132` adds `codex-thread-store`, and `codex-rs/Cargo.lock:2706` records `codex-thread-store` in the `codex-core` dependency list. That should not ride in the core-api identifier commit unless the core/thread-store slice is intentionally included and verified with it.

3. App-server schema JSON is unsafe to commit with the core-api slice; Bazel lock/build files are not currently dirty in this check.
   - The app-server schema dirt belongs with app-server protocol DTO/source changes, not the core-api identifier move. For example, current dirty app-server protocol source touches permission DTO conversion at `codex-rs/app-server-protocol/src/protocol/v2/permissions.rs:70`, `codex-rs/app-server-protocol/src/protocol/v2/permissions.rs:111`, and approval params around `codex-rs/app-server-protocol/src/protocol/v2/permissions.rs:700`, while schema output includes corresponding generated JSON such as `codex-rs/app-server-protocol/schema/json/v2/ThreadListResponse.json:453` and `codex-rs/app-server-protocol/schema/json/v2/ThreadReadResponse.json:453`.
   - The standing review note also says generated app-server schema files should be committed only with the DTO/source changes that caused them: `.codex/workflow/solid-refactor-review-findings.md:31`.
   - `git diff --name-only -- "*BUILD.bazel" "*MODULE.bazel.lock"` produced no Bazel lock/build paths. Wave 3/4 still list `just bazel-lock-update` and `just bazel-lock-check` as root follow-up verification at `.codex/workflow/agents/solid_refactor_wave3_core_api_boundary_worker.handoff.md:50`, `.codex/workflow/agents/solid_refactor_wave3_core_api_boundary_worker.handoff.md:51`, `.codex/workflow/agents/solid_refactor_wave4_core_api_consumer_worker.handoff.md:54`, and `.codex/workflow/agents/solid_refactor_wave4_core_api_consumer_worker.handoff.md:55`.

## Exact root-owned next action

Before committing the core-api source slice, root should split/reconcile the artifacts so the core-api commit contains only:

- `codex-rs/core-api/Cargo.toml`
- `codex-rs/core-api/src/lib.rs`
- `codex-rs/core-domain/types/Cargo.toml`
- `codex-rs/core-domain/types/src/lib.rs`
- only the `codex-core-domain-types`/`serde` portion of `codex-rs/Cargo.lock`

Root should keep `codex-rs/core/Cargo.toml`, the `codex-thread-store` lock delta, app-server schema JSON, and any future Bazel lock/build refresh out of that commit unless their owning source slices are intentionally grouped. After that split, run the focused release checks already named by wave 3/4: `cargo check --release -p codex-core-api`, `cargo check --release -p codex-thread-manager-sample`, then `just bazel-lock-update` and `just bazel-lock-check` before committing.
