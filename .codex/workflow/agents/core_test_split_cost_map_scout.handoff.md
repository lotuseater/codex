# core_test_split_cost_map_scout Handoff

Date: 2026-05-20
Status: complete

## Scope

Read-only scout for estimating `codex-core` integration-test split cost. This
memo only writes the requested handoff file; no Rust code, Cargo, Just,
formatters, staging, commits, or build lanes were run.

## Sources Inspected

- `.codex/workflow/solid-refactor-handoff.md`
- `codex-rs/core/tests/all.rs`
- `codex-rs/core/tests/suite_bootstrap.rs`
- `codex-rs/core/tests/suite/mod.rs`
- `codex-rs/core/tests/suite/*.rs` inventory and lightweight regex metrics

Harness facts:

- `codex-rs/core/tests/all.rs` is the single current integration test binary
  and declares `mod suite_bootstrap;` before `mod suite;`.
- `suite_bootstrap.rs` owns test-binary dispatch alias setup for
  `apply_patch`, `codex-fs-helper`, and Linux sandbox dispatch. Keep that
  bootstrap shared while splitting lanes.
- `suite/mod.rs` aggregates 84 suite modules. Platform-gated modules are:
  `abort_tasks`, `approvals`, `hooks`, `hooks_mcp`, `request_permissions`,
  and `request_permissions_tool` on non-Windows; `windows_sandbox` on Windows.

## Metric Method

Counts are approximate static metrics from the current files:

- `Tests~` = direct `#[test]`/`#[tokio::test]` attributes plus `#[test_case]`
  instances, so parameterized tests are visible.
- `Tokio/runtime` = `#[tokio::test]` count / async-runtime keyword hits.
- `Mock/net`, `snap/golden`, and `process` are regex hit counts over common
  helper names and concepts. They are cost signals, not exact behavior counts.
- `Cost` is a synthetic compile/iteration band from file size, test count,
  tokio/runtime, mock/network, snapshot/golden, and process/shell signals:
  `VH`, `H`, `M`, `L`.

Inventory totals: 84 modules, about 69,803 source lines, about 786 effective
test cases by the metric above.

## Ranked Module Map

| Rank | Module | Lines | Tests~ | Tokio/runtime | Mock/net | Snap/golden | Process | Cost | Score |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: |
| 1 | `apply_patch_cli` | 1827 | 82 | 35/37 | 145 | 0 | 238 | VH | 763.3 |
| 2 | `unified_exec` | 3221 | 31 | 31/40 | 291 | 0 | 447 | VH | 735.6 |
| 3 | `hooks` | 3800 | 37 | 37/50 | 411 | 5 | 212 | VH | 695.9 |
| 4 | `compact_remote` | 3368 | 29 | 29/31 | 549 | 68 | 27 | VH | 687.8 |
| 5 | `code_mode` | 2921 | 39 | 39/45 | 474 | 0 | 140 | VH | 668.7 |
| 6 | `realtime_conversation` | 3699 | 38 | 38/68 | 442 | 13 | 7 | VH | 602.5 |
| 7 | `client` | 3143 | 36 | 36/36 | 329 | 21 | 38 | VH | 541.7 |
| 8 | `compact` | 3768 | 25 | 25/31 | 433 | 34 | 26 | VH | 537.1 |
| 9 | `client_websockets` | 2138 | 36 | 36/45 | 402 | 7 | 20 | VH | 533.9 |
| 10 | `approvals` | 3281 | 15 | 10/19 | 169 | 0 | 427 | VH | 524.5 |
| 11 | `shell_snapshot` | 736 | 8 | 8/15 | 78 | 121 | 105 | H | 428.4 |
| 12 | `rmcp_client` | 2503 | 15 | 14/30 | 394 | 4 | 91 | H | 417.7 |
| 13 | `request_permissions` | 1908 | 14 | 14/14 | 145 | 0 | 238 | H | 366.0 |
| 14 | `otel` | 1610 | 24 | 22/22 | 184 | 0 | 53 | H | 324.6 |
| 15 | `view_image` | 1522 | 16 | 16/29 | 177 | 0 | 27 | H | 254.9 |
| 16 | `remote_models` | 1314 | 16 | 16/22 | 105 | 0 | 59 | H | 234.3 |
| 17 | `search_tool` | 1529 | 14 | 14/14 | 191 | 0 | 17 | H | 230.5 |
| 18 | `subagent_notifications` | 896 | 8 | 8/16 | 123 | 10 | 62 | H | 201.1 |
| 19 | `items` | 1183 | 14 | 14/14 | 137 | 0 | 2 | M | 189.5 |
| 20 | `truncation` | 818 | 10 | 10/10 | 131 | 0 | 51 | M | 186.4 |
| 21 | `remote_env` | 943 | 9 | 9/9 | 61 | 0 | 108 | M | 184.8 |
| 22 | `cli_stream` | 633 | 7 | 7/12 | 113 | 0 | 68 | M | 167.8 |
| 23 | `tools` | 717 | 9 | 9/12 | 78 | 0 | 68 | M | 165.2 |
| 24 | `collaboration_instructions` | 913 | 12 | 12/12 | 74 | 0 | 32 | M | 160.9 |
| 25 | `prompt_caching` | 1076 | 8 | 8/8 | 112 | 0 | 41 | M | 157.1 |
| 26 | `model_visible_layout` | 549 | 6 | 6/6 | 41 | 36 | 18 | M | 156.8 |
| 27 | `personality` | 829 | 12 | 12/16 | 72 | 0 | 19 | M | 152.8 |
| 28 | `user_shell_cmd` | 512 | 7 | 7/12 | 67 | 0 | 78 | M | 150.6 |
| 29 | `model_switching` | 1057 | 10 | 10/10 | 104 | 0 | 15 | M | 150.1 |
| 30 | `compact_resume_fork` | 867 | 4 | 4/4 | 154 | 13 | 10 | M | 149.5 |
| 31 | `shell_serialization` | 455 | 9 | 9/9 | 47 | 0 | 73 | M | 148.4 |
| 32 | `shell_command` | 311 | 13 | 9/16 | 25 | 0 | 49 | M | 144.1 |
| 33 | `pending_input` | 802 | 7 | 7/16 | 70 | 15 | 10 | M | 140.2 |
| 34 | `personality_migration` | 349 | 11 | 11/32 | 31 | 0 | 13 | M | 127.5 |
| 35 | `compact_remote_parity` | 1163 | 8 | 5/5 | 108 | 0 | 6 | M | 123.3 |
| 36 | `review` | 945 | 5 | 5/15 | 93 | 0 | 24 | M | 118.2 |
| 37 | `agent_websocket` | 429 | 7 | 7/7 | 96 | 0 | 14 | M | 116.0 |
| 38 | `permissions_messages` | 602 | 7 | 7/7 | 69 | 0 | 19 | M | 107.7 |
| 39 | `tool_parallelism` | 430 | 5 | 5/14 | 57 | 0 | 35 | M | 102.1 |
| 40 | `sqlite_state` | 528 | 6 | 6/30 | 60 | 0 | 5 | M | 101.8 |
| 41 | `tool_harness` | 556 | 5 | 5/5 | 77 | 0 | 23 | L | 98.7 |
| 42 | `safety_check_downgrade` | 408 | 7 | 7/7 | 73 | 0 | 4 | L | 97.3 |
| 43 | `exec_policy` | 401 | 5 | 5/5 | 42 | 0 | 35 | L | 88.0 |
| 44 | `exec` | 175 | 6 | 6/6 | 12 | 0 | 39 | L | 81.2 |
| 45 | `request_user_input` | 471 | 6 | 6/9 | 45 | 0 | 7 | L | 80.4 |
| 46 | `agent_jobs` | 449 | 4 | 4/4 | 58 | 0 | 16 | L | 75.5 |
| 47 | `websocket_fallback` | 251 | 4 | 4/7 | 75 | 0 | 2 | L | 74.3 |
| 48 | `web_search` | 278 | 5 | 5/5 | 58 | 0 | 2 | L | 71.7 |
| 49 | `resume` | 465 | 4 | 4/8 | 59 | 0 | 5 | L | 71.2 |
| 50 | `rollout_list_find` | 238 | 7 | 7/9 | 11 | 0 | 9 | L | 69.5 |
| 51 | `plugins` | 440 | 3 | 3/6 | 53 | 0 | 12 | L | 64.5 |
| 52 | `request_permissions_tool` | 510 | 2 | 2/2 | 40 | 0 | 27 | L | 59.4 |
| 53 | `hooks_mcp` | 464 | 3 | 3/3 | 56 | 0 | 4 | L | 58.5 |
| 54 | `models_cache_ttl` | 369 | 4 | 4/6 | 36 | 0 | 5 | L | 57.4 |
| 55 | `codex_delegate` | 243 | 3 | 3/3 | 37 | 0 | 11 | L | 51.7 |
| 56 | `abort_tasks` | 257 | 3 | 3/7 | 31 | 0 | 9 | L | 50.3 |
| 57 | `windows_sandbox` | 244 | 2 | 2/2 | 10 | 0 | 38 | L | 49.4 |
| 58 | `live_cli` | 154 | 2 | 0/3 | 19 | 0 | 29 | L | 43.4 |
| 59 | `skill_approval` | 287 | 2 | 2/2 | 11 | 0 | 28 | L | 43.4 |
| 60 | `turn_state` | 148 | 2 | 2/2 | 42 | 0 | 6 | L | 42.1 |
| 61 | `responses_api_proxy_headers` | 242 | 1 | 1/9 | 34 | 0 | 11 | L | 40.4 |
| 62 | `spawn_agent_description` | 232 | 1 | 1/3 | 24 | 0 | 23 | L | 39.5 |
| 63 | `mcp_turn_metadata` | 321 | 2 | 2/2 | 37 | 0 | 3 | L | 39.2 |
| 64 | `agents_md` | 142 | 3 | 3/3 | 15 | 0 | 9 | L | 38.3 |
| 65 | `deprecation_notice` | 144 | 3 | 3/3 | 14 | 0 | 7 | L | 36.4 |
| 66 | `override_updates` | 145 | 3 | 3/3 | 8 | 0 | 9 | L | 34.8 |
| 67 | `fork_thread` | 252 | 2 | 2/2 | 29 | 0 | 0 | L | 32.4 |
| 68 | `request_plugin_install` | 166 | 1 | 1/1 | 25 | 5 | 0 | L | 31.9 |
| 69 | `models_etag_responses` | 160 | 1 | 1/2 | 31 | 0 | 9 | L | 31.8 |
| 70 | `image_rollout` | 269 | 2 | 2/4 | 19 | 0 | 4 | L | 31.8 |
| 71 | `openai_file_mcp` | 225 | 1 | 1/2 | 41 | 0 | 0 | L | 31.2 |
| 72 | `request_compression` | 120 | 2 | 2/2 | 24 | 0 | 0 | L | 28.6 |
| 73 | `json_result` | 117 | 2 | 2/2 | 17 | 0 | 2 | L | 26.5 |
| 74 | `window_headers` | 151 | 1 | 1/1 | 30 | 1 | 0 | L | 26.2 |
| 75 | `hierarchical_agents` | 96 | 2 | 2/2 | 17 | 0 | 1 | L | 25.6 |
| 76 | `unstable_features_warning` | 109 | 2 | 2/4 | 5 | 0 | 6 | L | 24.6 |
| 77 | `model_overrides` | 94 | 2 | 2/3 | 9 | 0 | 4 | L | 24.3 |
| 78 | `stream_error_allows_next_turn` | 133 | 1 | 1/1 | 22 | 0 | 2 | L | 21.4 |
| 79 | `user_notification` | 83 | 1 | 1/2 | 17 | 0 | 3 | L | 19.8 |
| 80 | `stream_no_completed` | 103 | 1 | 1/1 | 19 | 0 | 2 | L | 19.6 |
| 81 | `skills` | 125 | 1 | 1/1 | 13 | 0 | 6 | L | 19.6 |
| 82 | `resume_warning` | 132 | 1 | 1/3 | 4 | 0 | 6 | L | 16.6 |
| 83 | `quota_exceeded` | 78 | 1 | 1/1 | 14 | 0 | 0 | L | 15.5 |
| 84 | `prompt_debug_tests` | 61 | 1 | 1/1 | 4 | 0 | 4 | L | 13.1 |

## Suggested Split Lanes

The membership below covers all 84 modules exactly once.

| Lane | Modules | Lines | Tests~ | Tokio tests | Mock/net | Snap/golden | Process |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `core_tests_process_exec_shell_lane` | 11 | 8293 | 168 | 115 | 767 | 121 | 1140 |
| `core_tests_policy_hooks_lane` | 8 | 11260 | 87 | 82 | 974 | 5 | 959 |
| `core_tests_client_streaming_lane` | 10 | 11215 | 135 | 135 | 1588 | 42 | 174 |
| `core_tests_compaction_context_lane` | 8 | 11729 | 92 | 89 | 1552 | 151 | 179 |
| `core_tests_remote_tools_media_lane` | 17 | 16783 | 179 | 176 | 2162 | 9 | 646 |
| `core_tests_agents_state_misc_lane` | 30 | 10523 | 125 | 125 | 1116 | 25 | 316 |

### `core_tests_process_exec_shell_lane`

Exact module set:

```text
abort_tasks
apply_patch_cli
exec
exec_policy
live_cli
shell_command
shell_serialization
shell_snapshot
unified_exec
user_shell_cmd
windows_sandbox
```

Rationale: isolates process spawning, shell serialization/snapshots,
apply-patch CLI dispatch, unified exec behavior, and sandbox platform tests.
This is process-heavy and should get a dedicated release lane because failures
here tend to be operational and slow to diagnose in a combined binary.

Platform notes: preserve existing cfg behavior for `abort_tasks`
non-Windows and `windows_sandbox` Windows-only.

### `core_tests_policy_hooks_lane`

Exact module set:

```text
approvals
hooks
hooks_mcp
permissions_messages
request_permissions
request_permissions_tool
safety_check_downgrade
skill_approval
```

Rationale: groups approval, permission, hook, and downgrade policy tests. The
lane is policy/process heavy and mostly non-Windows-gated in the largest files,
so keeping it separate avoids mixing platform-gated policy behavior with
client/mock lanes.

Platform notes: preserve existing non-Windows cfgs for `approvals`, `hooks`,
`hooks_mcp`, `request_permissions`, and `request_permissions_tool`.

### `core_tests_client_streaming_lane`

Exact module set:

```text
agent_websocket
cli_stream
client
client_websockets
models_cache_ttl
models_etag_responses
realtime_conversation
responses_api_proxy_headers
websocket_fallback
window_headers
```

Rationale: keeps HTTP/WebSocket/client streaming behavior together. This is one
of the highest mock/network lanes and should be independently runnable when
response transport, websocket fallback, headers, or realtime behavior changes.

### `core_tests_compaction_context_lane`

Exact module set:

```text
compact
compact_remote
compact_remote_parity
compact_resume_fork
model_visible_layout
prompt_caching
request_compression
truncation
```

Rationale: isolates context compaction, remote compaction parity, model-visible
layout, prompt caching, request compression, and truncation. This lane is
mock-heavy and has the strongest snapshot/golden signal outside shell snapshots.

### `core_tests_remote_tools_media_lane`

Exact module set:

```text
code_mode
items
mcp_turn_metadata
openai_file_mcp
otel
plugins
remote_env
remote_models
request_plugin_install
rmcp_client
search_tool
skills
tool_harness
tool_parallelism
tools
view_image
web_search
```

Rationale: groups remote/model/tool/media integrations and their mock-heavy
support tests. It contains several high-rank modules (`code_mode`,
`rmcp_client`, `otel`, `view_image`, `remote_models`, `search_tool`) and would
be the next candidate for subdivision if six lanes are not enough.

### `core_tests_agents_state_misc_lane`

Exact module set:

```text
agent_jobs
agents_md
codex_delegate
collaboration_instructions
deprecation_notice
fork_thread
hierarchical_agents
image_rollout
json_result
model_overrides
model_switching
override_updates
pending_input
personality
personality_migration
prompt_debug_tests
quota_exceeded
request_user_input
resume
resume_warning
review
rollout_list_find
spawn_agent_description
sqlite_state
stream_error_allows_next_turn
stream_no_completed
subagent_notifications
turn_state
unstable_features_warning
user_notification
```

Rationale: gathers smaller agent orchestration, state, resume, personality,
model switching, warning, and low-level turn/result tests. The module count is
high, but the cost is mostly medium/low and avoids loading the heavier process,
client, compaction, and remote-tool lanes when iterating on state behavior.

## First Lane To Implement

Implement `core_tests_process_exec_shell_lane` first.

Reasons:

- It gives immediate value for release test iteration because the lane has the
  highest process/shell signal and contains the two top ranked modules:
  `apply_patch_cli` and `unified_exec`.
- It exercises the shared `suite_bootstrap.rs` dispatch-alias boundary early.
  That boundary is likely to be the most important harness risk, so proving it
  first reduces uncertainty for later lanes.
- The lane is coherent enough that failures should point to shell/process/test
  binary dispatch behavior instead of generic core/client mock plumbing.

Suggested implementation shape:

- Keep `suite_bootstrap.rs` shared above all lane binaries, or move its logic to
  a clearly shared test-support module imported by every split binary that can
  spawn first-party helpers.
- Preserve platform cfgs exactly for `abort_tasks` and `windows_sandbox`.
- If the first patch needs a smaller mechanical step, create the lane target
  with the lower-risk shell modules first, then add `apply_patch_cli` and
  `unified_exec` after dispatch aliases are confirmed in the new binary.

## Verification Strategy After Splitting

Scout did not run verification commands. Recommended implementation verification:

1. Avoid debug-profile Cargo lanes in this checkout.
2. After adding the first split target, run the smallest release filter that
   exercises the new lane or moved modules, for example:

   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Filter <new_lane_or_module_filter>
   ```

3. For `core_tests_process_exec_shell_lane`, start with module filters around
   `shell_command`, `shell_serialization`, `shell_snapshot`, `exec`,
   `exec_policy`, and platform-local sandbox coverage, then run the whole new
   lane filter once aliases are proven.
4. Add `apply_patch_cli` and `unified_exec` verification before considering the
   lane complete, because they are the costliest and most dispatch-sensitive
   process modules.
5. Run neighboring lane filters only when shared test support, bootstrap, or
   common module plumbing changes.
6. Defer broad `codex-core` release validation until several split lanes exist
   and root/user approves the larger run. Do not use debug-profile Cargo tests.
7. If snapshot output changes in `shell_snapshot`, `compact*`, or
   `model_visible_layout`, inspect pending `*.snap.new` files before accepting
   them; splitting alone should not intentionally change snapshots.
