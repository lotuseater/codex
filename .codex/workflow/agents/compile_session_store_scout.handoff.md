# compile_session_store_scout Handoff

Status: completed read-only scout on 2026-05-20.

No source files were edited. No Cargo, Just, formatter, git staging, or commit
commands were run.

## Scope

Current compile blockers around session input, thread settings, and concrete
thread-store types after the SOLID/thread-store refactor.

Sources inspected:

- `.codex/workflow/agents/thread_store_boundary.handoff.md`
- `.codex/workflow/agents/canary_observer.handoff.md`
- `.codex/workflow/solid-refactor-handoff.md`
- targeted `rg` searches for `input_queue`, `thread_settings`,
  `Op::UserInput`, `LocalThreadStore`, `LocalThreadStoreConfig`,
  `thread_store_from_config`, and `InMemoryThreadStore`
- focused reads in `codex-rs/core`, `codex-rs/protocol`,
  `codex-rs/thread/thread-store-api`, and `codex-rs/thread/thread-store`

## Exact Blockers

### 1. `Session` no longer owns `input_queue`

Current source of truth:

- `codex-rs/core/src/session/session.rs:16-39` defines `Session`; it has
  `mailbox`, `mailbox_rx`, and `idle_pending_input`, but no `input_queue`.
- `codex-rs/core/src/session/session.rs:952-967` constructs those fields with
  `Mailbox::new()` and `Mutex::new(Vec::new())`.
- `codex-rs/core/src/session/mod.rs:3244-3257` exposes direct session mailbox
  helpers: `subscribe_mailbox_seq`, `enqueue_mailbox_communication`,
  `has_trigger_turn_mailbox_items`, and `has_pending_mailbox_items`.
- `codex-rs/core/src/session/mod.rs:3273-3294` owns `get_pending_input`.
- `codex-rs/core/src/session/mod.rs:3316-3331` owns queued response-item helpers.
- `codex-rs/core/src/session/mod.rs:3331-3345` owns `has_pending_input`.
- `codex-rs/core/src/agent/mailbox.rs:11-17` is the live mailbox type;
  `MailboxReceiver` exposes `has_pending`, `has_pending_trigger_turn`, and
  `drain` at `51-72`.

Stale callsites still compile against an `input_queue` field/object:

- `codex-rs/core/src/codex_thread.rs:399`
- `codex-rs/core/src/tasks/regular.rs:83`
- `codex-rs/core/src/stream_events_utils.rs:161`, `355`
- `codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs:62`
- `codex-rs/core/src/goals.rs:1348`, `1396`, `1403`, `1441`, `1444`
- tests: `codex-rs/core/src/agent/control_tests.rs:492` and
  `codex-rs/core/src/tools/handlers/multi_agents_tests.rs:2937`, `3425`,
  `3498`, `3597`, `3685`

Likely fixes:

- Replace `sess.input_queue.has_pending_input(&sess.active_turn).await` with
  `sess.has_pending_input().await`.
- Replace `session.input_queue.subscribe_mailbox().await` with
  `session.subscribe_mailbox_seq()` and update the local wait helper to use
  `watch::Receiver<u64>` instead of `watch::Receiver<()>`.
- Replace `sess.input_queue.queue_response_items_for_next_turn(items).await`
  with `sess.queue_response_items_for_next_turn(items).await`.
- Replace mailbox delivery/defer paths with the direct `Session` methods already
  in `session/mod.rs`, adding narrowly scoped methods there only if an old
  `InputQueue` method has no direct equivalent.

### 2. `Op::UserInput.thread_settings` is stale protocol shape

Current source of truth:

- `codex-rs/protocol/src/protocol.rs:327-338` defines `Op::UserInput` with only
  `items`, `environments`, `final_output_json_schema`, and
  `responsesapi_client_metadata`.
- `codex-rs/protocol/src/protocol.rs:344-409` defines
  `Op::UserInputWithTurnContext` for ordered input plus turn-context overrides.
- `codex-rs/protocol/src/protocol.rs:510-558` defines `Op::OverrideTurnContext`
  for persistent context updates without input.
- `codex-rs/core/src/session/handlers.rs:166-233` handles
  `UserInputWithTurnContext` and `UserInput` separately.
- `codex-rs/app-server/src/request_processors/turn_processor.rs:455-483`
  already uses `UserInputWithTurnContext` when overrides exist and plain
  `UserInput` otherwise.

Stale compile blockers:

- Many `Op::UserInput` construction sites still include
  `thread_settings: Default::default()`, which is no longer a field. Examples:
  - `codex-rs/core/src/codex_delegate.rs:190-196`
  - `codex-rs/memories/write/src/runtime.rs:259-265`
  - `codex-rs/mcp-server/src/codex_tool_runner.rs:119`, `169`
  - `codex-rs/thread-manager-sample/src/main.rs:313`
  - core suite tests across `abort_tasks.rs`, `client_websockets.rs`,
    `compact_remote_parity.rs`, `fork_thread.rs`, `hooks.rs`,
    `plugins.rs`, `quota_exceeded.rs`, `realtime_conversation.rs`,
    `request_compression.rs`, `search_tool.rs`, `stream_*`,
    `user_notification.rs`, and `window_headers.rs`
- `codex-rs/core/tests/common/lib.rs:253` references missing
  `codex_protocol::protocol::ThreadSettingsOverrides`.
- `codex-rs/core/tests/common/lib.rs:260` submits missing
  `Op::ThreadSettings`.
- `codex-rs/core/tests/suite/mcp_turn_metadata.rs:79` also references
  `ThreadSettingsOverrides`.

Likely fixes:

- For default/no-op cases, remove the `thread_settings: Default::default()`
  field.
- For actual override tests, migrate helpers to submit `Op::OverrideTurnContext`
  or `Op::UserInputWithTurnContext`, depending on whether the test needs
  settings-only behavior or settings ordered with user input.
- Do not resurrect `Op::ThreadSettings` or add `thread_settings` back to
  `Op::UserInput`; the live protocol source already models this through the
  context variants.

### 3. `codex-core` still leaks concrete thread-store types

Current source of truth:

- `codex-rs/core/Cargo.toml:74` depends on `codex-thread-store-api`, not
  `codex-thread-store`.
- `codex-rs/thread/thread-store/src/lib.rs:13-20` exports the concrete
  `ThreadStoreSelection`, `thread_store_from_config`, `InMemoryThreadStore`,
  `LiveThread`, `StoreLiveThreadFactory`, `LocalThreadStore`, and
  `LocalThreadStoreConfig`.
- `codex-rs/thread/thread-store/src/factory.rs:18-29` is the concrete factory;
  it now requires `(config, ThreadStoreSelection, state_db)`.
- `codex-rs/core/src/config/mod.rs:400-405` is the config source for local vs
  in-memory selection.
- App/integration crates already map config to selection at
  `codex-rs/app-server/src/message_processor.rs:278-283` and call the concrete
  factory at `312-318`.

Production blockers:

- `codex-rs/core/src/thread_manager.rs:351-357` constructs
  `LocalThreadStore`/`LocalThreadStoreConfig`, but core has only the API crate.
  This should not be fixed by adding a core dependency on `codex-thread-store`.
- `codex-rs/core/src/prompt_debug.rs:22`, `42` imports/calls
  `crate::thread_manager::thread_store_from_config`, but no current core-owned
  factory exists and the concrete factory belongs outside core.

Test and helper blockers:

- `codex-rs/core/tests/common/test_codex.rs:22`, `430` and
  `codex-rs/core/tests/suite/client.rs:9`, `1129` import/call
  `codex_core::thread_store_from_config`.
- `codex-rs/core/src/thread_manager_tests.rs:501`, `611`, `658`, `715`, `771`,
  `857`, `972`, `1190`, `1298`, `1395`, `1538` call
  `thread_store_from_config`.
- `codex-rs/core/src/tools/handlers/multi_agents_tests.rs:9`, `3878` call
  `thread_store_from_config`.
- `codex-rs/core/src/session/tests.rs:3736-3737`, `3888-3889`, `4163-4164`,
  `4267-4268`, `5748-5749` construct `LocalThreadStore`.
- `codex-rs/core/src/session/tests/guardian_tests.rs:736-737` constructs
  `LocalThreadStore`.
- `codex-rs/core/src/agent/control_tests.rs:1973-1975` constructs
  `LocalThreadStore`.
- `codex-rs/core/src/session/tests.rs:5305`, `5338` uses
  `InMemoryThreadStore` / `InMemoryThreadStoreCalls`.
- `codex-rs/core/src/thread_manager_tests.rs:860` downcasts to
  `InMemoryThreadStore`.
- `codex-rs/core/src/session/tests.rs:2945`, `5308` calls concrete
  `LiveThread::create`.

Likely fixes:

- Keep the concrete factory in application/integration crates.
- In core production/test code, use `Arc<dyn ThreadStore>` and
  `Arc<dyn LiveThreadFactory>` inputs, `UnsupportedThreadStore`, or small
  API-only fakes, not `codex_thread_store` concrete types.
- If a test genuinely validates concrete local/in-memory persistence behavior,
  move it to `codex-rs/thread/thread-store` or an application crate that owns
  the concrete dependency.

### 4. Dynamic-tools read no longer needs a downcast

Current source of truth:

- `codex-rs/thread/thread-store-api/src/store.rs:70-76` already has
  `ThreadStore::read_thread_dynamic_tools`, defaulting to `Ok(None)`.
- `codex-rs/thread/thread-store-api/src/types.rs:147-152` exports
  `ReadThreadDynamicToolsParams`.
- `codex-rs/thread/thread-store/src/local/mod.rs:249-259` implements dynamic
  tool reads through the local state DB.
- `codex-rs/thread/thread-store/src/in_memory.rs:238-254` implements dynamic
  tool reads from latest metadata updates or created-thread params.
- `codex-rs/thread/thread-store/src/in_memory.rs:99-110` includes
  `read_thread_dynamic_tools` in call counts.

Stale core blocker:

- `codex-rs/core/src/session/mod.rs:580-583` still downcasts
  `Arc<dyn ThreadStore>` to `LocalThreadStore` to recover `state_db()` before
  calling `state_db::get_dynamic_tools`.

Likely fix:

- In `codex-rs/core/src/session/mod.rs:570-589`, preserve the `config.ephemeral`
  guard, but replace the concrete downcast/state-db call with
  `thread_store.read_thread_dynamic_tools(ReadThreadDynamicToolsParams { thread_id }).await`.
- Keep fallback behavior equivalent: no thread id or unsupported store should
  resolve to no persisted dynamic tools rather than introducing a hard failure.

## Recommended Fix Order

1. Fix the production dynamic-tools downcast in `session/mod.rs` using the
   already-existing `ThreadStore::read_thread_dynamic_tools` API. This removes
   the most important production concrete store leak with minimal blast radius.
2. Repair production `input_queue` callsites by routing them to the existing
   direct `Session` methods. Do this before tests so the intended runtime API is
   clear.
3. Remove stale `thread_settings` fields from no-op `Op::UserInput` callsites,
   then migrate the real settings helpers/tests to `OverrideTurnContext` or
   `UserInputWithTurnContext`.
4. Remove core-owned concrete store construction/factory callsites. Replace
   production helpers with injected API objects or `UnsupportedThreadStore` only
   where unsupported persistence is actually valid; replace tests with API-only
   fakes or move concrete persistence tests to concrete-owner crates.
5. Run the boundary canary, then the focused release test lane for the touched
   crate(s). Root should own broad/final verification.

## Files Likely Touched

Production/root-sensitive:

- `codex-rs/core/src/session/mod.rs`
- `codex-rs/core/src/session/session.rs` only if a missing direct session helper
  must be added near the current fields
- `codex-rs/core/src/codex_thread.rs`
- `codex-rs/core/src/tasks/regular.rs`
- `codex-rs/core/src/stream_events_utils.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs`
- `codex-rs/core/src/goals.rs`
- `codex-rs/core/src/thread_manager.rs`
- `codex-rs/core/src/prompt_debug.rs`
- `codex-rs/core/src/codex_delegate.rs`
- `codex-rs/memories/write/src/runtime.rs`
- `codex-rs/mcp-server/src/codex_tool_runner.rs`
- `codex-rs/thread-manager-sample/src/main.rs`

Tests/helpers:

- `codex-rs/core/tests/common/lib.rs`
- `codex-rs/core/tests/common/test_codex.rs`
- `codex-rs/core/tests/suite/client.rs`
- `codex-rs/core/tests/suite/mcp_turn_metadata.rs`
- `codex-rs/core/src/thread_manager_tests.rs`
- `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
- `codex-rs/core/src/session/tests.rs`
- `codex-rs/core/src/session/tests/guardian_tests.rs`
- `codex-rs/core/src/agent/control_tests.rs`
- additional core suite files with only stale `thread_settings: Default::default()`
  fields, listed by `rg -l "thread_settings: Default::default\\(\\)"`

Manifest/Bazel/lockfiles:

- Root should decide whether any manifest changes are required. The preferred
  direction is to avoid adding `codex-thread-store` to `codex-core`; if any
  dependency or Bazel metadata changes become necessary, keep them root-owned.

## Ownership

Root-owned:

- Core/application ownership boundaries and any manifest/Bazel/lockfile edits.
- `codex-rs/core/src/session/mod.rs` dynamic-tools initialization and new
  session helper API decisions.
- `codex-rs/core/src/thread_manager.rs` constructor/API shape.
- Final formatting, `just fix -p ...`, boundary canary, and release-profile test
  selection.

Delegate-safe:

- Mechanical removal of `thread_settings: Default::default()` from leaf
  `Op::UserInput` constructors after root confirms the protocol migration rule.
- Converting leaf `input_queue` tests to direct `Session` helper methods after
  production helpers are settled.
- Building API-only test fakes for `ThreadStore` / `LiveThreadFactory` in core
  tests, if root defines the fake location and required behavior.
- Moving concrete persistence assertions into `codex-rs/thread/thread-store`
  tests, if root chooses that route.

## Split Recommendation

Yes, split implementation into smaller workers after root lands or explicitly
assigns the central API shape.

Suggested split:

1. Root or high-trust worker: production session/input and dynamic-tools cleanup
   in `core/src/session*`, `codex_thread.rs`, `tasks`, `stream_events_utils`,
   `wait.rs`, and `goals.rs`.
2. Delegate worker: stale `Op::UserInput.thread_settings` cleanup in leaf
   binaries/tests, with special handling for `mcp_turn_metadata` and
   `core/tests/common/lib.rs`.
3. Delegate worker: core test-store migration to API-only fakes or relocation
   of concrete persistence tests, avoiding manifest changes unless root approves.

Do not run these workers concurrently on the same central files
(`session/mod.rs`, `thread_manager.rs`, `core/tests/common/*`) without assigning
file ownership first.
