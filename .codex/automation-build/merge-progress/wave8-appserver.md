# Wave 8 — codex-app-server build fix progress

## State
- B1 self-assert OK (toplevel = C:/Users/Oleh/Documents/GitHub/open_ai/codex)
- All 19 errors extracted from log (grep `^error\[`); matches the brief exactly.

## Error inventory (19)
Mechanical .cwd() -> .cwd (15):
- bespoke_event_handling.rs:772 (`.cwd().clone()`), 1192 (`.cwd().clone()`)
- thread_processor.rs:64 (`.cwd().as_path()`), 68 (`.cwd().display()`), 1198 (`.as_path()`), 1200 (`.clone()`), 2709 (`.as_path()`), 3449 (`.as_path()`), 4308 (`.clone()`)
- turn_processor.rs:531 (`.clone()`), 628 (`.to_path_buf()`)
- apps_processor.rs:62 (`.to_path_buf()`)
- thread_lifecycle.rs:631 (`.clone()`)
- thread_summary.rs:193 (`.clone()`), 198 (`.as_path()`)

Judgment (4, all turn_processor.rs):
- 525: E0599 environment_selections() gone
- 655: E0560 CodexThreadSettingsOverrides has no `environments` (only `cwd`)
- 465: E0063 Op::UserInput missing `environments`
- 678: E0063 protocol ThreadSettingsOverrides missing `cwd`

## Next steps
1. Read ThreadConfigSnapshot def + AbsolutePathBuf API (as_path/display/to_path_buf availability).
2. Read turn_processor.rs judgment regions + diff vs upstream/main.
3. Apply mechanical fixes; apply judgment fixes.
4. Foreground `cargo check -p codex-app-server --release` from codex-rs/.

## Decisions (verified against defs + core consumers)
- AbsolutePathBuf has as_path()/to_path_buf()/display() directly -> all 15 mechanical sites are pure `.cwd()` -> `.cwd`. Grep confirms the ONLY `.cwd()` occurrences in app-server/src are the 15 error sites, so scoped literal replace per file is safe.
- turn_processor.rs:525: snapshot no longer carries selections -> `environment_selections.unwrap_or_default()` (shape #2: empty Vec; cwd still becomes legacy_fallback_cwd below).
- turn_processor.rs:655 (CodexThreadSettingsOverrides): core's thread_settings_update maps `cwd` -> TurnEnvironmentSelections{legacy_fallback_cwd: cwd, environments: Vec::new()} (codex_thread.rs:323-328). So pass `cwd: environments.as_ref().map(|e| e.legacy_fallback_cwd.clone())` -- preserves preview/validation of the new fallback cwd.
- turn_processor.rs:465 (Op::UserInput): `environments: None`. Reason: core handler (session/handlers.rs:227-239) OVERWRITES updates.environments with session's CURRENT cwd as fallback when Op-level environments is Some -- passing the parsed selections here would discard the request's cwd override that already travels via thread_settings.environments.
- turn_processor.rs:678 (protocol ThreadSettingsOverrides): `cwd: None`. Core destructures `cwd: _` with comment "folded into environments.legacy_fallback_cwd; standalone override unused" (session/handlers.rs:114-117); environments field already carries everything.

## Edits applied
- perl scoped replace `.cwd()` -> `.cwd` in all 6 owned files (15 sites; verified 0 remaining occurrences in app-server/src).
- turn_processor.rs:465 Op::UserInput + `environments: None` (with rationale comment).
- turn_processor.rs:~525 `environment_selections.unwrap_or_default()`.
- turn_processor.rs:~655 CodexThreadSettingsOverrides `cwd: environments.as_ref().map(|e| e.legacy_fallback_cwd.clone())`.
- turn_processor.rs:~678 protocol ThreadSettingsOverrides + `cwd: None`.

## RESULT: DONE, GREEN
- `cargo check -p codex-app-server --release` (run from codex-rs/): "Finished release profile [optimized] target(s) in 29.88s", EXITCODE=0.
- 24 warnings, all pre-existing unused imports (request_processors.rs etc.) -- none introduced by these edits.
- Edits left UNSTAGED per brief. No git mutations performed.
- Note: working tree also carries pre-existing unstaged changes from earlier waves in external_agent_config_processor.rs / external_agent_session_import.rs (NOT touched by this wave).
