# Custom Codex Improvements From Wizard_Erasmus And Cognos

## Scope

This note reviews what the local custom Codex fork should borrow from:

- `C:\Users\Oleh\Documents\GitHub\Wizard_Erasmus`
- `C:\Users\Oleh\Documents\GitHub\Cognos`
- the current Codex Rust workspace under `codex-rs`
- local Codex/Wizard conversation and tool telemetry from the week ending
  April 29, 2026

The goal is not to merge Wizard_Erasmus or Cognos into Codex. The useful path is
small native Codex changes that reduce repeated prompt cost and make future
automation more structural.

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
  - `codex-rs/core/src/context_manager/*`
  - `codex-rs/core/src/mcp_tool_exposure.rs`
  - `codex-rs/features/src/lib.rs`
  - `codex-rs/memories/README.md`
  - `codex-rs/app-server/README.md`
  - `codex-rs/core/src/tools/handlers/multi_agents*`

## Local Signals

Recent Wizard telemetry showed a low cache hit rate, about 0.1104, while still
saving roughly 13,276,034 bytes. Misses were dominated by repeated read/search
families, especially `Read`, `Grep`, and shell commands that render file
contents. Bash-style commands were often not cacheable because freshness and
side effects are ambiguous.

Recent Codex history on this machine showed many continuation turns such as
`go on` and `Implement the plan`, with repeated `Get-Content` and `rg -n`
queries while studying the same repos. This points to prompt resend cost more
than execution cost: even when a tool result is already known in the transcript,
the full text can be carried into later model requests again.

## Findings

### Cache

Wizard's cache work is useful because it is telemetry-driven and miss-reason
oriented. The important lesson for Codex is not to port the whole cache layer,
but to reduce the two largest obvious prompt costs first:

- eager MCP schema exposure
- repeated large tool outputs in prompt history

Codex already has a native tool discovery surface: `tool_search`. The existing
`tool_search_always_defer_mcp_tools` flag is therefore a low-risk way to avoid
eagerly injecting MCP schemas. Making it a local default fits this fork's goal
without changing the tool execution model.

For tool outputs, a full external cache would have to solve invalidation,
freshness, cross-tool semantics, and UI consistency. A smaller Codex-native
improvement is safer: keep raw history full, but elide later identical large
plain-text outputs only in the prompt clone.

### Memory

Codex already has a two-phase memory pipeline. Wizard and Cognos both reinforce
the need for visible memory status and low-signal filtering. A read-only
`memory/status` API remains a good idea, but it is not the best first local
implementation slice because app-server APIs may be changed remotely.

For this branch, memory work should stay in docs and later planning. The first
implemented memory-adjacent win is lower prompt duplication, which also reduces
the pressure to compact or summarize too early.

### Team App And Cross-Agent Control

Wizard Team App and loop docs show that terminal/window automation is fragile.
The durable direction is structural APIs: status, targeted waits, app-server
thread control, and resumable agent state.

Targeted `wait_agent` for multi-agent v2 is still a strong follow-up, but it
overlaps with remote multi-agent development. It should not be mixed into this
local token-saving branch.

### Cognos Patterns

Cognos is most useful here as a design reference:

- explicit inspection commands
- reviewer/planner discipline
- evidence summaries
- confidence and retrieval explainability
- low-signal memory filtering

Do not port the always-on cognitive loop, native memory graph, Prolog/ProbLog
reasoning, or broad automatic memory mutation into Codex before the small
control-plane and token-saving primitives are proven.

## Ranked Recommendations

1. Keep official Codex installed system-wide and use the fork executable only
   inside this repo while changing/testing it.
2. Enable `tool_search_always_defer_mcp_tools` by default in the local fork.
3. Add prompt-time duplicate elision for repeated large plain-text tool outputs.
4. Keep docs explicit that raw transcript/history remains complete.
5. Park `memory/status` and targeted `wait_agent` until remote-overlap risk is
   lower.
6. Later, add `mcp/cache/status` and compact Team App status APIs after real
   sessions validate the first two token-saving changes.
