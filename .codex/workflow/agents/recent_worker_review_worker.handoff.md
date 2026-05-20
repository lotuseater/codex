# recent_worker_review_worker Handoff

Status: review complete on 2026-05-21.

Review snapshot: `a0ad874831` (`Record boundary dependency manifest handoff`).

## Findings

1. High: `codex-core` still imports `codex_app_server_protocol`, but
   `codex-rs/core/Cargo.toml` no longer declares `codex-app-server-protocol`
   as a direct dependency. Rust will not allow these imports through a
   transitive dependency, so the shared `codex-core` library remains blocked
   before any split test binary can compile. Current references include
   `codex-rs/core/src/mcp_tool_call.rs:29`,
   `codex-rs/core/src/session/mod.rs:54`,
   `codex-rs/core/src/session/tests.rs:79`, and
   `codex-rs/core/src/thread_manager.rs:19`; the dependency list around
   `codex-rs/core/Cargo.toml:63` jumps from `codex-api` to
   `codex-app-catalog-types`. The current handoff also says not to start the
   MCP elicitation / `ThreadHistoryBuilder` / `TurnStatus` cleanup yet
   (`.codex/workflow/solid-refactor-handoff.md:72`), so either restore the
   direct dependency temporarily or move those DTOs into a boundary crate before
   expecting compile verification to go green.

2. High: residual routing lost the old non-Windows gate for
   `request_permissions.rs`. The current wrapper includes it unconditionally
   (`codex-rs/core/tests/permissions.rs:14`), while the suite file contains
   POSIX shell commands such as `touch` and `printf`
   (`codex-rs/core/tests/suite/request_permissions.rs:348` and
   `codex-rs/core/tests/suite/request_permissions.rs:634`). The old
   `suite/mod.rs` gated this module with `#[cfg(not(target_os = "windows"))]`;
   restore that cfg on both the `#[path]` and `mod request_permissions` lines
   or the Windows release lane can fail once the shared compile blockers are
   cleared.

3. Medium: `.codex/workflow/solid-refactor-handoff.md` now contradicts itself
   about residual test routing. The latest section says
   `core_tests_residual_router_worker` finished and committed `d0a3390511`
   (`.codex/workflow/solid-refactor-handoff.md:9`), but the older live section
   still says removed permissions modules need residual routing
   (`.codex/workflow/solid-refactor-handoff.md:774`), still lists
   `hooks_mcp.rs`, `permissions_messages.rs`, and `request_permissions.rs` as
   residual unassigned modules (`.codex/workflow/solid-refactor-handoff.md:805`),
   and still queues `core_tests_residual_router_worker`
   (`.codex/workflow/solid-refactor-handoff.md:820`). This can relaunch a
   completed lane or hide the real remaining blockers.

## Open Questions And Assumptions

- I treated commits through the cleanup commits plus the subsequently completed
  worker handoffs visible at snapshot `a0ad874831` as current review context,
  because the worktree moved during this review.
- I did not treat `boundary_dependency_manifest_worker` editing
  `codex-rs/Cargo.toml` / `codex-rs/Cargo.lock` as an ownership violation,
  because the current root handoff names it as the manifest/dependency repair
  lane.
- The earlier residual-routing gap is closed in current state: a read-only
  routing scan found `missing_count=0` for `codex-rs/core/tests/suite/*.rs`.

## Suggested Next Worker Lanes

- `app_server_protocol_boundary_worker`: migrate or isolate the remaining
  `codex_app_server_protocol` imports used by `codex-core`, or explicitly
  restore the direct dependency until that boundary move is complete.
- `permissions_windows_cfg_worker`: restore cfg parity for
  `request_permissions.rs` and scan old `suite/mod.rs` cfg-gated modules
  against the split wrappers.
- `solid_handoff_cleanup_worker`: reconcile the latest residual-routing status
  with the stale lower sections of `solid-refactor-handoff.md`.

## Verification

- Read-only review only. Per prompt, I did not edit source files and did not run
  cargo, just, Bazel, build scripts, or tests.
- Read-only checks used: `git log`, `git show`, `git diff`, `git status`,
  `git ls-files -u`, `rg`, and file reads.

## Commit

- Handoff commit: pending at file-write time. Preflight before writing showed no
  unmerged files and no staged files; final hash must be read after the commit
  because a file cannot contain its own final Git commit hash without changing
  that hash.
