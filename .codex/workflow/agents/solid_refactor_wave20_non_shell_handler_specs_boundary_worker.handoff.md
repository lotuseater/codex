# SOLID Refactor Wave 20 Non-Shell Handler Specs Boundary Worker

## Result

- Removed direct `codex_tools` imports from the non-shell handler/spec surface that could be moved without manifest edits.
- Added narrow registry-domain exports in `codex-tool-registry-api` for pure tool specs, discovery entries, plugin-install request DTOs, and handler search metadata.
- Kept execution-only test stubs on `codex-tool-execution-api`.

## Root Wiring Needed

- `codex-rs/core/src/tools/handlers/plan_spec.rs` still re-exports `codex_tools::create_update_plan_tool`.
- Moving that builder cleanly requires manifest/root wiring because the builder depends on the agent-policy prompt source. This worker did not edit manifests.

## Explicitly Skipped

- `codex-rs/core/src/tools/handlers/shell_spec.rs` still imports `codex_tools`; shell/unified-exec files are wave19-owned and out of scope for this worker.
- Existing dirty work outside this boundary was left untouched and unstaged.

## Verification

```powershell
rg -n "codex_tools" codex-rs/core/src/tools/handlers
scripts/check-cargo-dependency-boundaries.ps1 -SolidRefactor -Json
git diff --check -- codex-rs/core/src/tools/handlers codex-rs/tools-domain .codex/workflow/agents/solid_refactor_wave20_non_shell_handler_specs_boundary_worker.handoff.md
```

Observed:

- `rg` reports only `plan_spec.rs` (`root-wiring-needed`) and `shell_spec.rs` (`wave19-owned`).
- Solid refactor dependency boundary check passed with `violation_count: 0`.
- `git diff --check` passed; Git emitted only existing line-ending normalization warnings.
