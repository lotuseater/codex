# Continuation Handoff — codex-cli build repair on `slow-context-budget-mode`

Date: 2026-05-27. Author: prior worker session (stopped mid-way at user request).

## TL;DR for the next session

We are repairing a bad upstream merge on branch `slow-context-budget-mode`. The merge
**dropped finished upstream features** from the `tui` / `exec` / `tui-render` /
`permission-types` / `core/config` / `app-server-client` layers while keeping the
branch's OWN local `context_budget_mode` feature (109 files). Both features must
**coexist** — do NOT delete local `context_budget_mode` code, and do NOT revert the
crate-split refactor described in `docs/current-project-architecture-solid-refactor-plan.md`
(the user explicitly wants that refactor kept; line 33 of that plan forbids adding new
compatibility re-export shims — restore dropped code into the NEW owning crates instead).

Goal: `cargo check -p codex-cli --release --jobs 2` exits 0. Tests are NOT in scope for
that check (lib build only). Do NOT commit.

### Environment / mechanics (IMPORTANT)
- EDIT THE MAIN REPO via ABSOLUTE paths rooted at
  `C:/Users/Oleh/Documents/GitHub/open_ai/codex/`. (This session ran from an isolated
  worktree on an OLD commit; all edits must target the main tree, never the worktree.)
- Run cargo from `Push-Location C:/Users/Oleh/Documents/GitHub/open_ai/codex/codex-rs`.
- HEAD = `9fa81a44aaeb...`; upstream/main = `aa184548b1ee...` (rev to copy real code from).
- Method to get real upstream code: `git show upstream/main:<path>` and diff vs HEAD.
- Save build output to a file and grep it (don't re-run repeatedly):
  `cargo check -p codex-cli --release --jobs 2 --message-format short 2>&1 | Out-File C:/Users/Oleh/check_errors.txt -Encoding utf8`
  then grep `error\[E\d+\]`.

## Progress this session (DONE — verified by the error count dropping 81 → 66)

All edits are in the MAIN tree, uncommitted. `git diff --stat`:
- `codex-rs/permission-types/src/lib.rs`  (+65)
- `codex-rs/core/src/config/mod.rs`        (+76, -1)

### DONE-1: permission-types — restored two dropped `materialize` methods
File `codex-rs/permission-types/src/lib.rs` (this is where HEAD moved `PermissionProfile`
and `FileSystemSandboxPolicy`; upstream had them in `protocol/src/{models,permissions}.rs`).
- Added `FileSystemSandboxPolicy::materialize_project_roots_with_workspace_roots(self, &[AbsolutePathBuf]) -> Self`
  immediately after the existing `materialize_project_roots_with_cwd` (~line 1103).
  It expands `FileSystemPath::Special { value: FileSystemSpecialPath::ProjectRoots { subpath } }`
  into one concrete `Path` entry per workspace root (resolving `subpath` via
  `AbsolutePathBuf::resolve_path_against_base`), and passes all other entries through.
  NOTE: upstream's version also handled a `GlobPattern` project-roots arm via helpers
  `parse_project_roots_glob_pattern` / `resolve_project_roots_glob_pattern` — those helpers
  DO NOT exist in HEAD (HEAD's cwd-variant also ignores glob project-roots), so the port
  faithfully mirrors HEAD's cwd-variant (Special arm only). This is a deliberate adaptation.
- Added `PermissionProfile::materialize_project_roots_with_workspace_roots(self, &[AbsolutePathBuf]) -> Self`
  before `from_runtime_permissions` (~line 3843). Faithful copy of upstream
  `protocol/src/models.rs:427`: delegates through `file_system.to_sandbox_policy()` then
  `ManagedFileSystemPermissions::from_sandbox_policy(&fs)`; `Disabled`/`External` pass through.

### DONE-2: core/config — restored dropped runtime workspace-roots + snapshot API on `Permissions`/`Config`
File `codex-rs/core/src/config/mod.rs`. Background: HEAD core was **independently refactored**
(its `Permissions` uses `permission_profile: Constrained<PermissionProfile>` +
`active_permission_profile` + `profile_workspace_roots`; upstream used
`permission_profile_state: PermissionProfileState` + `workspace_roots`). The merge dropped
the runtime `workspace_roots` field + several methods that TUI/exec (and HEAD's OWN core
tests) still call. `cargo check -p codex-core` (lib only) passed because tests weren't built.
Restored, written HEAD-native (against `Constrained<PermissionProfile>`, NOT upstream's
`permission_profile_state` — same observable behavior, different internals):
- Added field `workspace_roots: Vec<AbsolutePathBuf>` to `struct Permissions` (after `profile_workspace_roots`).
- Added field `pub workspace_roots: Vec<AbsolutePathBuf>` to `struct Config` (after `cwd`).
- Populated both in the production `Config { .. permissions: Permissions { .. } }` literal
  in `load_from_base_config_with_overrides` using the in-scope dedup'd `workspace_roots` local
  (`Config.workspace_roots: workspace_roots.clone()`, `Permissions.workspace_roots: workspace_roots`).
- Added `impl Permissions` methods: `set_workspace_roots`, `workspace_roots()`,
  `effective_permission_profile()` (materializes current profile against `self.workspace_roots`),
  `set_permission_profile_from_session_snapshot(PermissionProfileSnapshot)`,
  `replace_permission_profile_from_session_snapshot(PermissionProfileSnapshot)`.
- Added `impl Config` method `effective_workspace_roots()` (dedup of `self.workspace_roots`
  + `self.permissions.profile_workspace_roots()`), and made `Config::set_legacy_sandbox_policy`
  sync `self.workspace_roots` after the call (matches upstream).
- Added re-export `pub use resolved_permission_profile::PermissionProfileSnapshot;` near the
  module decls (~line 153) — the merge had dropped it; `config_tests.rs` relies on it via `use super::*`.

These cleared the entire group-B error set (effective_permission_profile / set_workspace_roots /
workspace_roots / set_permission_profile_from_session_snapshot / replace_... / materialize... /
effective_workspace_roots on core/legacy_core types) AND made core itself compile.

## REMAINING WORK (66 errors, all in `codex-tui` (65) + `codex-exec` (1))

Reference the saved list at `C:/Users/Oleh/check_errors.txt` (regenerate to refresh line numbers).
`codex_app_server_client::legacy_core::config::{Permissions,Config}` are RE-EXPORTS of
`codex_core::config::{Permissions,Config}` (see `codex-rs/app-server-client/src/legacy_core.rs`,
`pub use codex_core::config::*`), so the methods restored in DONE-2 already satisfy the
`legacy_core` paths — no separate edit needed there.

### GROUP A (RESTORE) — `ThreadSessionState` lost `collaboration_mode` + `personality`
~10 errors (E0560/E0609) across `tui/src/app/thread_session_state.rs`,
`tui/src/app_server_session.rs`, `tui/src/app/thread_settings.rs`, `tui/src/chatwidget/session_flow.rs`.
ROOT: HEAD moved `ThreadSessionState` from `tui/src/session_state.rs` (upstream) to the LOCAL
crate `codex-rs/tui-render/src/session_state.rs`, but DROPPED two fields during the move. The
consumer code (struct literals + field assignments) already references them — only the struct
def is missing the fields.
FIX (faithful — copy from `upstream/main:codex-rs/tui/src/session_state.rs`): in
`C:/Users/Oleh/Documents/GitHub/open_ai/codex/codex-rs/tui-render/src/session_state.rs`, add to
`struct ThreadSessionState`, placed right after the `reasoning_effort` field (to match upstream
field order so positional literals line up):
```rust
    pub collaboration_mode: Option<Box<CollaborationMode>>,
    pub personality: Option<Personality>,
```
and add imports `use codex_protocol::config_types::CollaborationMode;` and
`use codex_protocol::config_types::Personality;`.
NOTE: HEAD's tui-render struct uses `pub` (not upstream's `pub(crate)`) and imports
`AskForApproval` from `codex_protocol::protocol` — keep HEAD's style; only add the 2 fields + imports.
Verify `CollaborationMode`/`Personality` live in `codex-rs/config-types/src/lib.rs`
(re-exported as `codex_protocol::config_types::*`). No consumer edits expected for this group.

### GROUP F (WIRE local) — `mod config_update;` missing from tui crate root
~22 errors (E0432/E0433 "could not find `config_update` in the crate root") across
`onboarding_screen.rs`, `app/config_persistence.rs`, `app/event_dispatch.rs`, `lib.rs`.
The file `C:/Users/Oleh/Documents/GitHub/open_ai/codex/codex-rs/tui/src/config_update.rs`
EXISTS but is not declared. FIX: add `mod config_update;` (or `pub(crate) mod config_update;`
— check sibling decls for visibility convention) to the tui crate root
`codex-rs/tui/src/lib.rs` alongside the other `mod` declarations. This single line should
clear ALL ~22 config_update errors.

### GROUP G (WIRE local) — `override_turn_context` takes 13 args, callers pass 12
Errors E0061 at: `app/config_persistence.rs:158, 942, 1021`; `app/event_dispatch.rs:1228`;
`chatwidget/context_budget.rs:27`; `chatwidget/settings.rs:775`.
The fn is `App`/`AppCommand::override_turn_context` at
`C:/Users/Oleh/Documents/GitHub/open_ai/codex/codex-rs/tui/src/app_command.rs:178`. Its 13 params,
IN ORDER, are:
  cwd, approval_policy, approvals_reviewer, permission_profile, active_permission_profile,
  windows_sandbox_level, model, effort, summary, service_tier,
  **context_budget_mode: Option<ContextBudgetMode>** (index 10 — the LOCAL feature param),
  collaboration_mode: Option<CollaborationMode>, personality: Option<Personality>.
FIX: each 12-arg caller is missing `context_budget_mode`; insert the value the surrounding
code already has (look for an in-scope `context_budget_mode` / `self.config.context_budget_mode`
/ `None`) between the `service_tier` arg and the `collaboration_mode` arg.
CAUTION — two callers ALSO show TYPE mismatches that prove they're passing args in the WRONG
POSITIONS, not merely one short:
  - `chatwidget/context_budget.rs:37` E0308 expected `Option<String>` found `ContextBudgetMode`
  - `chatwidget/settings.rs:786`     E0308 expected `ContextBudgetMode` found `CollaborationMode`
For those two, align ALL positional args to the 13-arg signature above (the `context_budget_mode`
value is currently landing in the `service_tier`/`collaboration_mode` slot). Read each call site
and map argument-by-argument against the signature.

### GROUP H (WIRE local) — `App.active_profile` field missing
Errors E0609 "no field `active_profile` on type `&App`/`&mut App`" at
`app/config_persistence.rs:357, 399`; `app/event_dispatch.rs:1428, 1609, 1718`.
Several E0282 "type annotations needed" lines immediately follow these (e.g. config_persistence
358, event_dispatch 1613/1722) and are almost certainly downstream of the missing field —
fix `active_profile` first, then re-check before touching them.
ACTION: determine whether `active_profile` is an UPSTREAM field (check
`git show upstream/main:codex-rs/tui/src/app.rs` / wherever `struct App` lives — grep
`struct App` under `upstream/main:codex-rs/tui/`) or a local-permission-feature field. Add the
field to `struct App` with the correct type (likely `Option<ActivePermissionProfile>` or a
profile-id type) and initialize it wherever `App` is constructed. Cite which side it came from.
This one needs a real upstream lookup — do not guess the type.

### GROUP I (WIRE local) — `E0027` pattern missing `context_budget_mode`
One error at `app/thread_settings.rs:100`. A struct/destructure pattern doesn't mention the
local `context_budget_mode` field. FIX: add `context_budget_mode` to the pattern (or `..` if
appropriate to the surrounding style). Read the pattern to see which struct it destructures.

### GROUP C (RESTORE — BIGGEST remaining piece) — `AppServerTarget::LocalDaemon`, `LocalStateDbStartupError`, `uses_remote_workspace`
Errors at `tui/src/lib.rs:339 (LocalStateDbStartupError), 344 (LocalDaemon), 1350
(uses_remote_workspace)`; `tui/src/status/remote_connection.rs:17 (LocalDaemon), 23 (E0308
&str vs String)`; plus E0282 at `onboarding_screen.rs:599`.
ROOT: HEAD's `tui/src/lib.rs` is the LOCAL side of the merge and DROPPED the upstream
"local daemon" app-server path. `upstream/main:codex-rs/tui/src/lib.rs` defines
`enum AppServerTarget { ... LocalDaemon { endpoint: RemoteAppServerEndpoint }, ... }` (line 319)
and uses it at lines 348, 527, 833, 2144, 2230, 2271, 2575 (some are tests). It also defines/uses
`LocalStateDbStartupError` and a `uses_remote_workspace` value/fn.
FIX: diff `git show upstream/main:codex-rs/tui/src/lib.rs` vs HEAD's `tui/src/lib.rs` and
additively restore: the `LocalDaemon` enum variant, the `LocalStateDbStartupError` type (grep its
upstream definition — may be in lib.rs or an imported module), the `uses_remote_workspace` binding,
and the match arms that handle `LocalDaemon` (around HEAD lib.rs 339-344 and the status module).
This is the largest/most delicate restore; treat it carefully and confirm each symbol's upstream
definition before adding. The `remote_connection.rs:23` `&str`/`String` mismatch is likely a small
fallout once `LocalDaemon`'s endpoint shape is restored — re-check after.

### GROUP E (RESTORE) — duplicate `set_permission_profile_with_active_profile` (E0592)
One error at `tui/src/chatwidget/settings.rs:37`. The bad merge left TWO definitions of
`pub(crate) fn set_permission_profile_with_active_profile` in
`C:/Users/Oleh/Documents/GitHub/open_ai/codex/codex-rs/tui/src/chatwidget/settings.rs` — at
line 22 AND line 37. FIX: read both; keep the ONE whose signature matches upstream
(`git show upstream/main:codex-rs/tui/src/chatwidget/settings.rs`, def ~line 33) and delete the
stale duplicate. (Also note ChatWidget needs `set_permission_profile_from_session_snapshot` —
see Group K, config_persistence.rs:921 E0599 — that method likely lives in this same impl and may
be restored from upstream alongside.)

### GROUP D (RESTORE) — `ConfigRequirements.allow_appshots` (resolved type)
One error at `tui/src/debug_config.rs:164` E0609 "no field `allow_appshots` on
`&codex_config::ConfigRequirements`". The RESOLVED `ConfigRequirements` type (NOT the Toml one)
lost the `allow_appshots` field. FIX: in `codex-rs/config/src/config_requirements.rs`, restore the
`allow_appshots` field + wherever it's mapped from the Toml side (copy from
`git show upstream/main:codex-rs/config/src/config_requirements.rs`; grep `allow_appshots`).
This is in the `config` crate — sanctioned exception per the task (named restore target).

### GROUP J (WIRE local) — exec `ConfigOverrides` literal missing fields
The single `codex-exec` error: `exec/src/lib.rs:399` E0063 missing `config_profile` and
`workspace_roots` in `ConfigOverrides` initializer. FIX: add `config_profile: <value>` and
`workspace_roots: <value>` to that struct literal. Check the `ConfigOverrides` def
(`codex-rs/core/src/config/mod.rs:~1982`, it already has `pub workspace_roots: Option<Vec<PathBuf>>`
at ~2011) for the field types; pass `None`/default unless the surrounding exec code has an obvious
value (mirror how `codex-tui` or `codex-cli` fill these same two fields — grep other
`ConfigOverrides {` literals for the convention).

### GROUP K (likely fallout) — misc, re-check after the above
- `tui/src/app/background_requests.rs:558` E0425 `cannot find value config` — read context; likely
  a renamed/missing local binding from the merge.
- `tui/src/app/thread_settings.rs:179` E0308 `AskForApproval` (protocol vs app_server_protocol) —
  a `.to_core()` / `into()` conversion is probably missing; check which `AskForApproval` is in scope.
- `tui/src/tui.rs:1007` E0308 expected `*mut c_void` found `usize` — a Windows FFI cast; compare to
  upstream `tui.rs` (probably needs `as *mut c_void` or a `.cast()`); small, isolated.
- The E0282 "type annotations needed" lines should mostly vanish once their parent errors
  (active_profile, PermissionProfileSnapshot import, override_turn_context) are fixed. Re-check
  before adding turbofish annotations.
- `PermissionProfileSnapshot` undeclared in TUI (config_persistence.rs:109,259,921;
  settings.rs:45,538): the type now lives at `codex_core::config::PermissionProfileSnapshot`
  (we just re-exported it) reachable via `codex_app_server_client::legacy_core::config::PermissionProfileSnapshot`.
  Add the appropriate `use` import to those TUI files (grep how they import other legacy_core
  config types like `Permissions`/`Config` and mirror it).

## Suggested batch order for next session
1. Group F (one line, clears ~22). 2. Group A (2 fields + imports, clears ~10).
3. PermissionProfileSnapshot imports in TUI (Group K bullet, clears ~5).
4. Group E (delete duplicate). 5. Group G (the 13-arg wiring; do the 2 positional ones carefully).
6. Group H (active_profile — needs upstream lookup). 7. Group D, J, I (small).
8. Group C (LocalDaemon — biggest, last). 9. Re-check; mop up remaining Group-K fallout.
Re-run `cargo check -p codex-cli --release --jobs 2` after each batch; iterate to exit 0.
~6 GB RAM free on this 15.7 GB machine — keep `--jobs 2`.

## Things I had to DESIGN (flag for review)
- core `Permissions` snapshot methods are written against HEAD's `Constrained<PermissionProfile>`
  + `profile_workspace_roots` fields, NOT upstream's `permission_profile_state: PermissionProfileState`.
  Behavior matches upstream (install profile, set active id + profile roots; `replace_*` bypasses
  constraints via `Constrained::allow_only`), but internals differ because HEAD core was refactored.
- `FileSystemSandboxPolicy::materialize_project_roots_with_workspace_roots` omits the GlobPattern
  project-roots expansion that upstream had (HEAD lacks the glob helper fns; HEAD's cwd-variant
  also omits it). If glob `:project_roots` patterns must expand against workspace roots, the helpers
  `parse_project_roots_glob_pattern`/`resolve_project_roots_glob_pattern` would need restoring too.

## Known PRE-EXISTING breakage (NOT in codex-cli lib scope; left untouched intentionally)
`codex-rs/core/src/config/config_tests.rs` literals (lines ~8323, 8498, 8658) construct
`Permissions { permission_profile_state: ..., workspace_roots: ... }` — i.e. they target the
UPSTREAM `Permissions` shape, which does NOT match HEAD's refactored struct
(`permission_profile` + `active_permission_profile` + `profile_workspace_roots`). These tests were
ALREADY un-compilable against HEAD's lib before this session and cannot be fixed without either
reverting HEAD's struct refactor (user forbids) or rewriting the test literals. `cargo check -p
codex-cli --release` does NOT build these (cfg(test)), so they don't block the goal. Flag to the
user: HEAD core's test suite vs lib `Permissions` shape is out of sync from the merge and needs a
separate decision (rewrite tests to the new shape, recommended).
