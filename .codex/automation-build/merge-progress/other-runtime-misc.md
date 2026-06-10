# Slice: other-runtime-misc — merge progress  (COMPLETE)

Guards: toplevel OK, all 9 files unmerged (stages 1/2/3 present).

## DONE (all 9, markers removed)
- app-server-client/src/lib.rs — union: fork pub(crate) uses + upstream legacy_core mod. low
- codex-mcp/src/lib.rs — union: both pub use (codex_apps_tools_cache_status + mcp::codex_apps_mcp_server_config). low
- mcp-server/src/codex_tool_runner.rs — union: both EventMsg arms (CollabCompact/Restart + SubAgentActivity). low
- thread/thread-store/src/in_memory.rs — union: both counter fields. low
- rollout-trace/src/protocol_event.rs — union x4 blocks: fork CollabCompact*/CollabRestart* + upstream SubAgentActivity (enum variants, Serialize arms, match arms, exhaustive None arm). low
- cli/src/main.rs — STRUCTURAL take-fork: fork split main.rs into cli_types.rs + dispatch.rs (thin shell, 72 lines). base/upstream top-of-file imports+mods are BYTE-IDENTICAL (lines 1-85), so upstream added zero new top-level items; all upstream body changes live in dispatch.rs/cli_types.rs (NOT my slice). Wrote fork stage-2 verbatim. low
- network-proxy/src/config.rs — take-fork (import migration): fork re-export `pub use codex_network_proxy_config::*;` (wildcard, captures upstream additions). low
- tools/src/json_schema.rs — take-fork (import migration): fork re-exports codex_tool_schema::{6 syms}. Verified upstream pub surface == those 6, all present in tool-schema crate. low
- tools/src/tool_output.rs — take-fork (import migration): fork re-exports codex_tool_execution_api::{9 syms}. Verified upstream only exposes ToolOutput+JsonToolOutput (already in list); telemetry consts are fork additions present in tool-execution-api crate. low

## CROSS-FILE NOTE (not my slice)
- cli/src/dispatch.rs + cli/src/cli_types.rs (other resolver / prior conflict-reduction) must carry upstream's ~128 lines of new cli_main/clap logic. main.rs shell only re-exports them.
- Orphan files remant in cli/src (remote_control_cmd.rs, sandbox_setup.rs, state_db_recovery.rs) are not declared by fork main.rs mod list — pre-existing fork reorg state, not introduced here.

ALL MARKERS REMOVED.
