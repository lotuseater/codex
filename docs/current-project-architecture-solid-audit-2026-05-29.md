# Current Project Architecture — SOLID Refactor Audit

**Date:** 2026-05-29 · **Branch:** `slow-context-budget-mode` (fork) · **Method:** read-only audit (4 parallel slices: grep / `wc -l` / `git show` / targeted reads) against the two source docs.

**Audited against:**
- `docs/current-project-architecture-solid-review.md` (11 findings + roadmap + Core↔App-Server addendum)
- `docs/current-project-architecture-solid-refactor-plan.md` (6 refactor phases + dependency rules)

**Question answered:** for each finding/phase, was the **code** actually moved in the prescribed direction, and were the **tests** improved the same way?

---

## Status legend
✅ Done · 🟡 Partial / in-progress · ❌ Not started (scaffold-only or unchanged) · ⚠️ Regressed (target file grew / moved backwards)

---

## Executive summary

The refactor is in an **early-scaffolding** state: the plan's *crate layout* exists broadly on paper (new `*-api`, `*-domain`, and `thread-/session-/turn-` package trees are real workspace members), but **logic migration is uneven and the highest-blast-radius monoliths are unchanged or larger.**

**Genuine wins (✅):**
- **Dependency direction — §1 / Addendum:** `codex-core` now has **0** dependencies on app-server protocol types; `core` and `app-server-protocol` are **mutually decoupled**. This is the single biggest architectural improvement and it is real.
- **Thread persistence (Plan Phase 2):** `thread-store` (6,931 LOC) + `thread-store-api` (1,863) are genuinely extracted and **consumed by core** (96 refs across 17 files).
- **TUI `app.rs` / `chatwidget.rs` (§9):** richly decomposed into submodule trees with **co-located, concern-split tests** — the strongest code+test improvement in the repo.
- **Tool dispatch (§5):** decentralized to a `HashMap`-based `ToolRegistry` + ~34 per-tool handler modules + handler traits (no monolithic match); tests reorganized to match.
- **Config loader boundary (§10):** the `config` crate is well-segmented (dedicated `loader/` package, fine-grained modules) with concern-split tests.

**Not started / scaffold-only (❌):**
- **Session & Turn split (Plan Phases 3–4):** all 8 session + 7 turn crates exist but are **DTO/skeleton stubs unused by core**; `turn-loop` is a self-described toy "without depending on core." The live monoliths are intact.
- **App-server client facade (§8):** only `remote.rs` peeled off; the prescribed six-way split is absent.
- **Local-fork feature isolation (§11):** `collaboration_mode` / `personality` / `context_budget_mode` are still threaded through core (200+ non-test refs).

**⚠️ Regressions — several of the docs' explicit target files GREW since the review snapshot:**

| File | Review baseline | Now | Δ |
|---|---|---|---|
| `tui/src/bottom_pane/chat_composer.rs` (Phase 5's #1 target) | 9,720 | **10,988** | +1,268 |
| `core/src/config/mod.rs` (§10) | 3,311 | **3,853** | +542 |
| `cli/src/main.rs` (§7) | 2,953 | **3,464** | +511 |
| `protocol/src/protocol.rs` (§6) | 4,592 | **4,978** | +386 |
| `tui/src/bottom_pane/mod.rs` | 2,650 | **3,000** | +350 |
| `core/src/session/mod.rs` (§4) | 3,244 | **3,512** | +268 |
| `core/src/session/turn.rs` (§4) | 2,620 | **2,983** | +363 |
| `app-server-client/src/lib.rs` (§8) | 2,120 | **2,213** | +93 |

**Tests verdict:** tests improved *where the code did* — thread-store (real unit tests), TUI `app/*` + `chatwidget/*` (co-located split suites), config (concern-split). They did **not** improve where code didn't: the ~16,400 LOC session/turn test mass is still inline-coupled to the core monolith, and the new session/turn/runtime/context crates carry only **skeleton tests** that exercise stubs, not behavior.

---

## Findings scorecard (review §1–§11 + Addendum)

| # | Finding | Code | Test | One-line |
|---|---|---|---|---|
| §1 | Core depends on app-server protocol types | ✅ | n/a | `core` has 0 app-server-protocol deps (Cargo + src); fully decoupled. |
| §2 | `codex-core` is still a dependency hub | 🟡 | n/a | Still **39** crates depend on `codex-core`; extraction into new `*-api` crates begun but core remains central. |
| §3 | `codex-core-api` is a facade, not a boundary | 🟡 | n/a | 65-line re-export facade ("for thread-management samples"), only **4** consumers; owns nothing. |
| §4 | Session & Turn mix too many reasons to change | ❌⚠️ | ❌ | `session/mod.rs` 3,512 + `turn.rs` 2,983 still mix all responsibilities inline; both **grew**; no phase objects in live code. |
| §5 | Tool routing too centralized | 🟡 | ✅ | Registry+HashMap dispatch & per-tool modules done; `spec_plan.rs` (572) still centrally wires ~56 handlers. |
| §6 | Public protocol file combines too many families | 🟡⚠️ | 🟡 | `protocol.rs` **grew to 4,978**; only 3 tiny families (~8%) carved out; `Op`/`EventMsg` still combined. |
| §7 | CLI entrypoint is a command switchboard | 🟡 | 🟡 | Command families extracted to modules, but `main.rs` (**3,464**) still owns the enum + 25-arm dispatch + inline exec. |
| §8 | App-server client facade large, blurs transport/domain | ❌ | 🟡 | Only `remote.rs` split off; `lib.rs` (2,213) still blends transport/RPC/thread/turn/notification. |
| §9 | TUI components have extreme file/responsibility size | 🟡 | ✅ | `app/*` & `chatwidget/*` richly decomposed w/ co-located split tests; but `chat_composer.rs` **regressed to 10,988**. |
| §10 | Good loader boundary, core config remains heavy | 🟡 | ✅ | `config` crate well-segmented; `core/config/mod.rs` **grew to 3,853**, still centralizes the `Config` struct. |
| §11 | Local fork features depend on high-touch core paths | ❌ | n/a | `collaboration_mode` (98), `personality` (81), `context_budget_mode` (30) still threaded through `core/src`. |
| Add. | Core ↔ App-Server protocol boundary | ✅ | n/a | Mutual decoupling: neither `core` nor `app-server-protocol` depends on the other. |

## Plan phase scorecard

| Phase | Intent | Code | Test | One-line |
|---|---|---|---|---|
| P1 | Enforce boundaries first | 🟡 | ❌ | `deny.toml` `[bans]` exists (crate hygiene, not arch-layering); `runtime-ports` canary (28 LOC) is **unused**; no architecture test. |
| P2 | Thread package split | 🟡 | 🟡 | `thread-store`(6,931)/`thread-store-api`(1,863) real & core-consumed; but local/memory/factory merged into one crate and `thread-manager` (1,578) still in core. |
| P3 | Session package split | ❌ | 🟡 | 8 crates are DTO/skeleton stubs (46–341 LOC) unused by core; live `core/src/session` ~12.8k LOC intact. |
| P4 | Turn package split | ❌ | 🟡 | 7 stub crates; `turn-loop` is a toy "without depending on core"; real loop still in `core/src/session/turn.rs`. |
| P5 | Core domain-type ownership | 🟡 | n/a | `config-types` (1,118) + `protocol` (13,387) own many types; but `core/config/mod.rs` still owns the heavy `Config`. |
| P6 | Tool/context/runtime ports | 🟡 | 🟡 | All 3 domain crate trees exist; **tool** ports real & consumed; **runtime/context** ports are thin stubs (23–45 LOC), `runtime-ports` unused. |

---

## Detail & evidence

### §1 + Addendum — Core ↔ App-Server protocol boundary — ✅ DONE
- `core/Cargo.toml`: **no** app-server-protocol dependency. `grep codex_app_server_protocol core/src` (non-test) = **0** import sites.
- Direction confirmed both ways: `app-server-protocol/Cargo.toml` does **not** depend on `codex-core`, and `core` does **not** depend on `app-server-protocol`. The change-magnet coupling the review flagged is gone.
- **Test:** no dedicated boundary/architecture test locks this in (relies on convention + `deny.toml`); a `cargo-metadata`-based direction test would prevent regression.

### §2 — `codex-core` dependency hub — 🟡 PARTIAL
- **39** workspace `Cargo.toml` files reference `codex-core`. Many new boundary crates were spun out (`*-api`, `tools-domain/*`, `runtime-domain/*`, `context-domain/*`, `thread-*`), but core is still the central dependency.

### §3 — `codex-core-api` facade — 🟡 (still a facade)
- `core-api/src/lib.rs` = **65 lines**, doc-comment: *"Public facade for non-core APIs used by thread management samples."* It is almost entirely `pub use` re-exports of `codex_config`, `codex_analytics`, `codex_arg0`, `codex_extension_api` types — it **owns nothing** and only **4** crates depend on it. Not yet the boundary the review wants.

### §4 — Session/Turn SRP — ❌ / ⚠️ (grew)
- `core/src/session/mod.rs` = **3,512** (↑ from 3,244); `core/src/session/turn.rs` = **2,983** (↑ from 2,620). Still mix run-loop, compaction, sampling, prompt-building, plan-mode, tools inline as free functions. **None** of the prescribed phase objects (`TurnRunner`, `SamplingPipeline`, `CompactionPolicy`, `PlanModeProjector`, `TurnEventSink`) exist in the live path.

### §5 — Tool routing — 🟡 PARTIAL (mechanism ✅)
- Decentralized: `ToolRegistry { tools: HashMap<ToolName, Arc<dyn RegisteredTool>> }` (`core/src/tools/registry.rs:478`), thin router (`router.rs`, 251 LOC), ~34 handler modules under `tools/handlers/`, `CoreToolRuntime` trait + externalized `tool-execution-api`/`tool-registry-api`.
- Remaining: `core/src/tools/spec_plan.rs` (**572**) still centrally `use`s + registers ~**56** concrete handlers (Open/Closed weakness persists).
- **Test ✅:** `router_tests.rs`, `registry_tests.rs`, `spec_plan_tests.rs` (~109 KB), `tool_dispatch_trace_tests.rs`, per-handler `*_tests.rs`.

### §6 — Protocol families — 🟡 / ⚠️ (grew)
- `protocol/src/protocol.rs` = **4,978** (↑ from 4,592). Still defines both `Op` (l.350) and `EventMsg` (l.871). Only `exec_command`/`mcp_tool`/`review` (~398 LOC, ~8%) carved out via the plan's re-export pattern.
- Contrast: `app-server-protocol/src/protocol/v2/` is well-split (~30 family files) — but carries its own monoliths `common.rs` (3,233) and `thread_history.rs` (3,452).

### §7 — CLI switchboard — 🟡 PARTIAL
- `cli/src/main.rs` = **3,464** (↑ from 2,953). Command families exist as modules (`doctor.rs` 4,040, `mcp_cmd.rs` 989, `plugin_cmd.rs` 524, `login.rs` 474, `marketplace_cmd.rs` 354, …), but `main.rs` still owns `enum Subcommand` (l.116) + a ~25-arm `match subcommand` (l.840-1353) + inline strict-config policy. No `CommandRunner`/table.
- **Test 🟡:** per-family integration tests under `cli/tests/` (pre-existing surface tests, not new per-module units).

### §8 — App-server client facade — ❌ NOT STARTED
- `app-server-client/src/lib.rs` = **2,213** (↑ from 2,120) still bundles `InProcess*Client`, request handles, `AppServerClient`, event conversion. Only `remote.rs` (1,035) + `request_method.rs` (15) split off. The prescribed transport/RPC/thread/turn/notification/bootstrap split has not happened.

### §9 — TUI extreme size — 🟡 (mixed; strong where done)
- **Done well:** `tui/src/app.rs` (1,376) is now a dir — `app/event_dispatch.rs` 2,288, `thread_routing.rs` 1,585, `config_persistence.rs` 1,426, `background_requests.rs` 1,136, … ; `tui/src/chatwidget.rs` (1,927) is now a dir — `chatwidget/plugins.rs` 2,193, `slash_dispatch.rs` 1,140, `status_surfaces.rs` 1,047, … with co-located, concern-split tests (`chatwidget/tests/*`, `app/tests*`).
- **Regressed:** `bottom_pane/chat_composer.rs` = **10,988** (↑ from 9,720) — the file Phase 5 says to split *first*; `bottom_pane/mod.rs` = **3,000**. The composer state/completion/attachments/modes/render/side-effects split is not done.

### §10 — Core config heavy — 🟡 PARTIAL
- `config` crate (loader boundary): healthy — dedicated `loader/` pkg + fine-grained modules (`config_requirements.rs`, `types.rs`, `config_toml.rs`, …), concern-split tests.
- `core/src/config/mod.rs` = **3,853** (↑ from 3,311) — still one giant `pub struct Config` (l.532, fields to 1278) + two big `impl Config` blocks. Peripheral concerns (permissions, agent_roles, otel) are modular; the resolved-config shape + read/write API are not split.

### §11 — Local fork feature coupling — ❌ NOT IMPROVED
- Non-test refs in `core/src`: `collaboration_mode` **98**, `personality` **81**, `context_budget_mode` **30** — these fork features remain threaded directly through high-touch core paths (config, session, turn), not isolated behind seams/ports.

### Plan P1 / Dependency Rules — 🟡 PARTIAL (soft enforcement)
- `deny.toml` exists with `[bans]` (l.191) + `deny = [...]` (l.216) but `wildcards = "allow"` — it governs crate/version hygiene, **not** architectural layering. `runtime-domain/runtime-ports/src/lib.rs` (28 LOC) is the intended "canary" but is consumed by **nobody** (only itself). No `cargo-metadata`/architecture test enforces the direction the §1 win achieved.

### Plan P6 — Ports — 🟡 (tools real; runtime/context stubs)
- `tools-domain/` ports (`tool-execution-api` 597, `tool-registry-api` 434, `tool-handler-api` 35) are substantial and genuinely consumed by core.
- `runtime-domain/` (`auth-api` 37, `model-client-api` 31, `runtime-ports` 28, `state-db-api` 33, `telemetry-api` 23) and `context-domain/context-budget` (24) are thin trait skeletons; only `auth-api` is wired into core; the rest are unadopted.

---

## Recommended next steps (priority order)

1. **Lock in the §1 win** with a `cargo-metadata`-based architecture test (forbid `core → app-server-protocol`, `turn → session` concrete types) so the one real boundary can't regress. *(P1 is "enforce boundaries first" — currently the weakest-enforced of the real wins.)*
2. **Stop the regressions before adding more crates:** `chat_composer.rs` (10,988), `core/config/mod.rs` (3,853), `cli/main.rs` (3,464), `protocol.rs` (4,978) are all growing *against* the plan. Splitting `chat_composer.rs` (Phase 5's #1) and `protocol.rs`'s `Op`/`EventMsg` would deliver the most blast-radius reduction.
3. **Convert skeletons to migrations:** the session/turn/runtime/context crates are scaffolds the live code doesn't use. Either wire them into core (move logic + tests) or defer creating them — empty stubs add dependency surface without SRP benefit, and their skeleton tests give false coverage signal.
4. **Migrate the session/turn test mass** (~16,400 LOC inline in `core/src/session/tests*`) alongside whatever logic moves, so tests track ownership.
5. **Promote `core-api` from sample-facade to real boundary** (or retire it) and route the 39 `codex-core` dependents through owning crates to actually reduce the hub (§2/§3).

---

*Audit is read-only — no code or tests were modified. The unrelated `LowMemRelease` build+deploy was running concurrently and is unaffected.*
