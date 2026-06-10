# Wave-5 build-fix Worker D progress

B1 self-assert: PASS (repo root = open_ai/codex).

## Edits applied (all UNSTAGED, no git run)

1. config_transforms.rs:~63 McpConfig init — added `legacy_apps_mcp_loader_enabled: true`
   (after `apps_enabled`). DEFAULT = `true`: legacy host-owned Apps MCP loader defaults
   on; runtime overlay (core/src/mcp.rs:51) flips it to false when a host extension
   contributes the server. mod_tests.rs sets it true too. With ASCII comment.

2. reducer.rs (analytics-appserver):~2377 CodexTurnEventParams init — added
   `codex_error_subreason: None` (after codex_error_kind, before http_status_code).
   Struct field type Option<String> (events.rs:581); no subreason source on the turn fact,
   so None is correct. With ASCII comment.

3. spawn.rs:18 — `meta_line.meta.thread_source` -> `.thread_source.clone()`
   (ThreadSource lost Copy: now has Feature(String)). Formatter reflowed the tuple.

4. residency.rs:55 — `.effective_agent_max_threads(MultiAgentVersion::V2)` now returns
   io::Result<Option<usize>>. Old `.unwrap_or(usize::MAX)` treated it as Option.
   Fix: `?` (propagate io err; fn returns CodexResult, sibling spawn.rs:232 does same)
   then `.unwrap_or(usize::MAX)` to keep fork fallback-to-MAX intent.

5. plan_mode.rs:~256 realtime_text_for_event match — added `EventMsg::SubAgentActivity(_)`
   to the `=> None` (non-rendered) group, next to Collab* family. Fork event, no-op
   passthrough consistent with neighbors. Variant NOT deleted.

## Verify status — DONE
- `cargo check -p codex-analytics-appserver --release` => EXITCODE=0 (clean; warnings only).
  reducer.rs error gone.
- `cargo check -p codex-core --release` => only ONE error remains and it is NOT mine:
  `E0432 unresolved import codex_thread_store::ExtraConfig` in config/mod.rs (Worker A1,
  brief item 6). All 5 of my owned-file errors (config_transforms / residency:56,58 /
  spawn:18 / plan_mode:175) are GONE — none of my files appear in the error output.

