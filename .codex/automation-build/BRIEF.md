# Shared brief — Claude Code automation build for the Codex fork

You are one of several fresh workers building **additive** Claude Code automation files for an OpenAI
Codex **Rust fork** the user maintains on **Windows (pwsh)**. Read this brief fully, then do only your
assigned slice (in your own prompt). Do NOT touch files outside your slice. Do NOT run `cargo`, builds,
or any mutating git. Write files to the **ABSOLUTE paths** given (subagents may run in a divergent
worktree; absolute main-repo paths guarantee your output reaches the live tree).

Repo root: `C:\Users\Oleh\Documents\GitHub\open_ai\codex`

## Project facts you must respect
- Rust workspace, ~150 `codex-`-prefixed crates. **Release-only locally**: debug `cargo` builds can
  exhaust C: disk/RAM here and are forbidden. Builds go through `scripts\build-local-codex.ps1`.
- Conventions live in `AGENTS.md` (repo root) — terse, idiomatic Rust; CI-enforced clippy rules.
- Existing `.claude/` is minimal: `settings.local.json` (allows `Bash(git *)` + `mcp__wizard__analyze_git_diff`).
  There is a heavy **user-level** wizard MCP + hook system in `~/.claude` — do not duplicate or fight it;
  our project hooks are additive.
- Match the surrounding style of any file type you create; keep files small and scannable.

## Exact commands (use these real names — do not invent)
- `just fmt` → `cargo fmt -- --config imports_granularity=Item`
- `just fix` → `cargo clippy --release --fix --tests --allow-dirty`
- `just test` → `cargo nextest run --no-fail-fast`
- `just write-config-schema` → regenerates `codex-rs/core/config.schema.json` (run when `ConfigToml` changes)
- `just write-app-server-schema` → regenerates app-server TS/JSON schema fixtures
- `just bazel-lock-update` / `just bazel-lock-check` → after `Cargo.toml`/`Cargo.lock` changes
- Build: `powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode FastRelease`
  (also `-Mode LowMemRelease`, `-Mode CleanSafe`; `-Mode DevRelease` intentionally throws "Build only release!").
- Test (scoped, release): `powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package <crate> -Filter <filter>` (`-Package` is mandatory).
- rustfmt settings (from `codex-rs/rustfmt.toml`): **`edition = "2024"`, `imports_granularity = "Item"`**.
  Per-file format: `rustfmt --edition 2024 --config imports_granularity=Item <file>`.

## Forbidden cargo command shapes (the debug-build guard blocks these)
- `cargo (build|test|check|run|clippy|nextest|bench)` WITHOUT `--release`
- anything targeting `target\debug`; `build-local-codex.ps1 -Mode DevRelease`
- broad debug lanes `cargo test -p codex-cli`, `cargo test -p codex-exec`
- Whitelisted (always allow): `cargo fmt`, `cargo insta`, `cargo metadata`, `cargo tree`.

## Verified Claude Code mechanics (use exactly)
- **PreToolUse block:** the hook reads tool-call JSON on **stdin** (`tool_name`, `tool_input`, `cwd`,
  `hook_event_name`). To DENY, print to stdout then `exit 0`:
  ```json
  {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"<msg>"}}
  ```
  To allow, just `exit 0` with no output. Matchers support `|` alternation (e.g. `Bash|PowerShell`,
  `Edit|Write|MultiEdit`) and are case-sensitive.
- **PostToolUse:** matcher `Edit|Write|MultiEdit`; stdin has `tool_input.file_path` (and `new_string`/
  `content`). May emit non-blocking context:
  `{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"<msg>"}}` then `exit 0`.
- **Stop hook:** stdin has `stop_hook_active`; if it's `true`, `exit 0` immediately (anti-loop). A Stop
  hook may print but here must NOT edit files.
- **Hook script path in settings.json command:** `pwsh -NoProfile -File "${CLAUDE_PROJECT_DIR}/.claude/hooks/<name>.ps1"`.
- **Skill frontmatter:** `name` (optional), `description` (required, "when to use"); user-only =
  `disable-model-invocation: true`; Claude-only = `user-invocable: false`; optional `allowed-tools`.
  Location: `.claude/skills/<name>/SKILL.md`.
- **MCP add (committed `.mcp.json`):** `claude mcp add --transport http <name> <url> [--header "Authorization: Bearer <PAT>"] --scope project`.

## CORRECTED hook design (safety: the working tree has many uncommitted .rs files)
Auto-format must touch ONLY the file Claude just edited — never the whole dirty tree. Therefore:
- **PostToolUse (`Edit|Write|MultiEdit`) → `post-edit-fmt.ps1`:** if `tool_input.file_path` ends in `.rs`,
  run `rustfmt --edition 2024 --config imports_granularity=Item <file>` on **that one file only**; exit 0.
- **Stop → `post-turn-remind.ps1`:** read-only. Guard `stop_hook_active`. From `git diff --name-only` +
  staged, PRINT reminders (never edit): config types → `just write-config-schema`; app-server-protocol →
  `just write-app-server-schema` + `just test -p codex-app-server-protocol`; `Cargo.toml`/`Cargo.lock` →
  `just bazel-lock-update` + `bazel-lock-check`. Use `additionalContext` style stdout or plain text; exit 0.
- **PreToolUse guards:** `block-debug-build.ps1` (matcher `Bash|PowerShell`), `guard-codex-sandbox.ps1`
  (matcher `Edit|Write|MultiEdit`; deny only when the ADDED text — `new_string`/`content` — contains
  `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` or `CODEX_SANDBOX_ENV_VAR`), `guard-merge-debris.ps1`
  (matcher `Bash`; only act when command is `git add`/`git commit`; deny if `*.orig` or `.codex/diff_*.patch`
  would be staged).

## Chronic merge hotspots (for the resolver agent + merge skill)
`codex-rs/core/src/session/turn.rs`, `handlers.rs`, `input_queue.rs`, `session_mailbox.rs`;
`codex-rs/protocol/src/protocol/op.rs`; `codex-rs/tui/src/bottom_pane/chat_composer.rs`;
`codex-rs/tui/src/app/event_dispatch.rs`; `codex-rs/features/src/lib.rs`;
`codex-rs/app-server-protocol/src/export.rs` (+ generated schema JSON); `codex-rs/config/src/config_toml.rs`.
Fork features + owner paths + per-feature health checks are catalogued in `docs/fork-feature-inventory.md`.
Merge policy reference: `.codex/tmp/merge_resolution_brief.md`; process: `.codex/workflow/HANDOFF_merge.md`,
`Main_Merge_Prompt.md`. Existing risk scorer to reuse: `scripts/analyze-branch-conflict-surface.ps1`.

## merge-conflict-resolver HANDOFF CONTRACT (W3 implements, W5 orchestrates — keep identical)
The resolver agent, given a disjoint slice of conflicted file paths, must finish by writing/returning:
```
HANDOFF_STATUS: success | partial | blocked
FILES_RESOLVED:
  - <path> | <strategy: union|take-fork|take-upstream|structural> | fork_feature_preserved: <name|none> | uncertainty: <low|med|high>
FILES_NEEDING_REGEN:
  - <path>            # e.g. export.rs / schema JSON — orchestrator regenerates post-merge
FILES_UNCERTAIN:
  - <path> | <why>    # needs human review
MARKERS_REMAINING: <int>   # must be 0 for success
```
Resolver rules: owns ONLY its listed files; union-by-default; preserve every fork feature whose owner
surface it touches (per `docs/fork-feature-inventory.md`); take-upstream only for pure upstream refactors;
NEVER run cargo/rustc or mutating git; treat generated files as best-effort union + flag for regen.

## Compact-survival clause (every worker follows this)
Write your files directly to their final ABSOLUTE paths as you finish each one, not only at the end. If
your context grows large (~150k tokens) or you sense an auto-compact, FIRST append a short handoff
(done / remaining / gotchas) to `C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\automation-build\<worker>.progress.md`,
THEN continue; after any compaction re-read it before resuming.

## Return format (every worker)
Return a TIGHT summary: the absolute paths you created, one line each on what they do, and any caveats or
follow-ups for the orchestrator. Do not paste full file contents back.
