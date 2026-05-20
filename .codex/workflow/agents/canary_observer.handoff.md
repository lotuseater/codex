# canary_observer Handoff

Status: completed read-only scan on 2026-05-20.

Launch marker: `.codex/workflow/agents/canary_observer.exec.marker.txt`

Scope: read-only SOLID canary observer. No source files, manifests, lockfiles,
Git state, Cargo/Just builds, or formatter commands were touched. The only edit
made by this lane is this handoff file.

## Commands run

Initial required reads:

```powershell
Get-Content -Raw .codex/workflow/solid-refactor-delegation-director-plan.md
Get-Content -Raw .codex/workflow/solid-refactor-subagent-contract.md
Get-Content -Raw .codex/workflow/worker-delegation-commit-protocol.md
Get-Content -Raw .codex/workflow/solid-refactor-handoff.md
Get-Content -Raw .codex/prototypes/check-core-boundaries.ps1
```

Boundary canary and summaries:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\prototypes\check-core-boundaries.ps1
```

```powershell
$PSNativeCommandUseErrorActionPreference = $false
$out = & powershell -ExecutionPolicy Bypass -File .codex\prototypes\check-core-boundaries.ps1 2>&1
$violations = $out | ForEach-Object { $_.ToString() } | Where-Object { $_ -match "^(\\.\\codex-rs\\|codex-)" }
"EXIT=$LASTEXITCODE"
"TOTAL=$($violations.Count)"
$violations | ForEach-Object {
    if ($_ -match " directly depends on forbidden crate ") { "dependency:direct" }
    elseif ($_ -match " transitively depends on forbidden crate ") { "dependency:transitive" }
    elseif ($_ -match "matches forbidden pattern '([^']+)'") { "source-pattern:$($Matches[1])" }
    else { "other" }
} | Group-Object | Sort-Object Name | ForEach-Object { "$($_.Name)=$($_.Count)" }
"VIOLATIONS"
$violations
```

```powershell
$PSNativeCommandUseErrorActionPreference = $false
$out = & powershell -ExecutionPolicy Bypass -File .codex\prototypes\check-core-boundaries.ps1 2>&1
$violations = $out | ForEach-Object { $_.ToString() } | Where-Object { $_ -match "^(\\.\\codex-rs\\|codex-)" }
$violations | Where-Object { $_ -match "matches forbidden pattern" } | ForEach-Object {
    if ($_ -match "^(?<path>.+?) matches forbidden pattern '(?<symbol>[^']+)'") { "$($Matches['path'])`t$($Matches['symbol'])" }
} | Group-Object | Sort-Object @{Expression='Count';Descending=$true}, Name | Select-Object -First 20 | ForEach-Object { "$($_.Count)`t$($_.Name)" }
"DEPENDENCIES"
$violations | Where-Object { $_ -match "transitively depends|directly depends" }
```

Worker note scans:

```powershell
Get-ChildItem .codex\workflow\agents -File | Select-Object Name,Length,LastWriteTime
rg -n "SOLID|risk|concern|blocked|blocker|manifest|Cargo|lock|Bazel|workspace|path dependency|re-export|shim|protocol|codex_app_server_protocol|codex-core|codex_core|thread_store|LocalThreadStore|AuthMode|ThreadHistoryBuilder|TurnStatus|elicitation|DAB" .codex\workflow\agents -g "*.md"
Get-Content -Raw .codex\workflow\agents\canary_observer.handoff.md
Get-Content -Raw .codex\workflow\agents\app_catalog_followup.handoff.md
Get-Content -Raw .codex\workflow\agents\auth_boundary.handoff.md
Get-Content -Raw .codex\workflow\agents\mcp_elicitation_boundary.handoff.md
Get-Content -Raw .codex\workflow\agents\thread_projection_boundary.handoff.md
Get-Content -Raw .codex\workflow\agents\thread_store_boundary.handoff.md
Get-Content -Raw .codex\workflow\agents\dab_availability_worker.handoff.md
Get-ChildItem .codex\workflow\agents -File -Filter "*.handoff.md" | Select-Object -ExpandProperty Name
Get-Content -Raw .codex\workflow\agents\auth_boundary.prompt.md
Get-Content -Raw .codex\workflow\agents\mcp_elicitation_boundary.prompt.md
Get-Content -Raw .codex\workflow\agents\thread_projection_boundary.prompt.md
Get-Content -Raw .codex\workflow\agents\thread_store_boundary.prompt.md
Get-Content -Raw .codex\workflow\agents\app_catalog_followup.prompt.md
Get-Content -Raw .codex\workflow\agents\dab_availability_worker.prompt.md
Get-Content -Raw .codex\workflow\solid-refactor-handoff.md
```

One attempted wildcard `rg` probe against `.codex\workflow\agents\*.handoff.md`
failed on Windows with an IO error and was not used for counts.

## Current leak counts

The canary exits `1`. Parsed current violations: 23.

- Direct forbidden crate dependencies: 0
- Transitive forbidden crate dependencies: 1
- Source-pattern leaks: 22

Source-pattern counts:

- `codex_app_server_protocol::`: 8
- `LocalThreadStore`: 5
- `LocalThreadStoreConfig`: 4
- `thread_store_from_config`: 3
- `InMemoryThreadStore`: 2

Dependency violation:

- `codex-core` transitively depends on forbidden crate
  `codex-app-server-protocol`.

## Top boundary blockers

Highest-value production blockers:

- `codex-rs/core/src/thread_manager.rs`: references both
  `codex_app_server_protocol::` and concrete local store symbols
  `LocalThreadStore` / `LocalThreadStoreConfig`.
- `codex-rs/core/src/session/mod.rs`: references both
  `codex_app_server_protocol::` and `LocalThreadStore`.
- `codex-rs/core/src/client.rs`, `compact_remote.rs`,
  `mcp_tool_call.rs`, and `realtime_conversation.rs`: remaining production
  `codex_app_server_protocol::` references in `codex-core`.

Concrete thread-store blockers:

- `LocalThreadStore`: `agent/control_tests.rs`, `session/mod.rs`,
  `session/tests.rs`, `session/tests/guardian_tests.rs`,
  `thread_manager.rs`.
- `LocalThreadStoreConfig`: `agent/control_tests.rs`,
  `session/tests.rs`, `session/tests/guardian_tests.rs`,
  `thread_manager.rs`.
- `InMemoryThreadStore`: `session/tests.rs`, `thread_manager_tests.rs`.
- `thread_store_from_config`: `prompt_debug.rs`,
  `thread_manager_tests.rs`, `tools/handlers/multi_agents_tests.rs`.

Protocol DTO blockers:

- `codex_app_server_protocol::`: `client.rs`, `client_tests.rs`,
  `compact_remote.rs`, `mcp_tool_call.rs`, `realtime_conversation.rs`,
  `session/mod.rs`, `session/tests.rs`, `thread_manager.rs`.

## Active worker note risks

No active worker handoff currently reports completed code changes. The active
handoffs are still launch/queue stubs, and the DAB worker note reports a stale
interactive launch plus an exec relaunch marker. No worker-introduced source
risk is directly observable from the notes.

Coordination risks still visible from active prompts/notes:

- Several lanes (`app_catalog_followup`, `auth_boundary`,
  `mcp_elicitation_boundary`, and `thread_projection_boundary`) are all
  extracting app-server-protocol ownership. Without root integration, they can
  converge on overlapping DTO/port crates or temporary shim/re-export patterns
  that preserve the dependency direction leak.
- Multiple prompts expect root-owned manifest/Bazel/lockfile wiring. Worker
  slices may be intentionally incomplete until root wires crates, so root
  should batch manifest ownership rather than letting each lane solve it
  locally.
- The thread-store lane and core protocol lanes both touch the same central
  blockers (`session/mod.rs` and `thread_manager.rs`). Order matters: abstract
  store ports and protocol DTO ownership should be coordinated so core does not
  trade one direct dependency for another.
- The DAB worker is separate from the current core/protocol canary, but its
  prompt allows a canary-first fix under the shared protocol. It should avoid
  central orchestration churn while the boundary lanes are active.
