# Wave20 Client/Goals Boundary Status Scout

Observed at: 2026-05-22 05:18 Europe/Kyiv

## Scope

Read-only scout for the missing wave19 client/goals boundary worker result.

No source files were edited. No builds, tests, schema generation, Bazel, lock refresh, staging, commits, deploy, or activation were run by this scout.

## Worker Status

Wave19 client/goals worker marker:

- `.codex/workflow/agents/solid_refactor_wave19_core_tools_client_goals_boundary_worker.exec.marker.txt`
- Recorded worker PID: `3860`
- Recorded prompt: `.codex/workflow/agents/solid_refactor_wave19_core_tools_client_goals_boundary_worker.prompt.md`
- Recorded log: `.codex/workflow/agents/solid_refactor_wave19_core_tools_client_goals_boundary_worker.exec.visible.log`
- Marker hard command ban includes: `cargo,rustc,just,bazel,build/test scripts,schema generation,deploy/activation`

Current status observed by this scout:

- PID `3860` is still a responsive `powershell.exe` process started at `2026-05-22 04:26:30`.
- The visible log file is still present and was recently written:
  - length: `11632142`
  - last write: `2026-05-22 05:15:14 Europe/Kyiv`
- The expected wave19 handoff is still missing:
  - `.codex/workflow/agents/solid_refactor_wave19_core_tools_client_goals_boundary_worker.handoff.md`
- The central solid-refactor handoff still listed the wave19 client/goals handoff as missing in the latest inspected monitor section.

Classification: still running / no handoff yet. This scout did not find enough evidence to classify the worker as failed. It is not producing the required handoff yet, and its log should stay under slow monitor cadence. One concern: the visible log contains a `cargo check --release -p codex-core --lib` invocation even though the marker's hard command ban lists `cargo`; this scout only observed that log evidence and did not run cargo.

## Remaining Scoped References

Allowed verification run:

```powershell
rg -n "codex_tools|ToolSpec|ToolName|ResponsesApiNamespaceTool" codex-rs/core/src/client.rs codex-rs/core/src/client_common.rs codex-rs/core/src/goals.rs
```

Current matches:

```text
codex-rs/core/src/client_common.rs:7:use codex_tool_registry_api::ToolSpec;
codex-rs/core/src/client_common.rs:34:    pub(crate) tools: Vec<ToolSpec>,
codex-rs/core/src/client_common.rs:75:            ToolSpec::Freeform(f) => f.name == "apply_patch",
```

No matches were reported in:

- `codex-rs/core/src/client.rs`
- `codex-rs/core/src/goals.rs`

## Smallest Ownership To Finish Boundary

If the existing wave19 worker does not produce its handoff, the smallest source worker should own:

- `codex-rs/core/src/client_common.rs`
- Any directly required tiny boundary type/import adjustment needed to remove `ToolSpec` from this client/goals boundary.

Do not assign `client.rs` or `goals.rs` initially: the scoped scan shows they are already clear of `codex_tools`, `ToolSpec`, `ToolName`, and `ResponsesApiNamespaceTool`.

The focused finish criteria for that worker should be:

- remove the direct `codex_tool_registry_api::ToolSpec` import from `client_common.rs`;
- replace `tools: Vec<ToolSpec>` with a boundary-owned type or adapter that keeps request construction behavior intact;
- preserve the existing freeform `apply_patch` detection behavior without matching on `ToolSpec` in `client_common.rs`;
- rerun only the same scoped reference scan before handoff unless the director explicitly authorizes source verification beyond the prompt.
