status: finding

## Findings

### Depth policy leak appears fixed

The recursive persisted-descendant resume path now uses the policy boundary rather than local arithmetic.

Evidence:
- `codex-rs/core/src/agent/control.rs:540-568` computes `child_depth` with `crate::agent::policy::next_thread_spawn_depth(parent_depth)` before constructing `SubAgentSource::ThreadSpawn { depth: child_depth, ... }` and before enqueueing the child.
- A targeted search of the reviewed policy/depth files found no remaining `parent_depth + 1` or `depth + 1` depth calculation in this path.
- `codex-rs/core/src/agent/registry.rs:76-81` also delegates `next_thread_spawn_depth` and `exceeds_thread_spawn_depth_limit` through `agent::policy`.
- `codex-rs/core/src/agent/policy.rs:13-18` delegates those helpers to `codex_agent_policy`.

Exact next action for root:
- Treat the original P2 resume-descendant depth finding as closed for source behavior, subject to the commit-boundary and verification items below.

### Agent-policy coverage is sufficient for the helper claim

The dirty `codex-rs/agent-policy/src/lib.rs` coverage is enough for the new policy helper.

Evidence:
- `codex-rs/agent-policy/src/lib.rs:45-50` defines `next_thread_spawn_depth(parent_depth)` with `saturating_add(1)` and keeps max-depth comparison in the same policy crate.
- `codex-rs/agent-policy/src/lib.rs:743-755` covers normal increments, `i32::MAX` saturation, and max-depth comparison.
- The fix handoff reports `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-agent-policy` passed.

Exact next action for root:
- Include `codex-rs/agent-policy/src/lib.rs` with the depth-policy slice if committing this behavior.

### Commit boundary is mixed and can break if staged incompletely

The depth fix is mixed with graph-store/policy adapter extraction in the same dirty source area. A tracked-only commit of the reviewed files would miss required untracked modules.

Evidence:
- `git status --short -- codex-rs/core/src/agent/graph_store.rs codex-rs/core/src/agent/policy.rs codex-rs/agent-graph-store/src/thread_spawn_graph.rs codex-rs/core/src/agent/control.rs codex-rs/core/src/agent/mod.rs codex-rs/core/src/agent/registry.rs codex-rs/agent-policy/src/lib.rs` reports:
  - `M codex-rs/agent-policy/src/lib.rs`
  - `M codex-rs/core/src/agent/control.rs`
  - `M codex-rs/core/src/agent/mod.rs`
  - `M codex-rs/core/src/agent/registry.rs`
  - `?? codex-rs/agent-graph-store/src/thread_spawn_graph.rs`
  - `?? codex-rs/core/src/agent/graph_store.rs`
  - `?? codex-rs/core/src/agent/policy.rs`
- `codex-rs/core/src/agent/mod.rs:3-5` declares both `graph_store` and `policy`, so omitting the untracked files would break the module tree.
- `codex-rs/core/src/agent/control.rs` imports `crate::agent::graph_store::*` in addition to the depth-policy change, so the control diff is not a depth-only diff.

Exact next action for root:
- Either commit the broader coherent agent graph-store/policy adapter slice with all required untracked files and related manifest/lock updates, or split the one-line depth fix plus policy helper/test into a smaller clean commit. Do not stage only the tracked `control.rs`/`mod.rs`/`registry.rs` files.

### Core verification remains blocked by existing test-support dependency drift

I did not run tests because this worker is read-only. The prior verification blocker is credible from the handoff and local source/log evidence.

Evidence:
- The fix handoff reports the final attempted command `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter multi_agent_v2 -AllowBroadCoreLibUnitTests` compiled `codex-core` and then failed in `core_test_support`.
- `codex-rs/core/tests/common/test_codex.rs:44-46` imports `codex_thread_store` and `codex_thread_store_api`.
- `logs/test-local-release-codex-core-multi_agent_v2-20260521-125357.log` reports unresolved imports for those crates in `core_test_support`.

Exact next action for root:
- Reconcile `codex-rs/core/tests/common/Cargo.toml` with the `codex_thread_store` / `codex_thread_store_api` imports, then rerun:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter multi_agent_v2 -AllowBroadCoreLibUnitTests
```

## Moving Tree Check

No moving-tree blocker was observed in the reviewed files before writing this handoff. The tree is broadly dirty outside this slice, so root should use path-scoped staging and re-run `git status --short` immediately before commit.
