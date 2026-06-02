---
name: new-crate
description: Use when creating a new Rust workspace crate, or when you find yourself about to add a significant new concept/feature to codex-core and should extract it instead.
disable-model-invocation: true
---

# New Crate Checklist

> **First ask:** does this belong in an existing crate other than `codex-core`?  
> If yes, put it there. If no existing crate fits, follow the steps below.  
> **Resist adding to `codex-core`** — it is already large; new concepts get their own crate.

## Steps

1. **Choose a name** — `codex-<thing>` (e.g. `codex-budget`, `codex-telemetry`). All crates are prefixed `codex-`.

2. **Create the crate skeleton:**
   ```
   codex-rs/<thing>/
     Cargo.toml    # package.name = "codex-<thing>", edition = "2024"
     src/lib.rs    # start with private modules; expose a minimal, explicit public API
   ```
   - Prefer private module files (`mod foo;`) and explicit `pub use` re-exports over making everything public by default.

3. **Add to workspace** — in `codex-rs/Cargo.toml` add `"<thing>"` to the `[workspace] members` array.

4. **Size discipline** — target modules under **500 LoC** (excluding tests). If a single file would exceed ~800 LoC, split into sub-modules from the start.

5. **Bazel integration** — if the crate uses any of the following, update `codex-rs/<thing>/BUILD.bazel`:

   | Pattern | BUILD.bazel attribute |
   |---------|----------------------|
   | `include_str!(…)` / `include_bytes!(…)` | `compile_data` |
   | `sqlx::migrate!(…)` or other build-script file reads | `build_script_data` |
   | Test fixture files | add to the test target's `data` |

   After updating `BUILD.bazel`, run:
   ```sh
   just bazel-lock-update   # refresh MODULE.bazel.lock
   just bazel-lock-check    # confirm no lockfile drift
   ```

6. **Config types** — if the new crate adds types that appear in `ConfigToml` or any nested config struct, run:
   ```sh
   just write-config-schema
   ```

7. **Dependency inversion toward `codex-core`** — if your crate needs to interact with core orchestration, prefer defining a small boundary trait in your crate and implementing it in a thin adapter, rather than importing large slabs of `codex-core` directly.

8. **Verify** (release-profile only):
   ```sh
   # Check it compiles
   powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode FastRelease

   # Run its tests
   powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-<thing>
   ```

## Anti-patterns to avoid

- Do not add a broad compatibility re-export from `codex-core` pointing at your new crate's types — fix the ownership boundary.
- Do not use `#[serde(default)]` or catch-all `pub use *` to paper over missing wiring.
- Do not add test helpers that depend on `codex-core` unless the test genuinely instantiates core runtime behaviour; prefer a small support crate based on protocol/domain fixtures only.
