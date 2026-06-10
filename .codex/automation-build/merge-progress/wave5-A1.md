# Wave-5 build-fix Worker A1 progress

B1 self-assert PASS: repo root = C:/Users/Oleh/Documents/GitHub/open_ai/codex
All edits UNSTAGED, no git mutations run.

## session.rs (DONE, 4 fixes)
- :209 E0560 ThreadConfigSnapshot has no `environments` field -> set `cwd: self.cwd().clone()`
  (ThreadConfigSnapshot has FIELD `cwd: AbsolutePathBuf`; `self.cwd()` -> &AbsolutePathBuf via legacy_fallback_cwd). Shape 2.
- :560 E0609 Config has no `extra_config` -> `extra_config: None` in CreateThreadParams.
  DECISION: ExtraConfig is empty placeholder, Config carries no payload, every other call site uses None. Brief item 6.
- :963/965 E0308 -> `effective_agent_max_threads` is FORK-LOCAL (OgelGbuzax), returns io::Result<Option<usize>>;
  upstream auto-merged callsite to `.unwrap_or(usize::MAX)` (wrong for that return type).
  Fix: `.ok().flatten().unwrap_or(usize::MAX)` preserving fork fallback-to-MAX intent.

## handlers.rs (DONE, 5 fixes + 1 import)
- :113 destructure ThreadSettingsOverrides missing `cwd` -> added `cwd: _` (cwd folded into environments).
- :158 E0062 dup `environments` in SessionSettingsUpdate init -> removed `environments: None` (real one from override).
- :171 E0599 snapshot.cwd() -> field `snapshot.cwd.clone()` (ThreadConfigSnapshot.cwd is FIELD).
- :206 E0027 Op::UserInput destructure missing `environments` -> added it AND wired canonically.
  upstream did `updates.environments = environments;` (same type), but FORK's SessionSettingsUpdate.environments is
  Option<TurnEnvironmentSelections> (wrapper) consumed by apply() session.rs:286. CONVERT: wrap turn-scoped envs
  (Option<Vec<TurnEnvironmentSelection>>) with session CURRENT cwd as legacy_fallback_cwd via
  TurnEnvironmentSelections::new(cwd, environments). Preserves fork wrapper + upstream turn-scoped-envs; cwd unchanged.
- Added import `use codex_protocol::protocol::TurnEnvironmentSelections;`

## codex_handle.rs (DONE, 4 fixes)
- :309/314 E0560 SessionConfiguration has no `cwd` + E0308 -> removed `cwd: config.cwd.clone()`,
  changed `environments: environment_selections.to_selections()` (Vec<TurnEnvironmentSelection>) to
  `environments: TurnEnvironmentSelections::new(config.cwd.clone(), environment_selections.to_selections())`. Shape 1/3.
- :337 E0061 Session::new takes 24 args, 23 supplied -> missing `thread_extension_init: ExtensionDataInit`
  (15th param, between extensions & agent_control). CodexSpawnArgs does NOT carry it (brief item 5) and spawn_internal
  has no other source -> pass `codex_extension_api::ExtensionDataInit::default()`, CONSISTENT with all thread_manager.rs
  root/spawn call sites.
- :503/510 E0308 thread_environment_selections returns Vec<TurnEnvironmentSelection> but `.environments.clone()` is
  the wrapper -> use accessor `.environment_selections().to_vec()`.
- CROSS-WORKER NOTE (A2): CodexSpawnArgs field set UNCHANGED. I did NOT add thread_extension_init back to the struct;
  only fixed the Session::new CALL with ExtensionDataInit::default(). A2 stays consistent (field is environment_selections).
- Imports OK: TurnEnvironmentSelection(s) in scope via `use super::*;` (session/mod.rs re-exports them).

## session_settings.rs (DONE, 2 fixes)
- :122 `state.session_configuration.cwd.clone()` and :127 `updated.cwd.clone()` E0615 (cwd is now a METHOD)
  -> `.cwd().clone()` both. Shape 1.

## config/mod.rs (DONE, 1 fix)
- :120 E0432 `pub use codex_thread_store::ExtraConfig;` -> path invalid (codex_thread_store doesn't export it;
  canonical is codex_thread_store_api::ExtraConfig) AND it's a forbidden FOREIGN-type `pub use` (rule 6) AND
  now UNUSED (only consumer was session.rs:560 which I set to None; no other consumer in/out of crate).
  DECISION: REMOVE the import entirely (dead + forbidden re-export). Cleanest brief-compliant fix.

## VERIFY: DONE
`cargo check -p codex-core --release` => EXITCODE=0, "Finished release profile", zero error lines.
Deduped error grep over whole crate = empty. All A1 owned-file errors gone; crate compiles clean
(other workers' fixes also landed by final run). Warnings only.
