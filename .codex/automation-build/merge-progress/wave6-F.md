# Wave-6 build-fix Worker F — progress

B1 self-assert: PASS (`git rev-parse --show-toplevel` = C:/Users/Oleh/Documents/GitHub/open_ai/codex).

## Root-cause findings (from git show upstream/main)

### A) Op::UserInput missing `environments: None`
- Op variant def: codex-rs/protocol/src/protocol/op.rs:62-80 ->
  `environments: Option<Vec<TurnEnvironmentSelection>>` with `#[serde(default, skip_serializing_if)]`.
  Default None preserves prior behavior. Canonical (protocol's own tests at op.rs/protocol.rs all set `environments: None`).
- Sites to fix:
  - memories/write/src/runtime.rs:262 (Op::UserInput in submit) -> add `environments: None,`
  - mcp-server/src/codex_tool_runner.rs:106 -> add `environments: None,`
  - mcp-server/src/codex_tool_runner.rs:157 -> add `environments: None,`

### B) image-generation E0432 codex_features
- Fork's extension.rs:10 `use codex_features::Feature;` is a FORK addition (upstream extension.rs has no
  such import; upstream uses ToolCall/ToolExecutor, fork uses ToolContributor + Feature::ImageGenExt gate
  at extension.rs:26/37). Upstream Cargo.toml ALSO lacks codex-features (its extension never imports it).
- Fix = add `codex-features = { workspace = true }` to fork ext/image-generation/Cargo.toml [dependencies]
  (the dep the FORK code needs; not present upstream because upstream code differs). Import unchanged.

### C) core_test_support (core/tests/common/) 5 errors in zsh_fork.rs
- zsh_fork.rs is an UPSTREAM file (exists upstream; fork last touched via merge 31d4cb92b8 + e6c470957d).
  NOT an orphan. It references:
  - crate::test_codex::{TestCodex, test_codex}  (E0432 x2) -> fork test_codex.rs EXISTS (exports
    TestCodexBuilder/TestCodex + fn test_codex) but fork lib.rs is MISSING `pub mod test_codex;`.
  - codex_features::Feature (E0432) -> Cargo.toml missing `codex-features` dep.
  - wiremock (E0433 x2) -> Cargo.toml missing `wiremock` dep.
- Upstream lib.rs declares: pub mod test_codex; pub mod zsh_fork; (and test_codex_exec, apps_test_server,
  hooks, responses, streaming_sse). Upstream Cargo.toml has codex-features + wiremock (+ many more).
- Fix (UNION-preserve, minimal): in fork lib.rs add `pub mod test_codex;` so crate::test_codex resolves.
  In fork Cargo.toml add `codex-features` and `wiremock` deps (both workspace deps).
  NOTE: this is a release check; zsh_fork/test_codex are #[cfg(test)]-adjacent integration support but the
  crate is a lib that compiles unconditionally, so deps/mods are needed for the release check to pass.

## Edits made
A) Added `environments: None,` at:
   - memories/write/src/runtime.rs (Op::UserInput submit)
   - mcp-server/src/codex_tool_runner.rs (run_codex_tool_session, line ~106)
   - mcp-server/src/codex_tool_runner.rs (run_codex_tool_session_reply, line ~157)
B) ext/image-generation/Cargo.toml: added `codex-features = { workspace = true }` after codex-extension-api.
C) core/tests/common/lib.rs: added `pub mod test_codex;` (between runtime_harness and tracing).
   core/tests/common/Cargo.toml: added deps:
     codex-features, wiremock (the immediate zsh_fork errors) PLUS the deps test_codex.rs needs
     once compiled: codex-exec-server, codex-extension-api, codex-login, codex-model-provider-info,
     codex-thread-store-api, codex-utils-absolute-path, futures. All are workspace deps already in
     root Cargo.toml; upstream's core_test_support declares the same (except thread-store-api, which
     the FORK's test_codex.rs uses via RecordingThreadStore/UnsupportedLiveThreadFactory — fork divergence).
   NOTE: enabling `pub mod test_codex;` was REQUIRED because zsh_fork.rs (merged-in) references
   crate::test_codex; fork's lib.rs had trimmed it out. test_codex.rs also refs crate::streaming_sse /
   ::find_codex_linux_sandbox_exe — both already satisfied by existing lib.rs re-exports, so no extra mods.

## Verify results
- Check #1 (3 crates: memories-write + mcp-server + image-generation-extension, --release):
  exit 0, NO errors in any of my 3 crates. A + B CONFIRMED GREEN.
  (core/src/config/config_loaders.rs E0425 x2 appeared but those are Worker E's files -> ignored.)
- core_test_support manifest: `cargo metadata` parses OK; all 9 added deps are valid workspace
  members (codex-features, wiremock, codex-exec-server, codex-extension-api, codex-login,
  codex-model-provider-info, codex-thread-store-api, codex-utils-absolute-path, futures).
- Combined check incl. core_test_support is rebuilding (blocked behind codex-core which has
  Worker E's in-flight errors). Will confirm once core compiles.
