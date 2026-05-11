# V8 / code-mode decoupling options

## Current state

- **`codex-code-mode`** (code-mode/Cargo.toml:29) unconditionally depends on `v8`, pulling in deno_core_icudata, icu_locale_core, icu_provider, and the ICU stack (~45 MB uncompressed).
- **`codex-v8-poc`** (v8-poc/Cargo.toml:18) also unconditionally depends on `v8`.
- **`codex-core`** (core/Cargo.toml:30) unconditionally depends on `code-mode`; rollout-trace (rollout-trace/Cargo.toml:17) does the same.
- **`codex-code-mode-spec`** is pure data—no v8 or code-mode deps (code-mode-spec/Cargo.toml).
- Core and rollout-trace source contain ~30 and ~11 files respectively that reference code-mode; none directly import v8 symbols.

## Proposed [features] shape

Add to `code-mode/Cargo.toml`:
```toml
[features]
default = ["v8-integration"]
v8-integration = ["v8", "deno_core_icudata"]
sandbox = ["v8/v8_enable_sandbox"]
```

Add to `v8-poc/Cargo.toml`:
```toml
[features]
default = ["v8-runtime"]
v8-runtime = ["v8"]
sandbox = ["v8/v8_enable_sandbox"]
```

Default behavior (CLI binary) unchanged. A consumer can then build with `--no-default-features` to skip V8.

## Downstream impact if disabled

- Code that directly calls `codex_code_mode::execute()` will fail to link if v8 feature is disabled (the execution engine is compiled away).
- Serialization layer (codex-code-mode-spec) remains available for spec/metadata operations.
- Any CLI subcommand that routes to code-mode tool handlers will panic at runtime if V8 is disabled but the handler is invoked.

## Binary size delta (rough)

V8 linkage typically adds 60–90 MB to a stripped release binary on x86_64. Snapshot pending; see docs/dep-snapshot.md § release-artifact families.

## Blockers preventing no-edit ship

- **Type re-export:**`code-mode/src/lib.rs` re-exports types from core (e.g., `Isolate`, `Snapshot`) unconditionally; feature-gating requires conditional pub paths.
- **CLI main() coupling:** `codex-cli` main hard-imports code-mode handlers without feature dispatch; must add a conditional init block.
- **Cargo.lock pin:** `v8 = 0.146.4` is locked; version bumps (if v8 releases a patch) require manual Cargo.lock edit or `cargo update -p v8`.

Status: roadmap, no implementation in this slice.
