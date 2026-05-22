classification: accepted

files changed:
- codex-rs/tools-domain/tool-registry-api/src/lib.rs
- codex-rs/tools-domain/tool-registry-api/src/tool_discovery.rs
- .codex/workflow/agents/solid_refactor_wave21_tool_registry_api_repair_worker.handoff.md

checks run:
- `rg -n "create_file_outline_tool|create_search_text_tool|create_first_moves|CloseAgentToolOptions|ListAgentsToolOptions|ResponsesApiTool \{|defer_loading" codex-rs/tools-domain/tool-registry-api/src`
  - result: passed; stale individual context/first-moves exports and stale agent option exports are gone, and all matched `ResponsesApiTool` literals include `defer_loading`.
- `git diff --check -- codex-rs/tools-domain/tool-registry-api/src .codex/workflow/agents/solid_refactor_wave21_tool_registry_api_repair_worker.handoff.md`
  - result: passed; Git reported LF-to-CRLF working-copy warnings for touched Rust files only.
- `cargo check --manifest-path codex-rs\Cargo.toml --release -p codex-tool-registry-api *> logs/wave21-tool-registry-api-repair.log`
  - result: passed.

cargo log:
- `logs/wave21-tool-registry-api-repair.log`
- registry crate check passed: yes

remaining fallout for core-lib repair lane:
- None in `codex-tool-registry-api`; the narrow release compile check now finishes successfully.
