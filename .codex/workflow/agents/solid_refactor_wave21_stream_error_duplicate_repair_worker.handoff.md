classification: accepted

files changed:
- codex-rs/core/tests/client_stream.rs
- .codex/workflow/agents/solid_refactor_wave21_stream_error_duplicate_repair_worker.handoff.md

files inspected:
- codex-rs/core/tests/stream_error_allows_next_turn.rs
- codex-rs/core/tests/suite/stream_error_allows_next_turn.rs
- codex-rs/core/Cargo.toml

checks run:
- `Select-String -Path codex-rs\core\tests\client_stream.rs -Pattern "stream_error_allows_next_turn|suite/stream_error"`
  - result: no matches after repair; `client_stream.rs` no longer includes `suite/stream_error_allows_next_turn.rs`.
- `rg -n 'name = "stream_error_allows_next_turn"|path = "tests/stream_error_allows_next_turn.rs"' codex-rs/core/Cargo.toml`
  - result: focused Cargo test wiring is present at lines 441-442, so no root wiring change is needed.
- `rg -n "cli_stream|stream_error_allows_next_turn|suite/stream_error" codex-rs/core/tests/client_stream.rs codex-rs/core/tests/stream_error_allows_next_turn.rs codex-rs/core/tests/suite/stream_error_allows_next_turn.rs`
  - result: `client_stream.rs` includes only `suite/cli_stream.rs`; focused wrapper includes `suite/stream_error_allows_next_turn.rs`.
- `git diff --check -- @paths .codex/workflow/agents/solid_refactor_wave21_stream_error_duplicate_repair_worker.handoff.md`
  - result: exit 0; Git printed a CRLF normalization warning for the pre-existing dirty `codex-rs/core/tests/suite/stream_error_allows_next_turn.rs` file.

duplicate include remains: no

notes:
- No Cargo/Rust builds, `just`, Bazel, schema generation, lock refresh, release builds, deploy, or activation were run.
- Commit was left to root/commit steward because owned paths include existing untracked or modified work that should not be bundled blindly.
