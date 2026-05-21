# solid_refactor_area_review_context_ops_worker Handoff

## Findings

1. Deleting `codex-rs/core/src/tools/handlers/replacement_shadow.rs` is safe for the current `codex-core` source tree.
   - `codex-rs/core/src/tools/handlers/mod.rs:5` declares `pub(crate) mod context_ops;` and has no `replacement_shadow` module declaration.
   - `codex-rs/core/src/tools/handlers/context_ops.rs:1-4` declares the live context-ops submodules: `execution`, `file_outline`, `search_text`, and `workflow_batch`; it does not route through replacement-shadow.
   - Focused `rg` for `replacement_shadow|ShellShadowRequest|maybe_compact_shell_output|maybe_spawn_shell_shadow` under `codex-rs/core` returned no remaining Rust/module references after the deletion, aside from the stale manifest dependency called out below.

2. `codex-rs/core/Cargo.toml` still has one dead dependency caused by the deleted replacement-shadow handler.
   - Dead now: `codex-rs/core/Cargo.toml:122` has `codex-replacement-shadow = { workspace = true }`.
   - The corresponding `codex-core` lock dependency is also still present at `codex-rs/Cargo.lock:2692` as `"codex-replacement-shadow",`.
   - Repo-wide focused `rg` only found replacement-shadow references in the workspace/package declarations, the replacement-shadow crate's own files, and `codex-rs/core/Cargo.toml:122`; no live `codex-core` Rust callsites remain.

3. `codex-context-ops-impl` is still required by live context-ops handlers and must not be removed from `codex-core` in this cleanup.
   - Required dependency: `codex-rs/core/Cargo.toml:94` has `codex-context-ops-impl = { workspace = true }`.
   - `codex-rs/core/src/tools/handlers/context_ops.rs:57-59` routes live tool calls to `file_outline::handle` and `search_text::handle`.
   - `codex-rs/core/src/tools/handlers/context_ops/search_text.rs:41-52` uses `DEFAULT_MAX_FILES`, `clamp_max_files`, `DEFAULT_MAX_MATCHES_PER_FILE`, `clamp_max_matches_per_file`, and `combined_globs` from `codex_context_ops_impl`.
   - `codex-rs/core/src/tools/handlers/context_ops/search_text.rs:59-62` builds remote `rg` args through `codex_context_ops_impl::rg_args`.
   - `codex-rs/core/src/tools/handlers/context_ops/search_text.rs:82-92` parses remote `rg --json` output and runs local search through `codex_context_ops_impl`.
   - `codex-rs/core/src/tools/handlers/context_ops/file_outline.rs:28-32` uses `DEFAULT_MAX_OUTLINE_ITEMS` and `file_outline_from_bytes` from `codex_context_ops_impl`.
   - `codex-rs/Cargo.lock:2664` still lists `"codex-context-ops-impl",` in the `codex-core` dependency set, matching those live calls.

4. There are no explicit `codex-rs/core/BUILD.bazel` dependency entries to delete for either crate.
   - `codex-rs/core/BUILD.bazel:3-24` defines `codex_rust_crate(name = "core", crate_name = "codex_core", ...)` with data/test metadata only in that section.
   - `codex-rs/core/BUILD.bazel:54-62` lists only `extra_binaries`.
   - Focused `rg` for `context_ops_impl|replacement_shadow|codex_context_ops_impl|codex_replacement_shadow|context-ops|replacement-shadow` in `codex-rs/core/BUILD.bazel` returned no matches.
   - Still-required workspace Bazel target: `codex-rs/context-ops-impl/BUILD.bazel:3-6` defines `context_ops_impl` / `codex_context_ops_impl`.
   - Replacement-shadow workspace Bazel target: `codex-rs/replacement-shadow/BUILD.bazel:3-6` defines `replacement_shadow` / `codex_replacement_shadow`; it appears unused outside its own crate after the core handler deletion, but deleting the whole crate is broader than the minimal `codex-core` cleanup.

## Root-Owned Next Action

Remove only `codex-rs/core/Cargo.toml:122` (`codex-replacement-shadow = { workspace = true }`) from `codex-core` for this slice, keep `codex-rs/core/Cargo.toml:94` (`codex-context-ops-impl = { workspace = true }`), and do not edit `codex-rs/core/BUILD.bazel` for these crates because it has no explicit entries.

After that source/manifest cleanup, root should refresh the generated dependency state that tracks the `codex-core` dependency edge, including dropping the `codex-rs/Cargo.lock:2692` `codex-replacement-shadow` entry from the `codex-core` dependency list and running the repo-required Bazel lock update/check plus focused release-profile verification. This worker did not run builds, tests, Cargo, just, or Bazel by request.
