# Wave-5 build-fix shared brief — codex-core (+analytics-appserver)

Merge `31d4cb92b8` (upstream 51b3cd51f6). `cargo check --workspace --release` is GREEN for all
crates EXCEPT `codex-analytics-appserver` (1 err) and `codex-core` (42 errs). Every error =
"merge dropped/changed an upstream symbol the fork still uses." Fix to COMPILE under `--release`
while PRESERVING fork behavior. `#[cfg(test)]` errors are Phase-E (release check hides them) — do
NOT chase test-only code.

## Universal rules (EVERY worker)
1. **B1 self-assert FIRST:** run `git rev-parse --show-toplevel`; it MUST equal
   `C:/Users/Oleh/Documents/GitHub/open_ai/codex`. If not, ABORT and report — do not edit.
   (Do NOT check `git ls-files -u`; the merge is already committed, there are no unmerged files.)
2. **Edit ONLY your owned files** (absolute paths given in your prompt). Reading any other file is
   fine and encouraged. Run **NO git** mutations. Leave all edits **UNSTAGED**.
3. **Run cargo from `codex-rs/`** (toolchain 1.95.0 + /DEBUG:NONE link-arg). To check your slice:
   `cargo check -p codex-core --release` (or `-p codex-analytics-appserver`). Iterate until your
   owned files' errors are gone. The ORCHESTRATOR runs the authoritative final foreground gate —
   your local "looks clean" is necessary but not final.
4. **Union-preserve:** NEVER delete fork logic to make it compile. If a fork symbol's upstream
   counterpart changed shape, ADAPT the fork code to the new shape, keeping the fork's behavior.
5. **Bank-as-you-go:** write progress + decisions to your own file
   `.codex/automation-build/merge-progress/wave5-<worker>.md` incrementally (not only at the end).
   If you near ~150k tokens, append a short handoff there FIRST, then continue.
6. Never touch lines mentioning `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` / `CODEX_SANDBOX_ENV_VAR`.
   No cross-crate `pub use` re-export of a FOREIGN type (a crate re-exporting its OWN type is fine).
7. ASCII only in code comments you add. Do not reformat unrelated code.

## Canonical post-merge shapes (DECIDED — apply consistently across workers)

1. **SessionConfiguration** (`core/src/session/session.rs:~133`): FIELD
   `environments: TurnEnvironmentSelections`. `cwd` is now a **METHOD**:
   `pub(super) fn cwd(&self) -> &AbsolutePathBuf { &self.environments.legacy_fallback_cwd }`.
   (Also a method returning the environments list at ~168.)
   - field read `cfg.cwd` → method `cfg.cwd()` (returns `&AbsolutePathBuf`; clone/deref as old code did).
   - init `SessionConfiguration { cwd: X, .. }` → `SessionConfiguration { environments: <TurnEnvironmentSelections>, .. }`.
   - pattern `SessionConfiguration { cwd, .. }` → mention `environments` (or use `..`).

2. **ThreadConfigSnapshot** (`thread/thread-manager-api/src/lib.rs:31`): FIELD
   `pub cwd: AbsolutePathBuf` (NO `environments` field, NO `cwd()` method).
   - `snapshot.cwd()` → `snapshot.cwd` (field). init `{ environments: X }` → `{ cwd: <AbsolutePathBuf> }`.

3. **TurnEnvironmentSelections** (`protocol/src/protocol.rs:~180`):
   `{ pub legacy_fallback_cwd: AbsolutePathBuf, pub environments: Vec<TurnEnvironmentSelection> }`.
   - wrap a cwd: `TurnEnvironmentSelections { legacy_fallback_cwd: cwd, environments: vec![] }`.
   - extract a cwd: `selections.legacy_fallback_cwd`.

4. **Op** (protocol): the variant at the error site gained `environments: Option<TurnEnvironmentSelections>`
   (it may still have `cwd: Option<AbsolutePathBuf>` too). READ the variant def; set `environments`
   (usually `None`, or the fork's existing selections) preserving fork intent.

5. **CodexSpawnArgs** (`core/src/session/codex_handle.rs`): READ its CURRENT field list. The field
   `thread_extension_init` is GONE from CodexSpawnArgs (upstream removed/relocated extension init).
   It has `environment_selections: Vec<TurnEnvironmentSelection>`. Fix initializers by removing the
   orphaned `thread_extension_init: ...` line once you confirm the struct lacks it. CAUTION: a
   DIFFERENT options/builder struct in `thread_manager.rs:~178-182` legitimately HAS
   `thread_extension_init` — don't confuse the two.

6. **ExtraConfig**: `pub struct ExtraConfig {}` (empty) in crate `codex-thread-store-api`
   (`codex_thread_store_api::types::ExtraConfig`). `StoredThread.extra_config: Option<ExtraConfig>`.
   The failing import `codex_thread_store::ExtraConfig` (config/mod.rs:120) must resolve from the
   correct crate/path — check what `codex_thread_store` actually re-exports; if it doesn't, import
   `codex_thread_store_api::ExtraConfig` (or wherever it's defined). `Config` (config_struct) has NO
   `extra_config` field — `config.extra_config` (session.rs:560) is merge-orphaned; reconcile
   minimally: the empty ExtraConfig carries no data, so source it from the thread snapshot/StoredThread
   if that's the intent, else default to `None`, preserving compile + fork intent (note the choice in
   your progress file so the orchestrator can sanity-check).

## Owner map (exactly one worker per physical file)
- **A1** session.rs, session/handlers.rs, session/codex_handle.rs, session/session_settings.rs, config/mod.rs
- **A2** codex_thread.rs, codex_delegate.rs, session/mod.rs, thread_manager.rs
- **B**  compact_remote.rs, compact_remote_v2.rs, task_memory.rs, session/context_budget.rs, config/config_loaders.rs
- **C**  tools/handlers/multi_agents_v2/interrupt_agent.rs, tools/registry.rs
- **D**  config/config_transforms.rs, agent/control/residency.rs, agent/control/spawn.rs, session/turn/plan_mode.rs, analytics-appserver/src/reducer.rs
