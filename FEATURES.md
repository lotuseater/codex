# Fork Feature Catalog -- claude-automation-toolkit

This file is the authoritative catalog of features added by the `claude-automation-toolkit`
fork on top of `openai/codex`. Use it as the KEEP-checklist when merging upstream: every
feature listed here must be verified wired after a merge. Last substantive update: 2026-06-24.

---

## Feature Index

**User-facing**
- Token-usage % display
- Session-limit footer
- `[local-fork]` startup marker
- Build-stamp in `--version`
- Self-review (git-grounded review loop)
- Slash commands: `/action-prompt`, `/batch-prompt`, `/delegate-prompt`, `/compact-config`

**Runtime / economy**
- Semantic auto-compact (context-reduction pipeline)
- Prompt economy / recency-weighted reduction (`codex-prompt-reducer`)
- Token-economy routing (`ContextBudgetMode` Slow/Standard)
- Elide-repeated-tool-outputs
- Defer-MCP-tools (`ToolSearchAlwaysDeferMcpTools`)

**Infra / coordination**
- Operation-cache bridge
- First-moves logic
- Repo-blackboard cross-session coordination
- Cognos-ops scouts
- Multi-agent `wait_agent` v2
- Repo-context-scout (Off/Shadow/Tool modes)
- Context-ops shadow/replace
- Memory/MCP cache-status APIs
- Agent-policy spawn control
- Context-pack / entrypoint hints
- Workflow batch
- Reasoning logic (RustBaselineReasoner / SwiplReasoner)
- Problem memory context (memories/context)

---

## User-facing features

### Token-usage % display

**Functional behavior.** The TUI right-side context line shows the active agent's context
window fill as a percentage (e.g. `--%  used`). Each agent in the multi-agent panel
carries its own independent percentage. The value is updated after every turn via server
token-usage events.

**Runtime behavior.** On each `TokenContextPercentUsed` event from the server the app
calls `update_agent_token_context_percent_used(thread_id, pct)`. The percentage is stored
per-agent-entry, forwarded to the collab-agents widget and to the session-limit footer on
every sync cycle.

**Implementation.**
- `codex-rs/tui/src/app/agent_navigation.rs` -- `update_token_context_percent_used()`,
  `token_context_percent_used: Option<i64>` field on `AgentNavigationEntry`
- `codex-rs/tui/src/app/thread_routing.rs` -- calls `token_context_percent_used()` from
  `multi_agents` module
- `codex-rs/tui/src/app/session_lifecycle.rs` -- `update_agent_token_context_percent_used()`
- `codex-rs/tui/src/chatwidget/collab_agents.rs` -- `set_collab_agent_token_context_percent_used()`
- `codex-rs/tui/src/bottom_pane/chat_composer.rs` -- `set_context_window(percent, used_tokens)`
- `codex-rs/tui/src/chatwidget/session_limit_footer.rs` -- renders token percent in footer

**Tests.** YES -- `codex-rs/tui/src/app/tests/token_usage.rs` (token_context_percent_used
entry checks); `codex-rs/tui/src/app/tests/inactive_threads.rs` verifies percent update.
Run: `cargo test -p codex-tui token_usage`

---

### Session-limit footer

**Functional behavior.** A footer line in the bottom-right of the TUI displays: token
context percentage used, plus one or two rate-limit reset countdowns (e.g. remaining
time until the next Codex request window resets). The line is hidden when there is no
active rate-limit or token data.

**Runtime behavior.** `sync_session_limit_footer()` is called on every rate-limit update,
on context-window updates, and when the token-usage percent changes. It calls
`session_limit_footer::line(...)` which assembles a `ratatui::text::Line` from available
data and passes it to the footer-right slot via
`set_session_limit_status_line()`.

**Implementation.**
- `codex-rs/tui/src/chatwidget/session_limit_footer.rs` -- `line()` (public, pure function)
- `codex-rs/tui/src/chatwidget/status_controls.rs:379` -- `sync_session_limit_footer()`
- `codex-rs/tui/src/chatwidget.rs:374` -- `mod session_limit_footer`; call sites at
  lines 1181, 1191, 1217
- `codex-rs/tui/src/bottom_pane/chat_composer.rs:4366` -- renders `right_footer_line_with_context()`
- `codex-rs/tui/src/chatwidget/rate_limits.rs:285` -- triggers sync on rate-limit update

**Tests.** YES -- `codex-rs/tui/src/chatwidget/tests/session_limit_footer.rs` with two
snapshot tests (`session_limit_footer_right_status`, `session_limit_footer_with_side_context`).
Run: `cargo test -p codex-tui session_limit_footer`

---

### `[local-fork]` startup marker

**Functional behavior.** The session-start history cell (the banner shown when Codex opens
a session) includes a `[local-fork]` / `local build <stamp>` label to distinguish the fork
binary from an upstream release.

**Runtime behavior.** At render time, `local_fork_version_label()` reads `WIZARD_CODEX_LOCAL_BUILD_STAMP`
(primary) or `CODEX_LOCAL_BUILD_STAMP` (fallback) from the runtime environment. If neither
is set, falls back to `"local source build"`. The resulting span is inserted into the
session-header history cell.

**Implementation.**
- `codex-rs/tui-render/src/version.rs` -- `local_fork_version_label() -> String`
- `codex-rs/tui-render/src/history_cell/session.rs:336` -- inserts the span into the cell

**Tests.** YES -- `codex-rs/tui-render/src/history_cell/tests.rs:1499`
(`session_header_shows_local_fork_version_label`).
Run: `cargo test -p codex-tui-render session_header_shows_local_fork_version_label`

---

### Build-stamp in `--version`

**Functional behavior.** `codex --version` outputs `<semver> (local build <stamp>)` when
the binary was built with a `CODEX_LOCAL_BUILD_STAMP` env var set, instead of the bare
semver. Without a stamp the output is just the semver, unchanged from upstream.

**Runtime behavior.** The stamp is baked in at compile time via `build.rs` using
`cargo:rustc-env=CODEX_LOCAL_BUILD_STAMP`. At runtime `option_env!("CODEX_LOCAL_BUILD_STAMP")`
is passed to `display_version()` in the version-utilities crate, which formats the string.

**Implementation.**
- `codex-rs/cli/build.rs` -- bakes `CODEX_LOCAL_BUILD_STAMP` at compile time
- `codex-rs/cli/src/main.rs:972` -- calls `display_version(CODEX_CLI_VERSION, option_env!(...))`
- `codex-rs/cli/src/dispatch.rs:411` -- same stamp injection for the `dispatch` sub-path
- `codex-rs/utils/cli/src/version.rs` -- `display_version(package_version, local_build_stamp)` (public)
- `codex-rs/exec/src/event_processor_with_human_output.rs:219` -- also injects stamp

**Tests.** YES -- `codex-rs/utils/cli/src/version.rs` has two inline tests:
`display_version_uses_package_version_without_local_build_stamp` and
`display_version_appends_local_build_stamp`.
Run: `cargo test -p codex-utils-cli display_version`

---

### Self-review (git-grounded review loop)

**Functional behavior.** After a configurable number of turns (or when a plan completes),
Codex emits a self-review checkpoint prompt containing exact `git diff` commands and file
baselines captured at review start, instructing the model to review its own recent
changes. A `GitReviewAnchor` captures HEAD + dirty-file snapshots at the start of each
review interval. Task-memory (`codex-task-memory`) builds a token-budgeted `<task_memory>`
block (active request + directives + latest plan) injected under context pressure and
preserved across compaction.

**Runtime behavior.** `SelfReviewTracker` tracks turn count and git-evidence state.
`should_remind(now)` gates whether a review reminder is emitted. `review_instructions()`
and `plan_tool_response()` produce the injected prompts. `maybe_inject_task_memory_for_sampling()`
in core injects the task-memory block when context pressure crosses a threshold.

**Implementation.**
- `codex-rs/self-review/src/lib.rs` -- `SelfReviewTracker` (pub struct), `GitReviewAnchor`,
  `SELF_REVIEW_CHECKPOINT_MESSAGE`, `review_instructions()`, `plan_tool_response()`,
  `is_plan_review_candidate()`, `plan_self_review_prompt()`
- `codex-rs/core/src/session/context_budget.rs` -- `maybe_inject_task_memory_for_sampling()`
- `codex-rs/core/src/compact.rs`, `compact_remote.rs`, `compact_remote_v2.rs` -- inject
  task-memory preservation in all three compaction paths
- `codex-rs/tui/src/chatwidget/review.rs` -- TUI-side review loop orchestration
- `codex-rs/core/src/tools/handlers/plan.rs:107,112` -- WIRED: calls
  `codex_self_review::is_plan_review_candidate()` then
  `codex_self_review::plan_tool_response(include_checkpoint)` to inject the checkpoint
  into the plan-tool response

**Tests.** YES -- `codex-rs/self-review/src/lib.rs` and `git_evidence.rs` contain
`#[cfg(test)]` blocks with 10+ tests. The plan-tool checkpoint (`plan_tool_response`,
`is_plan_review_candidate`) is exported and currently wired into
`codex-rs/core/src/tools/handlers/plan.rs` (lines 107, 112) at this working-tree state.
Run: `cargo test -p codex-self-review`

---

### Slash commands: /action-prompt, /batch-prompt, /delegate-prompt, /compact-config

**Functional behavior.**
- `/action-prompt [status|on|off|<variant>|custom <text>|clear]` -- toggles and configures
  the action-mode prompt injection variant (controls which token-economy injection is
  prepended to each user message in action/auto-loop mode).
- `/batch-prompt [status|on|off|<variant>|custom <text>|clear]` -- same for batch/mini-programming
  prompt injection.
- `/delegate-prompt [k <n>|status|...]` -- controls multi-agent delegation parameters
  (`plan_token_economy_delegation_k`, usage-hint injection).
- `/compact-config [on|off|<pct>|custom <text>|status]` -- toggles auto-compact, sets
  compact trigger percentage, and sets a custom compact prompt.

**Runtime behavior.** All four are parsed in `chatwidget/slash_dispatch.rs` under their
`SlashCommand::*` enum arms. Each writes to the on-disk config via the `config/src/edit.rs`
helpers (`/delegate-prompt` at line 141; `/compact-config` at line 205) so changes survive
session restart.

**Implementation.**
- `codex-rs/tui/src/chatwidget/slash_dispatch.rs:288,291,294,297` -- dispatch to
  `show_action_prompt_status()`, `show_batch_prompt_status()`,
  `show_delegate_prompt_status()`, `show_compact_config_status()`
- `codex-rs/tui/src/chatwidget/slash_dispatch.rs:759,762,765,768` -- handlers
  `handle_action_prompt_command_args()`, `handle_batch_prompt_command_args()`,
  `handle_delegate_prompt_command_args()`, `handle_compact_config_command_args()`
- `codex-rs/tui/src/chatwidget/prompt_injection.rs` -- `action_variant_from_token()`,
  `batch_variant_from_token()`, `ACTION_VARIANTS`, `BATCH_VARIANTS` constants
- `codex-rs/tui/src/chatwidget/delegate_prompt.rs` -- `/delegate-prompt` handler
- `codex-rs/tui/src/chatwidget/compact_config.rs` -- `/compact-config` handler
- `codex-rs/config/src/edit.rs:141,205` -- config-edit helpers for delegate and compact
- `codex-rs/tui/src/app/event_dispatch.rs:2277-2385` -- persist action/batch prompt state

**Tests.** YES -- snapshot tests for slash-command status output exist in
`codex-rs/tui/src/chatwidget/tests/`. The `app/event_dispatch.rs` error paths are
partially covered via chatwidget integration tests.
Run: `cargo test -p codex-tui action_prompt` / `cargo test -p codex-tui compact_config`

---

## Runtime / economy features

### Semantic auto-compact (context-reduction pipeline)

**Functional behavior.** In addition to upstream's token-limit-triggered compact, the fork
adds a *semantic* auto-compact that fires based on: visible-context-percent-used crossing
a configurable threshold (default 20 % remaining), a turn-cooldown (default 24 turns),
work-checkpoint heuristics (6 turns or 32k tokens of new work), and a tool-checkpoint
(12 tool calls). The compaction prompt is `PRUNE_NUDGE_PROMPT` -- instructs the model to
preserve the full plan and next steps while removing low-signal history.

**Runtime behavior.** After each sampling response, `post_sampling_auto_compact_action()`
evaluates a `SemanticCompactDecision` (Compact, Defer, Skip). If Compact, one of three
async compact tasks fires: `run_inline_auto_compact_task()` (local), or its remote /
remote-v2 variants. The decision is coordinated via `context_budget_adapter.rs`.

**Implementation.**
- `codex-rs/context-reduction/src/lib.rs` -- `ContextReductionPolicy`, `SemanticCompactInput`,
  `SemanticCompactDecision`, `post_sampling_auto_compact_action()`,
  `auto_compact_token_limit_for_mode()`, `PRUNE_NUDGE_PROMPT`,
  `DEFAULT_TRIGGER_CONTEXT_PERCENT`, `DEFAULT_TURN_COOLDOWN`
- `codex-rs/core/src/context_reduction_adapter.rs` -- `semantic_auto_compact_enabled()`,
  `semantic_compact_input()`, `auto_compact_budget_mode()`, `model_auto_compact_limits()`
- `codex-rs/core/src/session/context_budget_adapter.rs` -- orchestrates the call chain,
  keeping all adapter imports out of upstream-hot `turn.rs`
- `codex-rs/core/src/compact.rs:102` -- `run_inline_auto_compact_task()`
- `codex-rs/core/src/compact_remote.rs:55`, `codex-rs/core/src/compact_remote_v2.rs:58` --
  remote variants
- `codex-rs/core/src/compact_token_budget.rs:49` -- token-budget variant

**Tests.** PARTIAL -- `codex-rs/core-test-suites/compact/` has extensive tests for
compaction shape, limits, reasoning, and resume (`compact_limits.rs` tests
`auto_compact_body` at lines 928, 991). Direct unit tests for `post_sampling_compaction_decision`
are absent; coverage is through integration fixtures.
Run: `cargo test -p codex-core-test-compact`

---

### Prompt economy / recency-weighted reduction (codex-prompt-reducer)

**Functional behavior.** Before sending the full conversation history to the model, the
fork optionally reduces it by: (a) bundling and eliding repeated short tool-call outputs
into a `[N similar outputs omitted]` notice; (b) bundling short assistant-status messages;
(c) applying a recency-weighted tier policy -- most recent N items are preserved fully,
older items are progressively truncated to `EXCERPT_CHARS` (220) excerpts with a
`[stale: <source>]` label. The reduction is prompt-only; the persisted rollout history is
never mutated.

**Runtime behavior.** `codex-rs/core/src/session/turn/prompt_reduction.rs` calls the
reducer before each sampling request. `RecencyWeightedOpts` / `RecencyPolicy` configure
tier depths and preservation counts. Configured at runtime via `/compact-config` or
`context_budget_mode`.

**Implementation.**
- `codex-rs/prompt-reducer/src/lib.rs` -- main reduction entry point; constants
  `DEFAULT_MIN_REDUCE_CHARS`, `DEFAULT_PRESERVE_RECENT_ITEMS`, `EXCERPT_CHARS`,
  `SHORT_TOOL_BUNDLE_MIN_ITEMS`
- `codex-rs/prompt-reducer/src/recency.rs` -- `RecencyPolicy`, `RecencyWeightedOpts`,
  `RecencyTier`, `TierKind`, `recency_weighted_tiers()`, `conservative_tiers()`,
  `CANONICAL_CATEGORY_NAMES`
- `codex-rs/prompt-reducer/src/stale_notice_bundle.rs` -- `bundle_stale_reduction_notices()`
- `codex-rs/prompt-reducer/src/source_label.rs` -- `compact_source_label()`
- `codex-rs/core/src/session/turn/prompt_reduction.rs` -- call site in core

**Tests.** YES -- `codex-rs/prompt-reducer/src/batch_reduction_tests.rs` (inline test module).
`codex-rs/prompt-reducer/src/recency.rs` contains inline `#[cfg(test)]` tests at line 322+.
Run: `cargo test -p codex-prompt-reducer`

---

### Token-economy routing (ContextBudgetMode)

**Functional behavior.** A `context_budget_mode` config key (values: `Slow` (default) or
`Standard`) makes the fork token-frugal end-to-end: `Slow` tightens the auto-compact
trigger limit, clamps tool-output truncation, restricts first-moves injection depth, and
enables the semantic auto-compact pipeline. `Standard` matches upstream behavior. The
active mode is carried per-turn via `TurnContext` and per-thread via protocol.

**Runtime behavior.** `resolve_context_budget_mode()` in config-loaders resolves the
effective mode from CLI override -> config-profile -> config file -> default (`Slow`).
`ContextBudgetMode` is propagated via `SessionConfiguration` into the turn pipeline and
into `context_budget_adapter.rs` where it selects compaction limits via
`auto_compact_budget_mode()`.

**Implementation.**
- `codex-rs/core/src/config/context_budget.rs` -- `resolve_context_budget_mode()` (fork-local)
- `codex-rs/core/src/config/config_struct.rs:21` -- `context_budget_mode: ContextBudgetMode`
- `codex-rs/core/src/config/config_types.rs:190` -- `context_budget_mode: Option<ContextBudgetMode>`
- `codex-rs/core/src/context_reduction_adapter.rs:6,66,69,70` -- maps `ContextBudgetMode`
  to `AutoCompactBudgetMode`; `auto_compact_budget_mode()`, `model_auto_compact_limits()`
- `codex-rs/core/src/codex_thread.rs:356,395` -- carries `context_budget_mode` per-thread

**Tests.** PARTIAL -- `codex-rs/core/src/config/config_tests.rs` imports `ContextBudgetMode`
and tests config loading; direct decision-path tests are absent.
Run: `cargo test -p codex-core config_tests`

---

### Elide-repeated-tool-outputs

**Functional behavior.** When the same large tool output appears more than once in the
conversation history, subsequent copies are replaced with `"Repeated tool output omitted
to save prompt tokens. Call ID: <id>, sha1: <hash>"`. Small outputs (below a token
threshold) are never elided. This applies to both function-call outputs and custom-tool
outputs.

**Runtime behavior.** `elide_repeated_large_tool_outputs()` is called inside `for_prompt()`
in the history context-manager before each sampling request. It tracks seen content by
sha1 hash and call-id using a `BTreeSet<(String, String)>`.

**Implementation.**
- `codex-rs/core/src/context_manager/prompt_elision.rs` -- `elide_repeated_large_tool_outputs()`,
  `maybe_elide_tool_output()`
- `codex-rs/core/src/context_manager/history.rs:135` -- call site inside `for_prompt()`

**Tests.** YES -- `codex-rs/core/src/context_manager/history_tests.rs:1119,1144,1181`
(`for_prompt_elides_repeated_large_function_tool_outputs`,
`for_prompt_keeps_small_repeated_function_tool_outputs`,
`for_prompt_elides_repeated_large_custom_tool_outputs`).
Run: `cargo test -p codex-core for_prompt_elides`

---

### Defer-MCP-tools (ToolSearchAlwaysDeferMcpTools)

**Functional behavior.** When enabled (default on for local token-savings), MCP-hosted
tools are not exposed to the model at session start. Instead, a `tool_search` tool is
offered; the model must explicitly search for MCP tools before they appear in its tool
list. This prevents large MCP tool schemas from inflating every prompt.

**Runtime behavior.** The in-core split happens in `mcp_tool_exposure.rs`: tools are
split into `direct_tools` (bootstrap/wizard-codex tools exposed immediately) and
`deferred_tools` (loaded only via `tool_search`). Deferral activates when
`Feature::ToolSearchAlwaysDeferMcpTools` is enabled OR `deferred_tools.len() >=
DIRECT_MCP_TOOL_EXPOSURE_THRESHOLD` (100). `deferred_mcp_loading_enabled` is exposed in
the `mcp/cache/status` API response.

**Implementation.**
- `codex-rs/core/src/mcp_tool_exposure.rs` -- in-core split logic; `McpToolExposure`,
  `build_mcp_tool_exposure()`, `DIRECT_MCP_TOOL_EXPOSURE_THRESHOLD = 100` (pub(crate) const),
  `BOOTSTRAP_MCP_TOOL_NAMES` (line 17), `filter_bootstrap_mcp_tools()` (line 85), reads
  `Feature::ToolSearchAlwaysDeferMcpTools`. (File is clean at this working-tree state -- no
  conflict markers.)
- `codex-rs/core/src/session/turn.rs` -- call site for the exposure split
- `codex-rs/features/src/` -- `Feature::ToolSearchAlwaysDeferMcpTools` feature flag
- `codex-rs/app-server/src/request_processors/mcp_processor.rs:375` -- reads feature to
  set `deferred_mcp_loading_enabled`
- `codex-rs/app-server-protocol/src/protocol/v2/mcp.rs:78` -- `deferred_mcp_loading_enabled: bool`
  in MCP cache-status response
- `codex-rs/core/config.schema.json:838,5372` -- `tool_search_always_defer_mcp_tools` key

**Tests.** YES -- `codex-rs/core/src/mcp_tool_exposure_test.rs` (dedicated test file with
several `#[tokio::test]` cases: `directly_exposes_small_effective_tool_sets_when_always_defer_disabled`,
`excludes_tools_hidden_from_model_exposure`, `applies_per_tool_app_policy_across_the_exposure_build`).
Also `codex-rs/core-test-suites/tools-router/tests/suite/search_tool_mcp.rs:173` and
`search_tool.rs:84,249` exercise the deferred path; `codex-rs/features/src/tests.rs:222`
asserts the feature is enabled by default.
Run: `cargo test -p codex-core mcp_tool_exposure`

---

## Infra / coordination features

### Operation-cache bridge

**Functional behavior.** Before dispatching a tool call, the fork consults a shared SQLite
cache (`~/.claude/cache/tool_cache.sqlite`, shared with Claude Code hooks) via a Python
bridge process. On a cache hit, the stored tool result is returned without executing the
real tool. Successful tool results are stored back to the cache. Companion cognos op
`operation_cache_stats` and app-server v2 `mcp/cache/status` report cache state.

**Runtime behavior.** `lookup()` is called pre-dispatch; on hit it returns `OperationCacheHit`.
On success, `store()` persists the result. The bridge is a Python subprocess resolved via
`WIZARD_CODEX_OPERATION_CACHE` / `CODEX_BRIDGE_PY` env vars or default candidate paths.
`tool_is_cacheable()` gates which tools are eligible (DAB tools are excluded).

**Implementation.**
- `codex-rs/operation-cache/src/lib.rs` -- `OperationCacheHit`, `lookup()`, `store()`,
  `tool_is_cacheable()` (owner crate)
- `codex-rs/core/src/tools/operation_cache.rs` -- thin core adapter (`cwd()`, `lookup()`,
  `store()`, `result_from_hit()`)
- `codex-rs/core/src/tools/mod.rs:10` -- WIRED: `pub(crate) mod operation_cache;`
- `codex-rs/core/src/tools/registry.rs` -- WIRED: `operation_cache::cwd()` at line 883,
  `operation_cache::lookup()` at line 890 (sets `served_from_operation_cache`),
  `operation_cache::result_from_hit()` at line 899, `operation_cache::store()` at line 1014
- `codex-rs/cognos-ops/src/lib.rs` -- `operation_cache_stats()` cognos op
- `codex-rs/app-server-protocol/src/protocol/common/client_requests.rs:910` -- `mcp/cache/status`
  experimental API endpoint

**Tests.** YES (owner crate) -- unit tests in `codex-rs/operation-cache/src/lib.rs`
(cache_event, cache_scope, namespace sanitization, output_text, dab exclusion).
Dedicated registry-wiring tests: tests: NONE (the lookup/store hooks in registry.rs are
exercised only indirectly).
Run: `cargo test -p codex-operation-cache`

---

### First-moves logic

**Functional behavior.** On every fresh turn, the fork predicts which files and searches
the model will likely need first (based on intent keywords and per-repo sqlite hit history)
and injects a `<first_moves>` context block with those paths/searches. Hit-recording
(learning from which predicted files the model actually used) runs after each tool-use
call. `first_moves_stats` reports hit-rate. Configured by `first_moves.enabled` in config.

**Runtime behavior.** `predict()` scans the repo (up to 2000 files, sync), scores
candidates via keyword + hit-history matching, and returns a `FirstMovesBundle`. The
bundle is formatted by `format_first_moves_context()` and prepended to the fresh-turn
prompt. `spawn_record_tool_use_hit()` fires post-tool-use to update the sqlite DB, closing
the learning loop. The three bootstrap MCP tools (`first_moves_logic_advice`,
`first_moves_predict`, `first_moves_stats`) are always exposed directly even when other
MCP tools are deferred (see Defer-MCP-tools).

**Implementation.**
- `codex-rs/first-moves/src/lib.rs` -- `FirstMove`, `FirstMovesBundle`, `FirstMovesConfig`,
  `FirstMovesError`, `predict()`, `storage_for()`, `stats()`,
  `format_first_moves_context()`, `is_whole_repo_exploration_prompt()`,
  `record_tool_use_hit()`
- `codex-rs/core/src/session/first_moves.rs` -- `context_pack_for_fresh_turn()` call site;
  injects bundle into fresh-turn prompt (runs sync -- known blocking I/O issue)
- `codex-rs/core/src/mcp_tool_exposure.rs:17` -- `BOOTSTRAP_MCP_TOOL_NAMES`;
  `filter_bootstrap_mcp_tools()` at line 85 keeps the first-moves tools direct
- `codex-rs/core/src/tools/handlers/first_moves.rs:148` -- WIRED: `spawn_record_tool_use_hit()`
  (re-exported at `handlers/mod.rs:73`), called from `codex-rs/core/src/tools/registry.rs:1112`
  after each tool use

**Tests.** NONE confirmed in `lib.rs`. The sqlite storage and predict modules have no
verified `#[cfg(test)]` blocks.
Run: `cargo test -p codex-first-moves`

---

### Repo-blackboard cross-session coordination

**Functional behavior.** Agents coordinate across sessions via a `.codex/blackboard.md`
file (project-local, git-ignored by convention). Agents append typed events (intent,
proposal, claim, ack, done, abort) with fencing and sequence numbers; each session reads
the blackboard at turn start and injects an `<external_update>` block into the prompt
when new events are found. File locks prevent concurrent corruption; size caps prevent
unbounded growth.

**Runtime behavior.** `BlackboardSession::append_event()` and `read_events()` run on each
turn via `codex-rs/core/src/session/blackboard.rs`. `active_external_update()` returns
the current pending update for prompt injection. Events are parsed with corrupt-line
tolerance.

**Implementation.**
- `codex-rs/blackboard/src/lib.rs` -- `BlackboardSession`, `BlackboardEvent`, `GlobalIndexRecord`,
  `ExternalUpdate`, `append_event()`, `read_events()`, `active_external_update()`,
  `clear_if_no_active_external()`, `repo_id_for_root()`
- `codex-rs/core/src/session/blackboard.rs` -- core session integration (async I/O known
  to call sync git/fs -- blocking issue noted in FORK_FEATURES_REVIEW.md)

**Tests.** YES -- 9 tests in `codex-rs/blackboard/src/lib.rs` (parse_events, append
recovery, lock behavior, stale session filtering, terminal events).
Run: `cargo test -p codex-blackboard`

---

### Cognos-ops scouts

**Functional behavior.** Five specialized tool functions for code-intelligence and
memory-augmented look-ups: `problem_memory_lookup` (searches per-project memory indexes),
`code_relation_scout` (structural code-relation queries), `operation_cache_stats` (cache
health report), `evidence_fusion_summary` (fuses multiple evidence fragments into a
scored conclusion), `mission_trace_export` (exports the active mission trace).

**Runtime behavior.** Each function is registered as a Codex tool and dispatched through
the standard tool handler. They are implemented in the `codex-cognos-ops` owner crate and
exposed via a thin core adapter.

**Implementation.**
- `codex-rs/cognos-ops/src/lib.rs` -- `problem_memory_lookup()`, `code_relation_scout()`,
  `operation_cache_stats()`, `evidence_fusion_summary()`, `mission_trace_export()`

**Tests.** YES -- 6 tests in `codex-rs/cognos-ops/src/lib.rs` (mission trace, evidence
classification, problem memory scope, memory matching).
Run: `cargo test -p codex-cognos-ops`

---

### Multi-agent `wait_agent` v2

**Functional behavior.** Extends upstream's `wait_agent` tool with v2 semantics: target
resolution by name or mailbox, configurable timeout ranges, and activity-aware polling.
Companion v2 lifecycle tools: `compact_agent` (request compact on a sub-agent),
`close_agent` (force-close distinct from upstream `interrupt_agent`), `restart_agent`
(restart with optional model/effort override), `resume_agent` (resume from a stopped
state), `followup_task` (spawn follow-up with model/effort override).

**Runtime behavior.** `wait_agent` v2 handler in `multi_agents_v2/wait.rs` resolves the
target, subscribes to the mailbox, and polls with min/max/default timeout parameters
(configured via `WaitAgentTimeoutOptions`). Tool spec is built by `create_wait_agent_tool_v2()`
in `spec_plan.rs`.

**Implementation.**
- `codex-rs/core/src/tools/handlers/multi_agents_v2/wait.rs` -- v2 wait handler
- `codex-rs/core/src/tools/spec_plan_types.rs` -- `WaitAgentTimeoutOptions`
- `codex-rs/core/src/tools/spec.rs:50,53,56` -- applies timeout options to tool spec
- `codex-rs/core/src/tools/spec_tests.rs:476+` -- tool spec shape tests for `wait_agent`
- `codex-rs/core/src/tools/handlers/multi_agents_tests/wait_v2.rs` -- behavioral tests

**Tests.** YES -- `wait_v2.rs` tests; `spec_tests.rs` covers tool spec shape.
Run: `cargo test -p codex-core wait_agent`

---

### Repo-context-scout (Off/Shadow/Tool modes)

**Functional behavior.** Builds a ranked, git-aware index of the repo (files, git-changed
areas, ranked candidates) and produces a `ScoutBundle` packet. Three modes: `Off` (disabled),
`Shadow` (bundle injected silently into context -- default), `Tool` (exposed as a callable
tool). In Shadow mode the scout packet augments the fresh-turn context automatically.

**Runtime behavior.** The scout index is built via `build_index()` on first use and
cached. `read_changed_areas()` retrieves recent git changes. Ranking uses keyword and
file-recency scoring. The shadow path uses `spawn_blocking` for the async executor.

**Implementation.**
- `codex-rs/repo-context-scout/src/lib.rs` -- `RepoContextScoutMode`, `ScoutRequest`,
  `ScoutBundle`, `ScoutCandidate`, `ScoutStatus`, `ScoutCommandMode`, `RepoContextScoutConfig`,
  `RepoIndex`, `ScoutError`, `ScoutTrigger`, `Anchor`, `SupportRoute`, `ChangedAreas`,
  `ShadowRecord`
- `codex-rs/repo-context-scout/src/shadow.rs` -- shadow-mode injection (uses spawn_blocking)

**Tests.** NONE confirmed by grep. Scout rank/index/git modules have no verified test blocks.
Run: `cargo test -p codex-repo-context-scout`

---

### Context-ops shadow/replace

**Functional behavior.** `codex-context-ops-impl` provides file-outline and text-search
helpers for context-building. `codex-replacement-shadow` classifies shell-command outputs
as candidates for compact-digest replacement, and `should_replace_model_output()` gates
whether a given output is replaced by a shorter digest in the prompt.

**NOTE:** `replacement-shadow` is currently dead behind unconsumed feature flags
(`Feature::ContextOpsShadow`, `Feature::ContextOpsReplace`). The classification functions
have no callers in production code as of this writing.

**Implementation.**
- `codex-rs/context-ops-impl/src/lib.rs` -- `file_outline()`, `search_text()`, `ContextOpsError`
- `codex-rs/replacement-shadow/src/lib.rs` -- `ReplacementCandidate` (enum),
  `classify_shell_replacement()`, `classify_promoted_replacement()`,
  `should_replace_model_output()`

**Tests.**
- `context-ops-impl`: NONE confirmed.
- `replacement-shadow`: YES -- 6 tests (token estimates, replacement policy, intent/metadata).
Run: `cargo test -p codex-replacement-shadow`

---

### Memory/MCP cache-status APIs

**Functional behavior.** Two experimental app-server v2 endpoints:
- `memory/status` -- returns `MemoryStatusResponse` with running memory-job list
  (`MemoryJobStatus` array), enabling clients to see whether background memory writes are
  pending.
- `mcp/cache/status` -- returns the Codex Apps MCP tools cache path, hit/miss/invalid
  state, schema version, file metadata, and the `deferred_mcp_loading_enabled` flag.

**Runtime behavior.** Both are gated behind `#[experimental(...)]` and served by the
app-server request-processor layer. `mcp/cache/status` reads config feature flags to
populate `deferred_mcp_loading_enabled`.

**Implementation.**
- `codex-rs/app-server-protocol/src/protocol/common/client_requests.rs:513,910` --
  `MemoryStatus` and `McpCacheStatus` RPC definitions
- `codex-rs/app-server-protocol/src/protocol/v2/thread.rs:917,934` -- `MemoryStatusResponse`,
  `MemoryJobStatus` types
- `codex-rs/app-server/src/request_processors/mcp_processor.rs:375` -- populates
  `deferred_mcp_loading_enabled`
- `codex-rs/app-server/tests/suite/v2/mcp_cache_status.rs` -- integration test

**Tests.** YES -- `codex-rs/app-server/tests/suite/v2/mcp_cache_status.rs:32,63` asserts
`deferred_mcp_loading_enabled` and feature-flag behavior.
Run: `cargo test -p codex-app-server mcp_cache_status`

---

### Agent-policy spawn control

**Functional behavior.** The `codex-agent-policy` crate centralizes spawn-gate logic for
multi-agent v2: `evaluate_spawn_policy()` gates whether a child agent may be spawned
(depth, loop, budget checks). `auto_loop_should_plan_first()` decides whether an auto-loop
submission should route through a planning step before direct execution.
`is_continuation_message()` detects follow-on messages in auto-loop.

**Runtime behavior.** Called from the multi-agent v2 spawn handler and the auto-loop
submission path. `AgentRoiEstimate` carries per-agent expected-value estimates for
delegation routing. `MultiAgentV2SpawnLineage` tracks depth/ancestry for policy checks.

**Implementation.**
- `codex-rs/agent-policy/src/lib.rs` -- `MultiAgentV2SpawnParent`, `MultiAgentV2SpawnLineage`,
  `AutoLoopSubmissionContext`, `AgentRoiEstimate`, `SpawnPolicyRejection`, `SpawnPolicyInput`,
  `evaluate_spawn_policy()`, `auto_loop_should_plan_first()`,
  `auto_loop_plan_first_message()`, `auto_loop_request_user_input_answers()`,
  `is_continuation_message()`
- `codex-rs/agent-policy/src/plan_prompt.rs` -- plan-first system prompt

**Tests.** NONE confirmed via grep of `codex-rs/agent-policy/src/lib.rs`.
Run: `cargo test -p codex-agent-policy`

---

### Context-pack / entrypoint hints

**Functional behavior.** On fresh turns and re-routing triggers, Codex prepends a compact
project-context block to the user message containing ranked entry-point file hints and
scout-based graphify results, so the model starts with structural awareness rather than
blind exploration. `has_context_pack()` guards injection so it never fires twice.

**Runtime behavior.** `render_graphify_scout_pack(request)` builds the context block from
graph/index data; `prepend_context_pack_to_message(msg, request)` prepends it when a scout
is available. `render_entrypoint_hint(project_root, path_budget)` builds a concise
top-file list. `is_explicit_repo_routing_prompt()` detects prompts that already carry full
routing context and skips re-injection.

**Implementation.**
- `codex-rs/context-pack/src/lib.rs` -- `ContextPackRequest`, `render_graphify_scout_pack()`,
  `prepend_context_pack_to_message()`, `render_entrypoint_hint()`, `has_context_pack()`,
  `has_context_pack_or_scout()`, `has_scout_context()`, `is_explicit_repo_routing_prompt()`

**Tests.** YES -- 10+ `#[test]` blocks in `codex-rs/context-pack/src/lib.rs`.
Run: `cargo test -p codex-context-pack`

---

### Workflow batch

**Functional behavior.** A `workflow_batch` tool allows a model or orchestrator to submit a
structured list of work steps as a single tool call; the steps execute sequentially in a
controlled sub-session with per-step tracking and a typed summary returned on completion.
This enables high-throughput mechanical work without repeated round-trips.

**Runtime behavior.** `run_workflow_with_options(spec, options)` executes steps, records a
`StepRecord` per step, and returns a `WorkflowSummary`. `WorkflowOptions::context_tool(root)`
scopes the run to a project root. `WorkflowOptions::unrestricted()` allows broad access.

**Implementation.**
- `codex-rs/workflow-batch/src/lib.rs` -- `StepRecord`, `WorkflowSummary`, `WorkflowOptions`,
  `run_workflow()`, `run_workflow_with_options()`, `run_workflow_value_with_options()`

**Tests.** YES -- 9+ `#[test]` blocks in `codex-rs/workflow-batch/src/lib.rs`.
Run: `cargo test -p codex-workflow-batch`

---

### Reasoning logic (RustBaselineReasoner / SwiplReasoner)

**Functional behavior.** The first-moves and agent-policy subsystems can optionally use a
Prolog-based reasoner (`SwiplReasoner`) for richer exec-policy evaluation, falling back
to a fast Rust baseline (`RustBaselineReasoner`) when SWI-Prolog is unavailable.
`ReasonerAvailability` is checked at session startup.

**Runtime behavior.** `ExecPolicyCase` carries path aliases, host-executable facts, and
prefix rule facts; `ExecPolicyOutcome` carries the result. `ToolSuggestionCase` /
`ToolSuggestionProbabilities` model the tool-choice scoring. The active reasoner is
selected once and held for the session.

**Implementation.**
- `codex-rs/reasoning-logic/src/lib.rs` -- `ReasonerAvailability`, `RustBaselineReasoner`,
  `SwiplReasoner`, `ExecPolicyCase`, `ExecPolicyOutcome`, `ToolSuggestionCase`,
  `ToolSuggestionProbabilities`, `PrefixRuleFact`, `PathAliasFact`, `HostExecutableFact`

**Tests.** NONE confirmed by grep of `reasoning-logic/src/lib.rs`.
Run: `cargo test -p codex-reasoning-logic`

---

### Problem memory context (memories/context)

**Functional behavior.** At session start Codex can inject a compact problem-memory block
summarizing known project issues, unresolved hypotheses, or deferred work items retrieved
from the project's Wizard memory store. This reduces redundant re-exploration of already-
known issues.

**Runtime behavior.** `ProjectProblemMemoryContextRequest` is the typed request entry point;
the crate resolves and formats stored memory items into a context fragment ready for prompt
injection.

**Implementation.**
- `codex-rs/memories/context/src/lib.rs` -- `ProjectProblemMemoryContextRequest` (pub struct)

**Tests.** NONE confirmed by grep of `memories/context/src/lib.rs`.
Run: `cargo test -p codex-memories-context`
