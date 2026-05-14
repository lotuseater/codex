# Strategic Merge Maintainability Plan

Date: 2026-05-14

This note captures issues found during the `slow-context-budget-mode` merge with `origin/main` and the blackboard work. The purpose is to avoid short-term conflict resolutions that make future merges slower, heavier, or easier to regress.

## Current State

- The branch is in an in-progress merge with `MERGE_HEAD` present.
- No unmerged index entries were found during the latest inspection.
- A stale `codex-tui auto_loop` release test was stopped because source files had changed after it started.
- A fresh `codex-core --lib multi_agents` release canary was intentionally stopped after it began compiling heavy runtime dependencies (`rama-*`, `starlark`) unrelated to a narrow multi-agent canary.

## Immediate Commit Blockers

- `git diff --cached --check` reported a whitespace failure in `codex-rs/tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__review_submission_warning_snapshot.snap`.
- Several tracked module declarations reference files that were still untracked at audit time. Required spec/handler files must be staged with the merge, otherwise the commit will not build.
- MultiAgentV2 lost part of the current supervision tool surface during the tool-registry merge. `compact_agent`, `restart_agent`, and `resume_agent` need to be registered or explicitly deferred with compatibility notes.
- The new public `codex-tools::ToolExecutor` trait uses `#[async_trait]`. Convert it to the repo-preferred RPITIT shape before the API spreads.

## Dependency Graph Findings

- `codex-config` was pulling the runtime-heavy `codex-network-proxy` crate, which brought in `rama-*` dependencies even for config-only consumers.
- `codex-protocol` also depended on `codex-network-proxy` only for network policy DTOs, which reintroduced the same runtime-heavy proxy graph into `codex-config` through protocol dependencies.
- A first split now moves network proxy config/data types into `codex-network-proxy-config`, while `codex-network-proxy` remains the runtime crate and re-exports the same public types.
- A second split moves protocol-safe network policy DTOs (`NetworkPolicyDecision`, `NetworkDecisionSource`) into `codex-network-proxy-config`, allowing `codex-protocol` to avoid depending on the runtime proxy crate.
- A third split moves the hot `ContextBudgetMode` DTO into `codex-config-types` and re-exports it from `codex-protocol`. This gives slow-mode default checks a tiny release canary instead of requiring a full `codex-protocol` test binary.
- Release artifact growth had two separate causes. Orphaned `target/release/deps/lib*.rlib` and `lib*.rmeta` files are now safe cleanup candidates only when their Cargo `.d` dep-info no longer references them; same-name hash variants must not be pruned blindly because they can represent active feature/build-unit variants.
- `Cargo.lock` currently has duplicate package versions. The build diagnostic now classifies exact known unavoidable/transitional duplicates separately from action-required duplicates. Known examples include Windows target crates, common proc-macro/schema major-version transitions, crypto/randomness major-version transitions, and the temporary fork/upstream websocket split. This is an audit allowlist, not permission to delete active artifacts.
- The current duplicate inventory is captured in `docs/cargo-duplicate-dependency-audit.md`. The quick direct duplicate `quick-xml 0.38.4` was removed by aligning the workspace dependency to the already-present `0.39.4`; remaining high-value action-required candidates include `which`, `unicode-width`, `toml`, `zip`, `zstd`, and `http`/`http-body`, but each now needs reverse-owner analysis before changing manifests.
- Remaining heavy core compile cost is structural: `codex-core` directly owns shell execution, execpolicy, managed network proxy, sandboxing, protocol, and many runtime integrations. This cannot be fixed by version dedupe alone.
- `codex-protocol` is still heavy because it owns unrelated API, error, image, XML, number-formatting, permissions, and policy DTO/runtime helpers in one crate (`reqwest`, `codex-utils-image`, `quick-xml`, ICU, `globset`, `codex-execpolicy`). The next decoupling target is a broader DTO split, not dependency version dedupe.

## Merge Hotspots

- `codex-rs/tui/src/chatwidget.rs` must remain main’s split-module orchestration shell. Local behavior should live in `chatwidget/*` leaf modules, not in a restored monolith.
- Tool registry work currently has overlapping planning surfaces: older `codex-tools/src/tool_registry_plan.rs` and newer `codex-rs/core/src/tools/spec_plan.rs`. Collapse to one source of truth or clearly mark one transitional.
- Local tool features should be preserved by clean registry integration, not by keeping orphaned files that are no longer compiled.
- Generated files (`Cargo.lock`, config schema, app-server protocol schema, Bazel lock) should be regenerated only after source ownership is settled.

## Follow-Up Refactors

- Move more config-only and protocol-only DTOs out of runtime crates when they are used by broad crates such as `codex-config` or `codex-protocol`.
- Continue the config DTO split by moving `ReasoningEffort` or the full lightweight collaboration/config DTO group into a small crate, then switch config-only consumers away from `codex-protocol` imports.
- Add a dependency-dedupe lane that records `scripts\build-local-codex.ps1 -Mode Diagnose` and, when needed, `cargo tree -d --workspace --edges normal,build`. Track action-required duplicate count separately from known unavoidable duplicates so the signal stays useful.
- Extract shell/unified-exec/network proxy runtime ownership out of `codex-core` behind narrower crates or traits so core unit canaries do not compile the full runtime graph.
- Keep MultiAgentV2 tool definitions, implementation handlers, tool docs, and registry specs generated from or backed by one canonical source.
- Keep future feature work in separate crates/modules by default. Only add to `codex-core` when the feature truly requires core session ownership.
- Add a cheap dependency canary for broad crates that asserts `codex-config` does not depend on runtime-heavy crates such as `codex-network-proxy`, `rama-*`, or `starlark`.
- Keep the build/test scripts' release cleanup policy dep-info-aware: prune orphaned deps and disposable test executables, classify duplicate dependency versions, but do not delete active same-name hashed variants or known unavoidable duplicate-version cases.

## Verification Plan

- Run `git diff --check` and fix whitespace before staging the merge.
- Stage all required new spec/handler files and rerun a conflict-marker scan.
- Run `scripts\test-local-codex-release.ps1 -Package codex-network-proxy-config`.
- Run `scripts\test-local-codex-release.ps1 -Package codex-config-types` for slow-mode default and wire-format checks.
- Run `scripts\test-local-codex-release.ps1 -Package codex-config -Filter network`.
- Run a narrower dependency canary with `cargo tree -p codex-config -i codex-network-proxy` and expect no matching package in the graph.
- After tool registry and MultiAgentV2 fixes, rerun `scripts\test-local-codex-release.ps1 -Package codex-core -Lib -Filter multi_agents`.
