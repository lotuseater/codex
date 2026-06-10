# Fork Features Review — claude-automation-toolkit

**Date:** 2026-06-10
**Scope:** Full review of fork-specific features, diffed against `upstream/main = 51b3cd51f6` (just merged). Fork delta ≈ 2,486 files / +321k −162k lines. Eleven review slices, each backed by a detailed findings file in `.codex/tmp/fork-review/*-findings.md`.

---

## 1. Executive summary

**Overall health: needs-work across the board — architecture is good, operations are leaking.** Of 11 feature families, 10 land at *needs-work* and 1 (operation-cache) at *at-risk*. None is *healthy*, but none is badly designed either: the dominant pattern (fork-only owner crates + thin core adapters + extraction shims) is sound, live, and demonstrably cuts merge surface (e.g. `session/mod.rs` delta shrunk ~3.2k lines; blackboard touches core in ~20 lines; DAB and session-limit-footer are model citizens). The problems are operational: silently severed wiring, a dead test safety net, and unfinished refactors.

### The five themes that matter most

1. **Silent merge severance is the fork's #1 failure mode — it has already happened at least five times.** The May 15 merge (`a41364f808`) wholesale dropped the operation-cache integration from `tools/registry.rs` (the feature has been dead ~4 weeks); a May/June merge dropped the first-moves hit-recording hook (the lane's entire learning loop is inert); a merge rebuilt `plan.rs` on upstream and dropped the plan self-review checkpoint; the `repo_context_scout` tool handler lost its `mod` declaration; and the tui-render port silently dropped upstream's key-value/hyperlink table rendering (~611 lines). Every loss was an inline fork edit in an upstream-hot file, masked by a vacuously-green test or a release-only build gate. The cure is mechanical: ≤5-line seams instead of inline blocks, plus **wiring-guard tests** that turn the next silent loss into a red build.

2. **The fork currently has no working test safety net.** ~1,918 test-compile errors (`cfg(test)` / `--tests` debt) were deferred as "Phase E" after the 2026-06-04 merge and never repaired. The 60k-line relocated test estate (core-test-suites) does not compile; tui-render's 4.5k-line test corpus is unrunnable (missing dev-deps); app-server-client's bootstrap tests use a renamed struct field; a TUI insta snapshot is missing. This debt is *why* theme 1 keeps recurring — losses cannot fail loudly when the tests cannot even compile.

3. **Unfinished refactors and dead code add carriers instead of removing them.** Write-only `ForkFeaturesState` bundle (mirrored at ~8 constructor sites, never read); compaction domain types triplicated across three crates with a no-op double-mapping adapter; four speculative "port" API crates with zero consumers; ~2.7k-LOC replacement-shadow crate fully dead behind unconsumed feature flags; three near-identical dead duplicate files (agent_tool.rs 952 lines, desktop_automation.rs 243 lines, cognos_ops.rs); a dead `rmcp-conversions` feature gate; a fork-resurrected `Op::UserTurn` protocol variant that upstream has deleted. Each is either a merge-conflict magnet, a drift trap, or both.

4. **Extraction redirects merge pressure rather than eliminating it — and the redirect only pays off if the port pass is scripted.** Where the fork relocated upstream-hot code (tui-render ~24.7k lines, analytics-appserver, thread-store, the protocol/test-suite splits, the test estate), every upstream commit to the original paths becomes a manual cross-crate port; the markdown_render feature gap proves features are already being dropped in that port. The fixes are known: keep relocated files 1:1 with upstream shape, script rename-aware diff/port passes, bank rename maps in the merge playbook, and upstream the pure code-motion splits (~150 conflicts/6 weeks would drop to zero if accepted).

5. **Blocking I/O on async hot paths is a recurring pattern.** Context-pack render (full repo WalkDir + up to 256×32KB file reads, every fresh turn), first-moves `predict()` (2000-file scan, default-on), blackboard git/fs I/O inside `context_for_turn`, `GitReviewAnchor` running ~5 git subprocesses on the TUI thread, task-memory doing O(history) JSON serialization on *every* sampling request, and turn-diff shelling `git hash-object` per file. Each has a proven in-repo fix pattern (`spawn_blocking` — the scout shadow already does it correctly) — these are S/M-effort wins.

Also notable: `docs/fork-feature-inventory.md` (2026-05-15) is stale in **every single slice** — it predates tui-render, analytics-appserver, the thread-store relocation, the test relocation, and the merge-automation suite, and still names the old branch. This is operational, not cosmetic: `partition-conflict-slices.ps1 -EmitBriefs` parses it to compute "fork features at risk" for merge-resolver briefs, and the merge-metrics loop (docs/merge-metrics.csv) was never closed for the last two merges.

## 2. State of each feature

| # | Feature family | State | One-line verdict |
|---|----------------|-------|------------------|
| 1 | Context budget & token-saving routing | needs-work | Complete, wired, benchmark-backed; held back by a write-only ForkFeaturesState bundle, duplicated compaction domain types, blocking context-pack I/O, and a fork-resurrected `Op::UserTurn` upstream deleted. |
| 2 | Operation cache | **at-risk** | Owner crate intact, but the entire core runtime integration was silently dropped by the May 15 merge — dead code for ~4 weeks behind a vacuously-green test script. |
| 3 | TUI fork UX (footer, plan-mode, multi-agent display) | needs-work | Fully wired and mostly well-isolated; ships one guaranteed-red test (missing snapshot), a stale verify script, placeholder label noise, and an un-coalesced sub-agent delta flood. |
| 4 | tui-render extraction | needs-work | Architecturally sound ~24.7k-line extraction, but upstream's key-value/hyperlink table features were silently dropped in the port, the 4.5k-line test corpus can't compile, and 43 stale snapshots linger. |
| 5 | Self-review & task-memory | needs-work | Well-factored and live, but the plan-tool checkpoint was merge-dropped (dead exports remain) and pre-sampling injection does O(history) JSON serialization per model request. |
| 6 | Multi-agent v2 & blackboard | needs-work | Feature-complete with exemplary owner crates and sibling modules, undermined by a 950-line duplicated tool-spec file, zero behavioral tests on v2 lifecycle handlers, and inline fork edits in upstream-hot handlers. |
| 7 | Experimental lanes (first-moves, scout, replacement-shadow…) | needs-work | Clean trait-seam architecture, but the first-moves learning loop is merge-severed, replacement-shadow (~2.7k LOC) and the scout tool handler are fully dead, and the default-on predictor blocks the async executor. |
| 8 | Desktop Automation Bridge (DAB) | needs-work | Complete, live, well-layered owner crate; two concrete S-effort safety bugs (timed-out bridge child never killed; missing coordinates click at (0,0)) plus a dead duplicate spec lane. |
| 9 | Owner crates & config family | needs-work | The extraction strategy works and is mostly live, but turn-diff drifted functionally behind upstream's multi-environment rewrite, and 4 speculative port crates are dead weight. |
| 10 | Server-side (analytics, app-server splits, protocol) | needs-work | The strongest architecture in the fork (real DIP inversion, clean splits), but carries a non-compiling test module, a silent analytics-drop sync hazard, a dead feature gate, and the heaviest recurring merge pressure after codex-core. |
| 11 | Build/test automation & relocated test estate | needs-work | Merge automation is mature and battle-tested; the relocated test estate is transitional and broken (~1,918 deferred test-compile errors = no safety net), and the metrics/taxonomy loop is not closed. |

## 3. Per-feature reviews

Condensed from the per-slice findings files in `.codex/tmp/fork-review/`. Effort tags: **S** ≤ half a day, **M** = 1–3 days, **L** = a wave.

### 3.1 Context budget & token-saving routing — needs-work

*Findings: `.codex/tmp/fork-review/context-budget-findings.md`*

**Purpose.** `ContextBudgetMode` (Standard|Slow, default **Slow**) makes the fork token-frugal end-to-end: tightened auto-compact limits, clamped tool-output truncation, restricted first-moves injection; plus the `context-pack` crate (Graphify/Aider-style repo map rendered into fresh-turn prompts) and the fork-only post-turn semantic compaction pipeline (`core/src/session/context_budget_adapter.rs`).

**State.** The routing is complete, wired (config → SessionConfiguration → TurnContext → compaction decisions, plus per-turn protocol carriage) and benchmark-backed (`docs/token-saving-tools/`). The merge-shielding extractions (`context_budget_adapter.rs`, `session/context_budget.rs`, `config/context_budget.rs`) are the right architecture and demonstrably shrank the `session/mod.rs` delta by ~3.2k lines.

**Key issues.**
- **Blocking repo I/O on the async executor:** `first_moves.rs::context_pack_for_fresh_turn` synchronously runs `render_graphify_scout_pack` (uncapped WalkDir + up to 256 files × 32KB reads ≈ 8MB sync fs) inside an async fn at the start of every fresh turn. The sibling scout-shadow correctly uses `spawn_blocking`; this path does not (`core/src/session/first_moves.rs:102,167`).
- **`ForkFeaturesState` is write-only scaffolding** from a stalled migration: mirrored at ~8 constructor sites (session.rs:406, turn_context.rs:337/718, codex_handle.rs, review.rs, support_session.rs ×3) but never read in production; `ForkFeaturesUpdate` is always `Default` and `apply()` has zero non-test callers. It currently *adds a 4th parallel carrier* instead of replacing the 3 scalar fork fields.
- **Fork keeps the whole `Op::UserTurn` enum variant alive** (`protocol/src/protocol/op.rs:181`) although upstream has **deleted** it — solely so guardian review turns can carry per-turn `context_budget_mode` (`guardian/review_session.rs:729`). Guaranteed recurring conflict in the upstream-hot protocol crate.
- **Triple type duplication:** `context-domain/compaction-policy` (143 lines) mirrors the same enums/structs as `context-reduction` (885 lines), and `core/src/context_reduction_adapter.rs` round-trips every value through a no-op intermediate hop (~120 lines of mapping ceremony touched per reason/mode addition).
- No direct unit tests for the compaction decision orchestration (`post_sampling_compaction_decision`, `maybe_run_post_turn_semantic_compact`); coverage is only indirect via session tests sitting on the ~1,918-error test debt. Minor: `Slow` default silently clamps tool output for anyone who never set the knob; context-pack heuristics are English-only, `#`-comment stripping is undocumented, repo inventory has no .gitignore awareness or file cap.

**Key improvements.**
- **[S]** Wrap the context-pack render in `tokio::task::spawn_blocking` (same pattern as the scout shadow 60 lines below).
- **[M]** Finish OR revert the ForkFeaturesState migration: move readers to the bundle and delete the 3 scalar fields (cuts fork carriers 3→1 in the hottest merge files), or delete the bundle **[S]** to stop paying the mirror tax.
- **[M]** Retire the fork-kept `Op::UserTurn`: carry `context_budget_mode` via the fork's ThreadSettingsOverrides/UserInput extension (only 2 core call sites), then drop the resurrected variant from `protocol/op.rs`.
- **[M]** Collapse the compaction domain-type triplication: have `codex-context-reduction` depend on / re-export `codex-compaction-policy` types; delete the pass-through double-mapping.
- **[S]** Table tests for `post_sampling_compaction_decision` + `auto_compact_token_limit`; context-pack hygiene (file cap, .gitignore, compute prompt terms once, document the Slow default and `#` caveat); refresh the inventory row (still references branch `slow-context-budget-mode`).

### 3.2 Operation cache — AT-RISK (runtime integration dead since 2026-05-15)

*Findings: `.codex/tmp/fork-review/operation-cache-findings.md`*

**Purpose.** Cross-agent tool-result cache: a Python bridge consults the shared SQLite tool-cache (`~/.claude/cache/tool_cache.sqlite`, shared with Claude Code hooks) before dispatch; hits short-circuit, successes are stored back. Companion status APIs: cognos `operation_cache_stats`, app-server v2 `mcpCacheStatus`.

**State.** **P0 regression:** merge `a41364f808` (May 15) silently removed `mod operation_cache;` from `core/src/tools/mod.rs` and the ~50-line lookup/store/MCP-hit wiring from `registry.rs`. `core/src/tools/operation_cache.rs` is an orphaned, uncompiled file; the bridge never fires; the owner crate (417 lines, unit-tested) is healthy but unreachable. Last-good wiring: `git show 98923e8d85:codex-rs/core/src/tools/registry.rs` (lookup ~383–407, store ~610, `operation_cache_cwd` ~707; MCP hit-shaping from `770ea01e7d`).

**Key issues.**
- The regression went unnoticed for ~4 weeks because `scripts/test-operation-cache.ps1`'s rust leg (`cargo test -p codex-core --lib operation_cache`) matches **zero tests** once the module is unwired — vacuous green. The runtime canary script would have caught it but was not run post-merge.
- Status APIs outlive the feature: `operation_cache_stats` and `mcpCacheStatus` report on a cache codex no longer reads or writes; `operation_cache_stats` is also a static env echo that never probes bridge resolution or real sqlite stats.
- Hardcoded user-specific default bridge path (`~/Documents/GitHub/Wizard_Erasmus/.../codex_cache_bridge_cli.py`) baked into `default_bridge_candidates()`.
- Docs stale: `docs/operation-cache-status.md` (May 6) and the inventory row both describe the registry.rs integration as live. Adjacent: `cognos_ops.rs` duplicated verbatim as compiled modules in both `codex-rs/tools` and `tools-domain/tool-registry-api`.

**Key improvements.**
- **[M] (P0)** Re-wire: restore the mod declaration + dispatch integration cherry-picked from `98923e8d85` (incl. MCP `CallToolResult` hit-shaping), reshaped as a **≤5-line seam** (`try_serve` pre-dispatch / `maybe_store` post-dispatch) with all telemetry/trace bookkeeping inside the fork-owned module. Accept via `scripts/test-operation-cache-runtime.ps1`.
- **[S]** Wiring-guard test in core asserting a synthetic cache hit short-circuits dispatch and store fires on success — turns the next silent merge loss into a red test.
- **[S]** Harden `test-operation-cache.ps1` to fail on zero matched tests; add "grep registry.rs for operation_cache" + the runtime canary to the post-merge playbook.
- **[S]** Remove the hardcoded Wizard_Erasmus path (env or `~/.codex/config.toml` only); refresh the two stale docs.
- **[M]** Perf once re-wired: `OnceLock` the config, memoize repo-root, move success gating into `store()`'s signature, consider rusqlite/resident-bridge to drop the 2-python-spawns-per-tool-call pattern.

### 3.3 TUI fork UX — needs-work

*Findings: `.codex/tmp/fork-review/tui-ux-findings.md`*

**Purpose.** Five families: session-limit footer (context % + rate-limit windows in the bottom pane), startup marker ("local build {stamp}"), Plan-mode reasoning defaults (`plan_mode_reasoning_effort` config + scope-prompt UI), history_cell fork deltas (now a 173-line shim over tui-render), and multi-agent display surfaces (compact/restart cells, enriched agent labels, inactive sub-agent activity transcript via fork-only `multi_agents/activity.rs`).

**State.** All five families are complete, wired, and live — no dead code found. `session_limit_footer` and `activity.rs` are exemplary isolation (pure functions, fork-only modules, ~4 one-line call sites in hot files). Post-merge debt: one guaranteed-red test and a stale verify script.

**Key issues.**
- **Missing insta snapshot:** `multi_agents.rs` test `subagent_activity_notification_snapshot_includes_evidence_and_token_updates` asserts snapshot `subagent_activity_notifications`, but only the `indented` .snap exists — test fails on first run.
- `scripts/test-session-limit-footer.ps1` is stale and fails today: it greps for `renders_reset_percentage_without_token_usage` (singular) but the test was renamed to the plural form; its grep-assert design pins implementation details that rot.
- `agent_activity_spans` now always renders `(model ?, effort ?, --% used)` placeholders when runtime details are unknown — visual noise and a behavior regression vs upstream's omit-when-unknown.
- Rate-limit key casing inconsistency: snapshots keyed by raw `limit_id` (rate_limits.rs:178), the codex check is case-insensitive (line 206), but the footer lookup `get("codex")` (status_controls.rs:309/372) is exact-case — a `"Codex"` id silently drops the footer.
- `on_inactive_collab_agent_activity` turns every output delta from inactive sub-agents into its own history cell with no coalescing — chatty sub-agent commands flood the main transcript. Also: plan-mode override injection duplicated in `input_flow.rs:183` and `settings.rs:726`.

**Key improvements.**
- **[S]** Regenerate and commit the missing snapshot; **[S]** fix (or replace with `cargo test -p codex-tui session_limit_footer`) the stale verify script.
- **[S]** Render only known label segments; normalize rate-limit keys to lowercase with a shared `CODEX_LIMIT_ID` const; extract one `apply_plan_mode_reasoning_override` helper.
- **[M]** Coalesce inactive-agent output-delta cells (throttle per item_id or render only on ItemCompleted).
- **[M]** Shrink merge surface: move compact/restart cell builders + `agent_activity_spans` from upstream-hot `multi_agents.rs` (~370-line interleaved delta) into the fork-only `activity.rs` submodule; bank the scripted `tui/src/history_cell → tui-render/src/history_cell` path-rewrite port step in the merge playbook; refresh the inventory.

### 3.4 tui-render extraction — needs-work

*Findings: `.codex/tmp/fork-review/tui-render-findings.md`*

**Purpose.** Fork-only crate (~24.7k lines, 41 .rs files + ~102 snapshots) extracting the pure rendering layer (history cells, markdown/diff rendering, wrapping, highlight, hook_cell, MCP masking) out of `codex-rs/tui`, which keeps 1–5-line shims at the upstream paths. Intent: fork render features live in a crate upstream never touches.

**State.** Fully wired (tui is the sole consumer; no codex-core dependency — good DIP). Most extracted files are near-identical to upstream (10–30-line import-only deltas). But the extraction *redirects* merge pressure into a manual per-merge port, and that port has already dropped features.

**Key issues.**
- **Unported upstream features:** `tui-render/src/markdown_render.rs` lacks ~611 lines upstream has (key-value tables #24636/#24825, column classification/shrink, hyperlink-aware table cells). Upstream's module file `tui/src/markdown_render/table_key_value.rs` sits **orphaned** in the fork tree (no `mod` declaration anywhere) — dead code, feature silently absent from fork builds, and a renamed "fallback" test papers over it.
- **Tests cannot compile:** `history_cell/mod.rs` has `#[cfg(test)]` imports of `codex_config` and `codex_mcp`, neither in tui-render's deps/dev-deps — the entire 4.5k-line test corpus (~59 valid snapshots) is unrunnable. Zero regression protection on exactly the surface that is hand-ported each merge.
- **43 stale snapshots** named `codex_tui__*.snap` (pre-extraction crate name) alongside 37 current `codex_tui_render__*` snaps — ~30% of the 152-file fork delta is dead weight.
- The inventory does not mention tui-render at all (crate extracted 2026-05-16, one day after the inventory's date); merge resolvers lack the "shims must stay shims" rule. Minor SRP smell: non-render concerns (session_state.rs, update_action.rs, version.rs) live in the "render" crate.

**Key improvements.**
- **[M]** Port the missing upstream markdown_render features and delete the orphaned `table_key_value.rs` — restores silently-dropped user-visible upstream behavior.
- **[S]** Add `codex-config` + `codex-mcp` to tui-render dev-dependencies (or refactor the cfg(test) imports) to revive the test + snapshot suite.
- **[S]** Delete the 43 stale snapshots (`cargo insta test --unreferenced=delete` once tests compile).
- **[M]** Script the per-merge port pass: diff `upstream:tui/src/<file>` vs `HEAD:tui-render/src/<file>` for the ~25 extracted paths, emit a port worklist — deltas are mostly mechanical, and dropped features become detectable. (Un-extracting was considered and rejected: the crate boundary is what keeps hook_cell/MCP-masking out of upstream-hot files.)
- **[S]** Refresh the inventory with the crate, shim convention, and the never-resolve-shims-to-full-bodies rule.

### 3.5 Self-review & task-memory — needs-work

*Findings: `.codex/tmp/fork-review/self-review-task-memory-findings.md`*

**Purpose.** `codex-task-memory` builds a token-budgeted `<task_memory>` block (active request + directives + latest plan) injected under token pressure and preserved across compaction; `codex-self-review` provides the git-grounded self-review loop (turn tracker + `GitReviewAnchor` capturing HEAD/dirty baselines, emitting exact `git diff` commands) orchestrated from the TUI.

**State.** Mostly complete and live — pressure injection, compaction preservation, throttle reset, TUI review loop, and git-grounded prompts are wired end to end. One confirmed merge regression: the **plan-tool self-review checkpoint** (commit `38ff5d8528`) is no longer wired — `core/tools/handlers/plan.rs` was rebuilt on upstream during merges and never calls `codex_self_review::plan_tool_response`/`is_plan_review_candidate`; those exports are now dead.

**Key issues.**
- The plan-checkpoint merge drop (above) — the silent-regression class this fork keeps hitting; no characterization test existed to catch it.
- **Per-request overhead:** `maybe_inject_task_memory_for_sampling` (`core/src/session/context_budget.rs:259`) runs `build_task_memory` plus `estimated_prompt_tokens` (serde_json-serializes EVERY history item) on every sampling request, even far below the pressure threshold — gate order is inverted.
- Fragile cross-crate string coupling: prompt-reducer detects self-review prompts by hardcoded prose anchors duplicated from codex-self-review prompt text; a wording tweak silently breaks prompt dedup. No shared constant, no pinning test.
- Compaction wiring copy-pasted across three upstream-hot files (`compact.rs`, `compact_remote.rs`, `compact_remote_v2.rs`) — the same 4-step pattern triplicated; every upstream compaction refactor re-conflicts all three.
- `GitReviewAnchor::capture` runs ~5 git subprocesses + up to 32 file copies synchronously on the TUI thread (construction, cwd change, after each review). Inventory stale (says injection lives in `session/mod.rs`; lists the plan checkpoint as present).

**Key improvements.**
- **[S]** Fix or retire the plan-checkpoint regression (args are already parsed in the handler) + add a plan.rs characterization test so the next merge cannot silently drop it again.
- **[S]** Reorder gates in `maybe_inject_task_memory_for_sampling`: early-return when no task-memory item exists and pressure is below threshold, reusing the session's existing token estimate.
- **[M]** Extract one `apply_to_compaction()` helper in fork-owned `core/src/task_memory.rs`; call from all three compaction paths — three hot conflict sites shrink to one line each.
- **[S]** Export stable prompt-anchor constants from codex-self-review, consume in prompt-reducer, add a cross-crate pinning test; reuse `TASK_MEMORY_*` marker constants in `core/src/context/task_memory.rs`.
- **[M]** Move `GitReviewAnchor::capture` off the TUI thread (async/lazy); add the missing end-to-end pressure-injection test; refresh the inventory row.

### 3.6 Multi-agent v2 & blackboard — needs-work

*Findings: `.codex/tmp/fork-review/multiagent-blackboard-findings.md`*

**Purpose.** (1) Fork lifecycle supervision tools on upstream's multi_agents_v2: `compact_agent`, `close_agent` (distinct from upstream's `interrupt_agent`), `restart_agent`, `resume_agent`, model/effort overrides on `followup_task`, plus 12 `Collab*Begin/End` telemetry event types. (2) `codex-blackboard`: cross-*session* coordination via an ignored `.codex/blackboard.md` (intent/proposal events injected as turn context).

**State.** Complete and wired. Owner crates (codex-blackboard, codex-agent-policy, codex-agent-graph-store) with thin core facades; `compact_local.rs`/`override_local.rs` sibling modules are the **exemplary** merge-pressure pattern to replicate. Blackboard crate quality is good (atomic file locks, size caps, corrupt-line tolerance, 14 tests).

**Key issues.**
- **`codex-rs/tools/src/agent_tool.rs` (952 lines) is a near-identical dead copy** of `tools-domain/tool-registry-api/src/agent_tool.rs` (commit `b79d05a77b` copied instead of moved); core consumes only the registry-api copy — ~1,900 lines of drift-prone duplication incl. tests.
- **Zero behavioral tests** for the fork v2 lifecycle handlers (compact/close/restart/resume) — coverage is tool-name snapshots plus a manual canary script.
- `message_tool.rs` **replaces** upstream `SubAgentActivityEvent(Interacted)` with fork `CollabAgentInteractionBegin/End` inline in an upstream file — behavioral divergence + merge magnet; Collab* events also carry an always-empty placeholder `prompt` field.
- **Blocking I/O inside async on the turn hot path:** `blackboard/src/session.rs` shells out to git (`std::process::Command` in `current_branch`) and does sync fs I/O in `append_event`/`read_events` from async fns called by `context_for_turn`; plus a `join_repo_for_path` double-lock TOCTOU.
- Fork API-migration churn (`codex_tools::ToolSpec` → `codex_tool_registry_api`, async_trait removal, `&dyn ToolOutputPayload`) spread across all 10 upstream multi_agents_v2 handler files inflates every future merge in a fast-moving upstream area (3 upstream PRs since May, incl. the close→interrupt rename).

**Key improvements.**
- **[S]** Delete `tools/src/agent_tool.rs` + tests; re-export the spec builders from `codex_tool_registry_api`.
- **[M]** Behavioral tests for the lifecycle handlers using the existing multi_agents_tests harness (compact status validation + root rejection; restart override/followup; resume 3-stage target resolution + depth limit; close persisted-edge paths).
- **[M]** Extract fork additions in `message_tool.rs`/`spawn.rs` into a fork-local sibling module (`multi_agents_v2/collab_events.rs`, mirroring the compact_local.rs pattern); consider dual-emitting `SubAgentActivityEvent` for upstream consumers.
- **[S]** Blackboard async hygiene: `spawn_blocking`/`tokio::fs` on the context_for_turn path; single locked block in `join_repo_for_path`; refresh branch on heartbeat.
- **[S]** Event fidelity: populate or drop the empty `prompt`; keep `Option<ReasoningEffort>` instead of `unwrap_or_default`; make eager `.or(persisted_agent_metadata(...).await)` lookups lazy.

### 3.7 Experimental lanes — needs-work

*Findings: `.codex/tmp/fork-review/experimental-lanes-findings.md`*

**Purpose.** Context-economy lanes: **first-moves** (predicts files/searches to open first, injects `<first_moves>` block, learns from tool-use hits in per-repo sqlite), **repo-context-scout** (cached repo index + ranked scout packet, Off/Shadow/Tool modes), **replacement-shadow** (compact digests of shell-command output), **reasoning-logic** (Prolog comparison lab, dormant by design), **context-ops-impl** (rg/outline helpers, live), **memories/context** (project/problem routing index + recall block).

**State.** Injection lanes are alive; the learning and replacement halves are dead:
- **first-moves learning loop SEVERED (merge regression):** `spawn_record_tool_use_hit` has zero callers — the `registry.rs` hook from `ed537b8e19` was dropped in a May/June merge, so hit-count learning, the "confirmed local hit history" scoring boost, and `first_moves_stats` hit-rate are all inert.
- **replacement-shadow (~2.7k LOC) fully dead:** `classify*/should_replace*` have no consumers; `Feature::ContextOpsShadow/ContextOpsReplace` exist but are consulted nowhere; the canary script probes flags that gate nothing (false-green).
- **`repo_context_scout` tool handler orphaned:** not declared in `handlers/mod.rs` (not compiled), implements a pre-merge trait shape that no longer exists — yet config still accepts `repo_context_scout.mode="tool"`.

**Key issues (beyond the dead wiring).**
- Blocking I/O on the default-on async fresh-turn path: `predict()` runs a sync WalkDir scan of up to 2,000 files, sync prewarm reads, sync JSONL index reads, and sync shadow-log appends inline (`session/turn.rs:369`), unlike the scout shadow which correctly uses `spawn_blocking`.
- Overfit, compiled-in heuristics: `predict.rs` hardcodes this user's vocabulary (`dab_`, `FastRelease`, `build-local-codex`, first-moves itself) as intents; memory-index parsing and scope matching are duplicated sync (first-moves) vs async (memories/context); scout's rank/index/git parsing has zero tests; sqlite writes lack transactions; shadow JSONL grows unbounded.

**Key improvements.**
- **[S]** Re-wire the tool-use-hit hook (restore the `ed537b8e19` hunk — ideally behind a fork-owned post-tool-use observer seam instead of inline registry.rs code).
- **[S]** Decide replacement-shadow: wire the interception behind the existing flags, or remove the crate from workspace members and delete the dead flags + canary script.
- **[S]** Fix or remove the orphaned scout tool handler (port to ToolExecutor + register, or drop `Tool` from config).
- **[M]** Move `predict()` scan/prewarm/shadow-append into `spawn_blocking`.
- **[M]** Add a **fork-wiring smoke test** asserting each lane's call chain is intact (hit-hook caller exists, scout handler registered, feature flags consumed) — this slice has already lost wiring twice. **[L]** Externalize the overfit intent heuristics into per-repo data.

### 3.8 Desktop Automation Bridge (DAB) — needs-work

*Findings: `.codex/tmp/fork-review/desktop-automation-findings.md`*

**Purpose.** 18 native Windows GUI tools (`dab_*`: find/screenshot/OCR/click/drag/send-keys/…) implemented in the fork-only `codex-rs/desktop-automation` owner crate, which writes an embedded 1,082-line PowerShell bridge (Win32 P/Invoke + UIAutomation) to temp and spawns `powershell.exe` per call; a prompt-side classifier (EN+UK needles) injects a `<desktop_automation>` context block.

**State.** Complete, live, well-isolated (owner crate + 120-line handler + 28-line session shim + ~13 lines in turn.rs; no core dependency), with decent tests including a real Windows integration test. This slice's inventory rows are the only ones found accurate.

**Key issues.**
- **Timeout never kills the spawned bridge child** (`windows.rs:43-81` lacks `kill_on_drop`/explicit kill): a timed-out mutating tool (click/sendkeys/drag) can keep injecting input after Codex reports failure, and leaks a process per timeout.
- **`dab_click`/`dab_bg_click` with missing x/y click at (0,0):** the schema has no required fields and the ps1 skips `Get-MissingNumericFields` for click (only `dab_drag` got validation) — `[int]$null = 0`.
- `codex-rs/tools/src/desktop_automation.rs` is a byte-identical **dead duplicate** of the registry-api copy, consumed only by `build_tool_registry_plan` which nothing calls — silent-divergence trap for any spec fix.
- `is_mutating_tool` is exported dead code; mutating DAB tools bypass per-invocation approval entirely (only gates: registration-time config + external hooks).
- Every call rewrites the ps1 and recompiles the embedded C# via `Add-Type` in a fresh powershell.exe (~1–3s fixed overhead in inherently multi-call flows); screenshot embedding has no size/downscale guard.

**Key improvements.**
- **[S]** `.kill_on_drop(true)` (or explicit kill on timeout) — highest safety payoff per line.
- **[S]** Validate click/scroll/bg_click coordinates in the ps1 and mark x/y required in the schemas.
- **[S]** Delete the dead spec duplicate (and decide the fate of the unused `build_tool_registry_plan` lane); share `DAB_*_TOOL` name constants instead of duplicated string literals.
- **[M]** Wire `is_mutating_tool` into the approval flow (or delete it and document registration-time gating as the deliberate model).
- **[L]** Persistent stdio JSON bridge (spawn once per session, health-check/restart): ~10x latency reduction + real cancellation; add a screenshot size guard while in there.

### 3.9 Owner crates & config family — needs-work

*Findings: `.codex/tmp/fork-review/owner-crates-config-findings.md`*

**Purpose.** The fork's merge-pressure strategy itself: extract logic into fork-only owner crates (turn-diff, cognos-ops, tools-domain, context-domain, config-types, permission-types, git-types) behind thin core adapters and 1-line facades; relocate/split thread-store; config-edit moved into codex-config; core-plugins DIP fix + test split; windows-sandbox dep dedupe.

**State.** The architecture is sound, live, and demonstrably cuts merge surface (fork-only crates carry near-zero direct conflict). Execution gaps: functional drift, dead speculative crates, and unscripted recurring conflicts.

**Key issues.**
- **Functional drift in turn-diff:** upstream rewrote `core/src/turn_diff_tracker.rs` (#26433) with `TrackedPath{environment_id,path}` multi-environment tracking; the fork's `codex-turn-diff` keeps the old single-env uuid design and the adapter in `core/src/tools/events.rs` explicitly **discards** `environment_id` — multi-env turns conflate baselines. The fork also still shells `git hash-object` per file and computes the full diff twice per patch event.
- **Four dead crates with zero source consumers:** `tools-domain/tool-handler-api`, `context-domain/history-api`, `thread/thread-handle-api`, `thread/thread-projection-api` — speculative DIP seams never wired; build time, lockfile churn, reviewer confusion.
- The `thread/` family is a relocation+split of upstream's actively-developed `thread-store` (lib.rs similarity already ~58%); every upstream change needs rename-aware porting and the mapping is recorded nowhere.
- Fork deleted upstream files it still must merge against (`config/src/thread_config/remote.rs` −523 + protos) — recurring modify/delete conflicts; stale empty `proto/` dir on disk.
- Test asymmetry: codex-turn-diff has only 4 tests for a diff engine that replaces upstream's (no rename-chain/mode-change/binary/symlink cases), vs 50 for config edit; inventory predates the thread relocation and dead-crate additions.

**Key improvements.**
- **[M]** Port upstream #26433 multi-environment tracking into codex-turn-diff (env_id+path, in-memory baselines, drop the git subprocess) and forward `environment_id` through the adapter — fixes a correctness regression.
- **[S]** Delete or explicitly document-and-gate the 4 dead port crates (if they are Phase-5 seams, add a README stating the target wave).
- **[S]** Merge-time drift-check script diffing upstream's `protocol/src/config_types.rs` and `network-proxy/src/config.rs` against the owner crates; hang it off `check-cargo-dependency-boundaries.ps1`.
- **[M]** Expand codex-turn-diff test coverage (rename chains, mode changes, binary, symlink, multi-env case).
- **[S]** Record the thread-store rename map + facade/deleted-file lists in the merge playbook; housekeeping (stale proto/ dir, `track_delta`'s constant bool, inventory refresh).

### 3.10 Server-side (analytics, app-server, protocol) — needs-work

*Findings: `.codex/tmp/fork-review/server-side-findings.md`*

**Purpose.** (1) `codex-analytics-appserver`: DIP split of upstream analytics — protocol-free `AnalyticsReducer` trait stays low, protocol-aware reducer/events move up, injected one-line at the composition root (resolves the memory-noted core→analytics→app-server-protocol coupling). (2) app-server-client SRP split + `legacy_core` shim (TUI dropped its direct codex-core dep; 146 greppable call sites). (3) Pure code-motion splits of protocol God files (common.rs, thread_history.rs, export.rs, v2/tests.rs, turn_start.rs) into re-export hubs. (4) Small semantic protocol additions (`context_budget_mode` turn param, `MemoryStatusResponse`, fork `SubAgentSource`). (5) ext/goal adaptation to the fork's extension-api.

**State.** Complete, wired, and architecturally the strongest part of the fork (real DIP inversion, validated boundary conversions, zero unwraps in prod paths). Debt is operational.

**Key issues.**
- **Bug (test debt instance):** `app-server-client/src/bootstrap.rs`'s own test module uses stale field `cloud_requirements: CloudRequirementsLoader` while the struct has `cloud_config_bundle: CloudConfigBundleLoader` — the crate's tests do not compile; release-only build gates hide it.
- **Sync hazard (silent analytics drop):** `analytics-appserver/src/reducer.rs::is_unconditional_custom()` hardcodes 7 `CustomAnalyticsFact` variants that must mirror codex-analytics's exhaustive match; the upper crate's catch-all means a new upstream unconditional fact compiles fine but is **silently dropped** in app-server. No test characterizes the sync.
- **Dead feature gate:** `rmcp-conversions` is enabled by no workspace crate — the 5 cfg-gated conversion sites and the mcp_elicitation tests never compile or run in any default build.
- The analytics crate relocation defeats git rename tracking (upstream events.rs 27 + reducer.rs 24 commits/6wk need manual cross-crate porting); the protocol/test splits conflict with upstream monoliths (common.rs 52, v2/tests.rs 46, turn_start.rs 29 commits/6wk).
- Inventory lists none of this slice's major deltas; `request()`/`request_typed()` bodies copy-pasted between client and handle in app-server-client.

**Key improvements.**
- **[S]** Fix the bootstrap.rs test fields; add the crate to the targeted test-debt repair list.
- **[S]** Eliminate the sync hazard: delegate ALL Custom facts not handled by connection-gated arms to the inner `CustomFactReducer` (behavior-identical), delete `is_unconditional_custom`, add a one-assert characterization test.
- **[M]** **Upstream the pure code-motion splits** as 2–3 mechanical PRs — the largest single merge-pressure reduction available (~150 conflicts/6wk drop to zero if accepted).
- **[S]** Refresh the inventory with rows for analytics-appserver, the app-server-client split + legacy_core, the protocol splits, and the fork protocol additions.
- **[M]** Move fork-only protocol additions out of upstream-hot `v2/thread.rs`/`v2/turn.rs` into a dedicated fork-ext module; wire CI to build with `--features rmcp-conversions` or delete the gated conversions. Note: do **not** re-split reducer.rs/events.rs — their 1:1 shape with upstream is what keeps porting cheap.

### 3.11 Build/test automation & relocated test estate — needs-work

*Findings: `.codex/tmp/fork-review/build-test-automation-findings.md`*

**Purpose.** (1) Local Windows build/release workflow (`build-local-codex.ps1`, 1,790 lines / 11 modes, wrapper deploy, watchers, smoke test). (2) Merge-automation suite (preflight rehearsal → hotspot map → conflict-slice partition with resolver briefs → residue/verify gates → buildfix triage → metrics). (3) Relocated test infrastructure: `core-test-suites/` (10 topic crates) + `test-support/` behind `#[path]` shims and a façade in `core/tests/common/lib.rs`.

**State.** Build workflow and merge suite are mature, battle-tested across two big merges, read-only by design, with consistent exit-code contracts — above-average PowerShell. The test relocation is **transitional and broken**: ~1,918 test-compile errors explicitly deferred as Phase E and never executed — the fork currently runs without a test safety net.

**Key issues.**
- The ~1,918-error debt makes the entire 60k-line relocated test estate dead weight (and is the root enabler of the silent-severance theme).
- **Duplicated AND divergent test sources both still wired:** `core/tests/suite/tool_harness.rs` vs the core-test-suites copy differ by 558 lines (tools.rs 719, search_tool.rs 285); plus 9 orphaned never-compiled files in `core/tests/suite` (hidden by `autotests=false`).
- Merge-area taxonomy hand-duplicated in ≥4 scripts (partition/preflight/hotspot-map/adapter-gaps) and stale: no slice for core-test-suites/test-support/scripts — the biggest next-merge conflict family lands in one catch-all "other" slice.
- Metrics/inventory loop not closed: merge-metrics.csv has one unfinalized row and NO row for the just-completed 51b3cd51f6 merge; the inventory omits merge-automation and the test relocation — and `-EmitBriefs` parses it, degrading resolver briefs.
- Zero automated tests for ~16.7k lines of fork PowerShell (the merge gates themselves are untested); `build-local-codex.ps1` is a 47-function God-script with a dead DevRelease mode; the justfile whole-file `--release`+`"$@"` rewrite breaks just-shell.py's portable `{args}` token and guarantees a conflict every merge (plus a bazel-lock-check inline regression).

**Key improvements.**
- **[L]** Execute the deferred Phase-E test-repair wave (per-crate `cargo check --tests -p <crate>`, fresh workers per the post-merge playbook) — restores the fork's only regression safety net.
- **[M]** Single-ownership cleanup: reconcile the 3 divergent duplicates, delete the 9 orphans + the identical apply_patch_harness copy, point all shims at core-test-suites.
- **[S]** Extract the merge-area taxonomy into one shared artifact (`scripts/merge-area-taxonomy.psd1` dot-sourced by all 4 scripts); add core-test-suites/test-support/scripts/.codex slices.
- **[S]** Close the metrics/inventory loop (finalize the 2026-06-05 row, add the 51b3cd51f6 row, refresh the inventory).
- **[M]** justfile de-conflict (single profile variable, restore `{args}` tokens and upstream's bazel-lock-check script call); drop dead DevRelease; add fixture-log Pester tests for the 4 gate scripts.

## 4. Cross-cutting themes

### T1. Silent merge severance of fork wiring (confirmed 5×, suspected more)

Every realized loss followed the same shape: **a small inline fork edit in an upstream-hot file, resolved away during a merge, with no test that could fail.**

| Lost wiring | Hot file | Dropped by | Masked by |
|---|---|---|---|
| Operation-cache lookup/store/MCP-hit (~50 lines) | `core/src/tools/registry.rs` | merge `a41364f808` (May 15) | test script matching zero tests = vacuous green |
| First-moves hit-recording hook | `core/src/tools/registry.rs` | a May/June merge | no caller-exists assertion; stats run on frozen data |
| Plan self-review checkpoint | `core/tools/handlers/plan.rs` | merge rebuilt plan.rs on upstream | no characterization test; exports went dead silently |
| `repo_context_scout` tool handler `mod` decl | `core/src/tools/handlers/mod.rs` | pre-dates latest merge | file orphaned on disk; config still promises the tool |
| markdown_render key-value/hyperlink tables (~611 lines) | tui→tui-render hand-port | port pass during a merge | renamed "fallback" test papers over the gap |

**Standing fixes (apply everywhere):** (a) reduce every fork touchpoint in a hot file to a ≤5-line seam delegating to a fork-owned module (the pattern `context_budget_adapter.rs`, `compact_local.rs`, and the DAB session shim already prove); (b) add a **fork-wiring smoke test** per lane asserting the call chain is intact; (c) harden canary scripts to fail on zero matched tests; (d) add per-lane grep checks to the post-merge build-fix playbook.

### T2. Test debt: ~1,918 cfg(test)/--tests errors (deferred Phase E) = no safety net

The debt is not abstract — this review hit concrete instances in five slices: tui-render's whole 4.5k-line corpus (missing `codex-config`/`codex-mcp` dev-deps), app-server-client bootstrap.rs (stale field name), the missing `subagent_activity_notifications` snapshot, divergent duplicated test sources in the core test estate, and `cargo check --release` silently skipping `#[cfg(test)]` everywhere (the documented false-green gotcha). Until Phase E runs, **no fork feature has compiling regression coverage**, which is precisely what enables T1. Phase E is the single highest-leverage L-effort item in this review.

### T3. Unfinished refactors & dead code

Two flavors, both costly at merge time:
- **Stalled migrations that added carriers instead of removing them:** write-only `ForkFeaturesState` (4th parallel carrier at ~8 constructor sites); compaction-policy/context-reduction type triplication + no-op adapter hop; `legacy_core` (deliberate and documented — fine, but the call-site migration should finish someday).
- **Dead weight:** 4 zero-consumer port crates (tool-handler-api, history-api, thread-handle-api, thread-projection-api); replacement-shadow (~2.7k LOC) + 2 unconsumed feature flags + false-green canary; dead duplicates (`agent_tool.rs` 952L, `desktop_automation.rs` 243L, `cognos_ops.rs` ×2); the unused `build_tool_registry_plan` lane; dead `rmcp-conversions` gate; dead `is_mutating_tool`; orphaned scout handler; 43 stale snapshots; 9 orphaned test files. Dead code is not neutral here — dead flags and duplicate specs invite wrong conflict resolutions and silent divergence.

### T4. Relocations need scripted port passes (the "redirect tax")

Fork-only owner crates genuinely carry near-zero conflict surface — the strategy works. But where the fork *relocated* upstream-hot code, each upstream commit becomes a manual cross-crate port: tui-render (47 upstream commits on extracted paths since March), analytics-appserver (51 commits/6wk on reducer+events), thread-store relocation (lib.rs similarity down to ~58%), protocol re-export hubs (common.rs 52, v2/tests.rs 46, turn_start.rs 29 commits/6wk), the test estate (17 modify/delete conflicts in one merge), and 1-line facades (config_types, network-proxy config, json_schema). None of these port passes is scripted, and the inventory/playbook records none of the rename maps. Three reducers: **script the rename-aware diff/port per surface; keep relocated files 1:1 with upstream's internal shape (do not re-split them); upstream the pure code-motion splits.**

### T5. Blocking I/O on async hot paths

Six independent findings of sync I/O on the tokio executor or UI thread: context-pack render (every fresh turn), first-moves `predict()` (default-on, every fresh turn), blackboard git/fs in `context_for_turn`, `GitReviewAnchor::capture` on the TUI thread, task-memory O(history) serde_json per sampling request (gate-order inversion), and turn-diff's per-file `git hash-object` + double diff. The repo already contains the correct pattern (`spawn_blocking` in the scout shadow); applying it is mostly S-effort. These compound into first-turn latency on large repos — directly against this fork's token/latency-frugality purpose.

### T6. Stale meta-artifacts break the automation loop

`docs/fork-feature-inventory.md` (2026-05-15) is stale for all 11 slices and feeds `-EmitBriefs` resolver briefs; `docs/merge-metrics.csv` was never finalized for either 2026-06 merge; `docs/operation-cache-status.md` describes a dead integration; the merge-area taxonomy is hand-duplicated in 4 scripts with no slice for the fork's own biggest conflict family. The merge automation is only as good as the data it reads — closing this loop is cheap (S) and improves every future merge.

### T7. Schema/protocol regen friction (small but persistent)

Fork additions sit inline in upstream-hot protocol files: `Op::UserTurn` (resurrected after upstream deletion), `context_budget_mode` on v2 turn params, `MemoryStatusResponse`/`MemoryJobStatus` in v2/thread.rs, `SubAgentSource`, and 12 Collab* event types rippling through event_msg.rs → app-server-protocol event_mapping/thread_history → TS exports → generated schema JSONs. Every upstream protocol change forces schema regen + conflict churn (the working tree during this review showed exactly that: dozens of modified/deleted schema JSONs). Mitigation: a dedicated fork-ext protocol module re-exported from v2/mod.rs, retire Op::UserTurn, and keep collaboration.rs the single fork-owned protocol module.

## 5. Prioritized improvement roadmap

### P0 — next iteration: restore severed features, fix safety bugs, install guards

| Item | Owner surface | Effort | Payoff |
|---|---|---|---|
| Re-wire the operation cache as a ≤5-line seam (cherry-pick from `98923e8d85`, incl. MCP hit-shaping; bookkeeping in the fork module); accept via the runtime canary | `core/src/tools/{mod,registry}.rs` + `core/src/tools/operation_cache.rs` | M | Revives a dead-4-weeks feature; the seam shape prevents the next merge loss |
| Re-wire the first-moves hit-recording hook (ideally behind a fork-owned post-tool-use observer seam) | `core/src/tools/registry.rs` → fork seam | S | Revives the lane's entire learning premise |
| Fix or retire the plan self-review checkpoint + characterization test | `core/tools/handlers/plan.rs`, codex-self-review | S | Restores a deliberate fork feature; test makes the next drop loud |
| DAB safety pair: `.kill_on_drop(true)` on the bridge child; validate click/bg_click coordinates + required x/y in schemas | `desktop-automation/src/windows.rs`, `dab_bridge_windows.ps1`, tool-registry-api specs | S+S | Stops post-timeout input injection + blind (0,0) clicks — real safety bugs |
| Port the missing upstream markdown_render features (key-value tables, column shrink, hyperlinks); delete orphaned `table_key_value.rs` | `tui-render/src/markdown_render.rs` | M | Restores silently-dropped user-visible upstream behavior |
| Fork-wiring smoke tests + script hardening: wiring-guard test for cache hits; caller-exists assertions per lane; fail-on-zero-matched-tests in canary scripts; playbook grep lines | core tests, scripts/ | M (batch of S) | Converts the fork's #1 failure mode (T1) from silent to red-build |
| Fix analytics `is_unconditional_custom` sync hazard (delegate to inner CustomFactReducer + one-assert test) | `analytics-appserver/src/reducer.rs` | S | No silent analytics loss when upstream adds fact variants |
| Test-debt quick wins: commit missing `subagent_activity_notifications` snapshot; fix bootstrap.rs test fields; add tui-render dev-deps; fix stale footer script | tui, app-server-client, tui-render, scripts/ | S×4 | Removes guaranteed-red tests; revives the 4.5k-line render test corpus |

### P1 — structural debt, perf, and next-merge enablers

| Item | Owner surface | Effort | Payoff |
|---|---|---|---|
| Execute the deferred **Phase-E test-repair wave** (per-crate `cargo check --tests`, fresh workers per playbook) | workspace-wide (~1,918 errors) | L | Restores the only regression safety net; every future merge cheaper to verify; enabler for half the items below |
| Async-hygiene batch: spawn_blocking the context-pack render, first-moves `predict()`, blackboard git/fs; lazy/async `GitReviewAnchor`; reorder task-memory gates (reuse existing token estimate) | first_moves.rs, blackboard, self-review/TUI, session/context_budget.rs | S–M each | Removes executor stalls and O(history) serialization from every fresh turn / sampling request |
| Finish OR revert the ForkFeaturesState migration | SessionConfiguration/TurnContext/SessionSettingsUpdate | M (revert: S) | Fork carriers 3→1 in the hottest merge files, or stop paying the mirror tax |
| Retire the fork-resurrected `Op::UserTurn` (move context_budget_mode to ThreadSettingsOverrides/UserInput ext) | protocol/op.rs, guardian/review_session.rs | M | Defuses the largest recurring protocol conflict |
| Dead-code purge: delete duplicate `agent_tool.rs` (~1,900 lines incl. tests), duplicate `desktop_automation.rs` + unused registry-plan lane, dedupe `cognos_ops.rs`; delete/gate the 4 dead port crates; decide replacement-shadow (wire or archive + delete flags); fix/remove orphaned scout handler; delete 43 stale snapshots + 9 orphaned test files | codex-tools, tools-domain, context-domain, thread/, replacement-shadow, handlers/, tui-render | batch of S | Removes drift traps and wrong-resolution magnets; shrinks codex-tools merge surface |
| Extract `apply_to_compaction()` helper (task-memory) — 3 hot files → 1 line each | core/src/task_memory.rs ← compact*.rs ×3 | M | Each upstream compaction refactor conflicts once, not three times |
| Collapse compaction domain-type triplication | context-reduction ← compaction-policy, context_reduction_adapter.rs | M | One place to add a reason/mode; less merge build-fix ceremony |
| Port upstream #26433 multi-environment tracking into codex-turn-diff + forward environment_id; expand its tests | turn-diff, core/src/tools/events.rs | M | Fixes a real correctness regression vs upstream; kills per-diff subprocess spawns |
| Behavioral tests for fork v2 lifecycle handlers (compact/close/restart/resume) | multi_agents_tests harness | M | The feature's only coverage today is name snapshots + a manual canary |
| Move fork additions in message_tool.rs/spawn.rs into `multi_agents_v2/collab_events.rs`; consider dual-emitting SubAgentActivityEvent | multi_agents_v2 handlers | M | Shrinks the hottest conflict surface in the multi-agent slice |
| Script the port passes + bank the maps: tui-render path-rewrite port; analytics rename-aware diff; thread-store rename map; facade drift-check (config_types, network-proxy, json_schema) | scripts/, merge playbook | S–M each | Turns the recurring "redirect tax" (T4) into mechanical scripted steps |
| Close the meta loop: taxonomy → one `merge-area-taxonomy.psd1` + new slices; finalize/backfill merge-metrics.csv; full fork-feature-inventory refresh (all 11 slices) | scripts/, docs/ | S | Resolver briefs and metrics regain accuracy for the next merge |
| Test-estate single-ownership cleanup (reconcile 3 divergent duplicates, point shims at core-test-suites) | core/tests, core-test-suites | M | Removes silent divergence + double compile; shrinks modify/delete surface |

### P2 — long-horizon merge-pressure reduction and polish

| Item | Owner surface | Effort | Payoff |
|---|---|---|---|
| **Upstream the pure code-motion splits** (protocol common.rs + v2/tests.rs; thread_history.rs + export.rs; turn_start.rs; app-server-client modules) as 2–3 mechanical PRs | app-server-protocol, app-server, app-server-client | M | Largest single merge-pressure reduction available (~150 conflicts/6wk → 0 if accepted) |
| Dedicated fork-ext protocol module for fork-only additions (MemoryStatus*, SubAgentSource, context_budget_mode); wire CI for `rmcp-conversions` or delete the gated code | app-server-protocol v2 | M | Hot-file protocol conflicts shrink to hub lines; no dead gated code |
| Persistent DAB stdio bridge (spawn once per session) + screenshot size guard | desktop-automation | L | ~10x latency on every GUI interaction; real cancellation |
| justfile de-conflict (profile variable, restore `{args}`, restore bazel-lock-check script call); Pester tests for the 4 merge-gate scripts; split build-local-codex.ps1 into a psm1 module | justfile, scripts/ | M | Removes a guaranteed every-merge conflict; puts the gates under test |
| Wire `is_mutating_tool` into DAB approval flow (or delete + document) | desktop-automation, core approval | M | Coherent safety model for mutating GUI tools |
| Coalesce inactive-agent output-delta cells; move multi_agents.rs fork builders into activity.rs | tui | M | Readable transcripts; smaller multi_agents.rs delta |
| Externalize first-moves' overfit compiled-in heuristics into per-repo data; dedupe memory-index recall with memories-context | first-moves, memories/context | L | Lane becomes portable; one source of truth for index schema |
| Migrate TUI legacy_core call sites to RPCs, then delete the shim; relocate non-render state out of tui-render if consumers grow | app-server-client, tui, tui-render | L | Completes the documented transitional architecture |
| Prune/archive `.codex/workflow` session artifacts (301 files, machine-specific markers) | .codex/ | S | Repo hygiene (zero merge pressure either way) |

## 6. Appendix — merge-pressure map

Where fork code still lives in (or collides with) upstream-hot files, and the extraction/automation that fixes each. Sorted roughly by pressure. Upstream heat figures are commit counts on upstream/main since ~2026-04-25 unless noted.

| Pressure | Fork surface in upstream-hot territory | Evidence / heat | Fix |
|---|---|---|---|
| HIGH (realized) | Operation-cache + first-moves hooks inline in `core/src/tools/registry.rs`; scout handler `mod` in `handlers/mod.rs`; plan checkpoint in `plan.rs` | All four already severed by merges | Re-wire as ≤5-line seams / fork-owned observer; wiring-guard tests (P0) |
| HIGH | Relocated test estate: upstream edits `core/tests/{suite,common}` (93 suite files) vs fork's core-test-suites + `#[path]` shims | 17 modify/delete conflicts in the 2026-06-05 merge; core-tests slices dominated both merges | Finish single ownership; delete stale copies; add core-test-suites/test-support slices to the partition taxonomy |
| HIGH | tui-render extraction: 1–5-line shims at upstream paths ⇒ whole-file conflicts, manual hand-port | 47 upstream commits on extracted paths since 2026-03; 19 on history_cell since May; markdown_render features already dropped | Scripted path-rewrite port pass per merge; revive snapshot tests; keep file layout byte-parallel |
| HIGH | analytics relocation: upstream `analytics/src/{events,reducer}.rs` vs fork `analytics-appserver` (rename tracking defeated) | events.rs 27 + reducer.rs 24 commits/6wk | Keep files 1:1 with upstream shape (do NOT re-split); scripted rename-aware diff-port step in the playbook |
| HIGH | Protocol/test re-export hubs vs upstream monoliths: common.rs, v2/tests.rs, turn_start.rs, thread_history.rs, export.rs; app-server-client lib.rs | common.rs 52, v2/tests.rs 46, turn_start.rs 29, lib.rs 21 commits/6wk | **Upstream the pure-motion splits** (no fork semantics — plausible PRs); until then, scripted hub-routing |
| HIGH | thread-store relocation + API split (`thread/thread-store` + `-api`) | lib.rs similarity down to ~58% vs actively-developed upstream crate | Record the rename map in the merge playbook; consider drift-check script |
| MOD-HIGH | `Op::UserTurn` resurrected in protocol/op.rs (upstream deleted it) to carry per-turn context_budget_mode | Every upstream Op/turn-path change conflicts | Retire: move the field to ThreadSettingsOverrides/UserInput ext (2 call sites) |
| MOD-HIGH | 3 scalar fork fields + mirrored fork_features bundle through SessionConfiguration/TurnContext/SessionSettingsUpdate (~8 constructor sites) | 4 parallel carriers per constructor | Finish the bundle migration (3→1 carriers) or revert the bundle |
| MOD | multi_agents_v2 handlers: API-migration churn + inline fork features in 6 upstream files; 12 Collab* event types through protocol → app-server-protocol → TS → schema JSONs | 3 upstream PRs in the area since May; schema files upstream-hot | Extract to `collab_events.rs` sibling module; dual-emit SubAgentActivityEvent; fork-ext protocol module |
| MOD | Compaction trio: same 4-step task-memory pattern in compact.rs / compact_remote.rs / compact_remote_v2.rs; 1 call in session/turn.rs | Already forced one re-implementation (upstream #27106) and one drop (plan.rs) | One `apply_to_compaction()` helper → 1 line per hot file |
| MOD | tui multi_agents.rs ~370-line interleaved fork delta (match arms, label spans) | Every upstream collab-tool addition conflicts | Move builders/spans into fork-only activity.rs; keep only match-arm registrations |
| MOD | 1-line facades: `protocol/src/config_types.rs`, `network-proxy/src/config.rs`, `tools/src/json_schema.rs` (18-line shim over tool-schema) | Upstream keeps editing the original files (config_types upstream = 871 lines) | Mechanical port-hunk resolution; add a facade drift-check script to the boundary checker |
| MOD | Fork-deleted upstream files: `config/src/thread_config/remote.rs` (−523) + protos | Modify/delete conflicts recur each merge | Record the deletion list in the playbook so resolvers stop rediscovering it |
| MOD | Fork-only protocol additions inline in v2/thread.rs (+26), v2/turn.rs (+12) | v2/thread.rs 19 commits/6wk | Dedicated fork-ext module re-exported from v2/mod.rs |
| LOW-WIDE | `plan_mode_reasoning_effort`: one-line fields across config God files (config_struct, config_loaders, config_lock, config_toml, profile_toml, schema) | Frequent but mechanical | Playbook "restore dropped member" pattern; no extraction warranted |
| GUARANTEED | justfile whole-file `--release` + `"$@"` rewrite | Conflicts on any upstream recipe change; breaks `{args}` portability | Single `profile` variable + restore `{args}` tokens → ~2-line permanent delta |
| LOW | windows-sandbox-rs dep-bump textual diff (17 files); core-plugins manager_tests split | Steady low-grade noise; conflicts localized per scenario module | Accept; nothing further needed |
| ZERO | Fork-only owner crates (context-pack, context-reduction, compaction-policy, prompt-context, operation-cache, blackboard, agent-policy/graph-store, desktop-automation + ps1, tui-render *contents*, turn-diff, cognos-ops, tools-domain, config-types, permission-types, first-moves, scout, task-memory, self-review, analytics-appserver *crate*, scripts/, .codex/, docs/) | — | This is the model working as designed — keep putting fork logic here |

**Bottom line.** The owner-crate strategy is validated: fork-only crates carry zero conflict surface, and the realized merge damage all came from the remaining inline touchpoints and unscripted relocation ports. The next merge gets materially cheaper if, before it happens, the fork (1) lands the P0 re-wiring + wiring-guard tests, (2) scripts the four port passes (tui-render, analytics, thread-store map, facades), (3) upstreams the pure-motion splits, and (4) refreshes the inventory/taxonomy/metrics that the merge automation itself consumes.

---

*Sources: per-slice findings in `.codex/tmp/fork-review/{context-budget,operation-cache,tui-ux,tui-render,self-review-task-memory,multiagent-blackboard,experimental-lanes,desktop-automation,owner-crates-config,server-side,build-test-automation}-findings.md`; `docs/fork-feature-inventory.md` (2026-05-15, stale); `Main_Merge_Prompt.md` (the recurring SRP/DIP merge-reduction goal). Reviewer disagreements were minimal; one brief-level correction is recorded: `ext/goal` exists upstream (the fork delta is only an extension-api adaptation), contrary to an earlier assumption. Claims about test behavior are static-analysis based — cargo was not run during this review (release build owned the target dir).*
