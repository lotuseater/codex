# Local Fork Feature Inventory And Merge Health Checklist

Date: 2026-05-15

This is the durable inventory of local fork features that should be checked when `main` is merged into `slow-context-budget-mode` or when a large refactor touches their owner modules.

## Scope

- Base used for this audit: `6d65686313d484db0bb1212cf1a8e1915282024d` (`git merge-base HEAD origin/main` on 2026-05-15).
- Range audited: `6d65686313..HEAD`.
- Commit authors in the audited local range: `OgelGbuzax`.
- Merge commits were treated as integration points, not separate features.

## Merge-Time Checklist

Before or during a recurring `main` merge:

- Run `git log --oneline $(git merge-base HEAD origin/main)..HEAD` and update this document if new local features were added.
- Re-check each feature family below when conflicts touch its owner paths.
- Prefer extracting owner crates/modules before resolving repeated conflicts in broad files such as `codex-core`, `codex-tui`, app-server protocol schemas, or workspace manifests.
- Run the focused health checks listed here before the final FastRelease build/deploy.

## Feature Families

| Feature family | Representative commits | Owner surfaces | Merge-time health checks |
| --- | --- | --- | --- |
| Local fork UX/startup marker and Plan-mode reasoning defaults | `768a348521`, `af57c95123`, `838be5fb74`, `cff65c4b2d`, `cd4e625c9e` | `codex-rs/tui`, model/reasoning config paths | TUI snapshots, Plan-mode tests, local startup smoke |
| Tool/context token saving and context budget routing | `c91b0d4131`, `a2231cae35`, `6601892d54`, `81018cd450`, `283dd25d63`, `04a5d81cb9` | `codex-rs/core`, `codex-rs/tui`, `codex-rs/app-server-protocol`, `codex-rs/context-pack` | App-server protocol schema generation/tests, core release filters, TUI context-budget tests |
| Operation cache bridge and cache/status APIs | `f10106165a`, `2c77068078`, `770ea01e7d`, `af98b63b47`, `f82a8e7552` | `codex-rs/operation-cache`, core tool dispatch/cache, app-server protocol schemas, operation-cache scripts | `scripts/test-operation-cache.ps1`, `scripts/test-operation-cache-runtime.ps1`, `codex-operation-cache` release tests |
| Session limit footer, window/reset telemetry, and footer cache UI | `c78a6f0c84`, `3cc2f8c4ac`, `6b0ff3064e`, `3552baa5e4` | `codex-rs/tui/src/chatwidget/session_limit_footer.rs`, footer snapshots, telemetry paths | `scripts/test-session-limit-footer.ps1`, targeted `codex-tui` snapshot tests |
| Self-review system and task-memory preservation | `055fe77508`, `38ff5d8528`, `4ed3fc5645`, `f92193a30d`, `cdb3967dc9`, `6247ee7963`, `e599813b28`, `89285b438b` | `codex-rs/self-review`, `codex-rs/task-memory`, `codex-rs/core/src/task_memory.rs`, `codex-rs/tui` queue/input paths, `.codex/hooks/prototype_first_hint.py` | `codex-self-review`, `codex-task-memory` if changed, targeted TUI queue/restore tests |
| MultiAgentV2 supervision and lifecycle tools | `088d1790a8`, `ed7cc676f1`, `a2f54893de`, related handler commits | `codex-rs/core/src/tools/handlers/multi_agents_v2`, `codex-rs/tools`, `codex-rs/tui/src/multi_agents.rs` | `scripts/run-multiagent-v2-canary.ps1`, core tool registry/spec tests, TUI multi-agent snapshots |
| First-moves, logic-guided routing, and memory context | `e0ed836656`, `c2b4855f16`, `4158f63fd6` | `codex-rs/first-moves`, `codex-rs/repo-context-scout`, `codex-rs/reasoning-logic`, `scripts/context-scout-map.ps1` | `codex-first-moves`, optional `codex-reasoning-logic`, first-moves shadow/hit stats |
| Context replacement/shadow tools and context reducer experiments | `80c45c7523`, `7b3584afad`, `8506fd5610`, `fdbe9b4ba5`, `769e9b906f` | `codex-rs/context-ops-impl`, `codex-rs/replacement-shadow`, `codex-rs/repo-context-scout`, replacement-shadow scripts | `scripts/run-replacement-shadow-canaries.ps1`, context-ops tests |
| Desktop Automation Bridge helpers | `9e388c9eb7`, `0d2527a7b5` | `codex-rs/desktop-automation`, core/tool schema DAB handlers, visual skills | `codex-desktop-automation`; live DAB canary for GUI-impacting changes |
| Blackboard coordination | `34c8e537c0`, `21700e4565`, integrated by `be3880b3b5`; owner-crate extraction pending commit | `codex-rs/blackboard`, thin adapter in `codex-rs/core/src/session/blackboard.rs` | `codex-blackboard`, then MultiAgentV2 canary if core session wiring changes |
| Config/dependency boundary refactors | `d68b1cdcb1`, `2fb4ccf390`, `567ef11aab` | `codex-rs/config`, `config-types`, `permission-types`, `git-types`, `thread-config-remote`, `model-provider-info`, `features` | `scripts/check-cargo-dependency-boundaries.ps1 -Package codex-config`, `just write-config-schema`, app-server protocol schema tests |
| Local Windows release/build workflow | `51b484c00a`, `e0899ec64a`, `1552f64a5d`, `50a5af55d8`, `23efa6b3ba`, `9ac186cf90`, `abdad75351`, `f009a1870c`, `25aaf139bc`, `f8166737cd` | `.cargo/config.toml`, `codex-rs/.cargo/config.toml`, `scripts/build-local-codex.ps1`, `scripts/test-local-codex-release.ps1` | `scripts/build-local-codex.ps1 -Mode FastRelease`, `-Mode Progress`, `-Mode CleanSafe`, release-only test wrapper |
| Dependency dedupe and release graph auditing | `1ad31e1932`, `ae4f4dc94f`, `c07dd436e7` | Workspace manifests/locks, `docs/cargo-duplicate-dependency-audit.md`, `scripts/dep-snapshot.ps1` | `just bazel-lock-check`, duplicate audit diagnose mode, release build |
| Tool schema/code-mode decoupling and capability internalization | `26f6a641f4`, `ed537b8e19` | `codex-rs/tool-schema`, `code-mode-spec`, `agent-policy`, `codex-rs/tools` | Tool spec/registry tests, core tool router tests |
| Turn-diff tracking ownership | owner-crate extraction pending commit | `codex-rs/turn-diff`, thin adapters in `codex-rs/core/src/turn_diff_tracker.rs` and tool event handling | `codex-turn-diff`; core compile canary if event plumbing changes |
| Cognos operation ownership | owner-crate extraction pending commit | `codex-rs/cognos-ops`, thin adapter in `codex-rs/core/src/tools/handlers/cognos_ops.rs` | `codex-cognos-ops`; targeted core tool canary if runtime invocation changes |

## Current Modularization Contract

These extractions are part of the active merge-pressure reduction pass and should be preserved during recurring `main` merges:

- `codex-turn-diff`: owner crate for turn diff tracking; `codex-core` only converts protocol/apply-patch change DTOs.
- `codex-cognos-ops`: owner crate for Cognos operation logic; `codex-core` only adapts tool invocation/runtime context.
- `codex-blackboard`: owner crate for blackboard session runtime; `codex-core` only maps config/session-source data into blackboard options.
- `scripts/analyze-branch-conflict-surface.ps1 -IncludeWorkingTree`: merge-prep canary for finding committed and uncommitted local churn still living in upstream-hot files.

## Task-Memory Feature Check

The compaction/task-memory preservation feature is still present as of this audit:

- `codex-rs/task-memory/src/lib.rs` builds a `<task_memory>` item from substantive user prompts and the latest plan.
- `codex-rs/core/src/task_memory.rs` converts model history into task-memory input items and renders the task-memory response item.
- `codex-rs/core/src/session/mod.rs` injects task memory before sampling under token pressure and resets the throttle after compaction.

Merge-time risk: conflicts in compaction, sampling, or history mapping can silently weaken this feature. When those files are touched, verify `codex-task-memory` tests and a focused core sampling/compaction canary.
