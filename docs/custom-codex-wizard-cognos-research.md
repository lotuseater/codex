# Custom Codex Improvements From Wizard_Erasmus And Cognos

## Scope

This note reviews what the local custom Codex fork can borrow from:

- `C:\Users\Oleh\Documents\GitHub\Wizard_Erasmus`
- `C:\Users\Oleh\Documents\GitHub\Cognos`
- the current Codex Rust workspace under `codex-rs`

The goal is not to merge either project into Codex. The useful direction is to
add small native Codex APIs and agent-control behavior that make Wizard/Team App
automation more reliable and reduce repeated work.

## Sources Inspected

- Wizard research and policy docs:
  - `Wizard_Erasmus/research/Cache_Chaining_Implementation_Plan_2026_04_17.md`
  - `Wizard_Erasmus/docs/research/cache_hit_rate_audit_2026_04_20.md`
  - `Wizard_Erasmus/docs/research/codex_memory_cache_repo_audit_2026_04_22.md`
  - `Wizard_Erasmus/docs/loop_session_control_antipatterns.md`
  - `Wizard_Erasmus/docs/cognos_codex_synthesis.md`
- Wizard implementation surfaces:
  - `Wizard_Erasmus/src/mcp/tool_cache.py`
  - `Wizard_Erasmus/src/mcp/hooks/*cache*`
  - `Wizard_Erasmus/src/team_app/*`
  - `Wizard_Erasmus/ai_wrappers/*loop*`
- Cognos implementation and docs:
  - `Cognos/src/memory/knowledge_base.*`
  - `Cognos/src/core/probability.*`
  - `Cognos/src/orchestration/team_orchestrator.*`
  - `Cognos/src/orchestration/consciousness.*`
  - `Cognos/docs/cognos_hybrid_architecture_research.md`
  - `Cognos/docs/cognos_implementation_roadmap.md`
- Codex implementation surfaces:
  - `codex-rs/memories/README.md`
  - `codex-rs/app-server/README.md`
  - `codex-rs/app-server-protocol/src/protocol/v2.rs`
  - `codex-rs/app-server/src/codex_message_processor.rs`
  - `codex-rs/core/src/tools/handlers/multi_agents*`
  - `codex-rs/codex-mcp/src/*`

## Findings

### Memory

Codex already has a real two-phase memory pipeline. Phase 1 extracts per-thread
memory records into the state DB, and Phase 2 consolidates them into
`CODEX_HOME/memories` with a serialized workspace diff. App-server already has
`thread/memoryMode/set` and `memory/reset`, but it lacks a read-only status API
for external controllers.

Wizard's memory/cache audit notes repeatedly needed a single status surface for
memory health, generated artifacts, and cache visibility. Cognos reinforces the
same idea from another angle: it separates working, episodic, semantic, and
procedural memory and makes retrieval/debug status visible through inspection
commands.

Best immediate Codex improvement: add `memory/status` as a small read-only
app-server v2 method. It should report filesystem memory artifact counts and
sizes without triggering consolidation or mutating DB state.

### Cache

Wizard's cache work is mature and telemetry-driven: it tracks miss reasons,
session scope, source agent, invalidations, and cache-hit hotspots. Its strongest
lesson is that cache changes should follow observed miss families, not broad
whitelist guesses.

Codex already has native cache-related pieces:

- deferred tool loading and `tool_search`
- Codex Apps tool disk cache under `codex-rs/codex-mcp`
- app-server plugin/skill cache clearing
- connector cache refresh paths

Best immediate Codex improvement is documentation and API planning, not a large
cache rewrite. A later `mcp/cache/status` app-server method can expose Codex Apps
tool-cache age, hit/miss labels, and refresh status. Tool-output caching like
Wizard's `tool_cache.py` should stay outside the first slice because it crosses
tool runtimes, invalidation, and token accounting.

### Team App And Cross-Agent Control

Wizard Team App carries strong process lessons: visible team runs need reliable
status, targeted resume, review gates, and safe terminal/session recovery. The
loop anti-patterns show that raw terminal/window automation is fragile and can
damage unrelated sessions.

Codex already has native multi-agent control and app-server thread APIs. The
useful bridge is to improve native cross-agent status and wait behavior so Team
App can drive Codex through app-server/agent-control surfaces instead of
guessing terminal state.

Concrete immediate gap: multi-agent v2 `wait_agent` currently only waits for
mailbox changes. The older multi-agent wait supports explicit targets and final
status maps. Porting that behavior into v2 is low risk and directly helps
supervisors, loops, and Team App controllers.

### Loop

Wizard loop failures came from terminal focus, coordinate clicks, dry-run flags
accidentally used in production, too many windows, and delayed first nudges
after resume. These should not be copied into Codex.

Codex should instead expose structural primitives that make external loops
boring:

- read thread and loaded-thread status through app-server
- wait for specific agent/thread statuses natively
- resume/fork/start through app-server rather than terminal text injection
- report memory/cache status without launching side effects

### Cognos Patterns

Cognos is useful as a design reference, not a subsystem to port wholesale.

Good borrow candidates:

- explicit status/inspection commands
- planner/reviewer discipline for multi-agent workflows
- evidence cards and blackboard-style summaries for future reviews
- retrieval explainability and low-signal filtering
- Beta-distribution confidence tracking for future memory/belief scoring

Do not port now:

- always-on C++ cognitive loop
- Prolog/ProbLog default reasoning
- native memory graph
- broad automatic memory mutation

Those would add high complexity before Codex has the small control-plane APIs
that Wizard and Team App actually need.

## Ranked Recommendations

1. Add safe clean fast rebuild script for local fork operations.
2. Add `memory/status` app-server v2 API.
3. Add explicit-target support to multi-agent v2 `wait_agent`.
4. Document `mcp/cache/status` as the next cache observability API.
5. Later: expose a compact agent/team status API for Team App control.
6. Later: add memory retrieval/status scoring explanations only after telemetry
   shows where current memory use fails.

