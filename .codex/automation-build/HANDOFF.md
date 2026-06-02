# Handoff — Claude Code automation + merge-toolkit build (CP-1)

Plan: `C:\Users\Oleh\.claude\plans\spicy-leaping-puzzle.md`. Shared worker brief:
`.codex/automation-build/BRIEF.md`. This build is **additive only** — no Rust source touched.

## State at this checkpoint

All deliverable files are written to disk and the hooks are **live this session** (they fired on
my own edits). Nothing committed yet — that is the next step. The working tree still carries the
large pre-existing `slow-context-budget-mode` diff; the automation files must be committed
**selectively** (do NOT `git add -A`).

## What was built (all present on disk)

**Hooks — `.claude/settings.json` + `.claude/hooks/*.ps1`**
- `block-debug-build.ps1` — PreToolUse `Bash|PowerShell`; denies debug-profile cargo
  (build/test/check/run/clippy/nextest/bench without `--release`), `target\debug`, `-Mode DevRelease`,
  broad lanes (`-p codex-cli`, `-p codex-exec`); whitelists fmt/insta/metadata/tree.
- `guard-codex-sandbox.ps1` — PreToolUse `Edit|Write|MultiEdit`; **now scoped to `*.rs` only** (see
  "Refinement" below); denies added text matching the sandbox env-var constants.
- `guard-merge-debris.ps1` — PreToolUse `Bash`; on `git add`/`git commit` denies staging `*.orig` or
  `.codex/diff_*.patch`.
- `post-edit-fmt.ps1` — PostToolUse `Edit|Write|MultiEdit`; rustfmt's the single edited `.rs` file
  (`--edition 2024 --config imports_granularity=Item`); errors swallowed; always exit 0.
- `post-turn-remind.ps1` — Stop hook, read-only, `stop_hook_active`-guarded; prints schema/bazel
  reminders based on changed files.

**Subagents — `.claude/agents/*.md`**
- `rust-conventions-reviewer.md` — reviews branch diff vs CI-enforced AGENTS.md rules.
- `core-boundary-guard.md` — flags new code/deps landing in `codex-core` that belong elsewhere.
- `merge-conflict-resolver.md` (opus) — resolves a disjoint slice of conflicts under fork policy;
  encodes the HANDOFF CONTRACT from BRIEF.md; never runs cargo/mutating git.

**Skills — `.claude/skills/<name>/SKILL.md`**
- `app-server-api/` (model+user) — app-server-v2 API checklist.
- `new-crate/` (user-only) — scaffold a `codex-`-prefixed crate.
- `merge-upstream/` (user-only) — 6-step upstream-merge runbook (preflight → pre-merge extraction →
  no-build resolution fan-out → fix-up → release verify → deploy).

**Merge toolkit — `scripts/*.ps1`** (validated against live `upstream/main`)
- `generate-merge-hotspot-map.ps1` → writes `docs/merge-hotspot-map.md`;
  `HotspotScore=(MergeTouches+1)*(UpstreamChurn+1)*RiskWeight`.
- `detect-adapter-gaps.ps1` → flags fork-heavy upstream-hot files lacking a seam; exit 1 on gaps.

**Brief** — `.codex/automation-build/BRIEF.md` (shared worker context).

## Refinement made at CP-1 (important)

The original `guard-codex-sandbox.ps1` matched the forbidden constant names in **any** file's added
text, which is too broad: it blocked editing docs (this handoff), the brief, and even the guard
script itself. Fixed: the guard now (a) only enforces on `file_path` ending in `.rs` (the AGENTS.md
rule is about code, not prose) and (b) expresses the match as a single regex
`CODEX_SANDBOX(?:_NETWORK_DISABLED)?_ENV_VAR` that does not spell either full constant contiguously,
so editing the guard never trips itself. Smoke-tested 3 ways (`.rs`+const → deny; `.md`+const →
allow; `.rs`+clean → allow) — all pass.

## Verification status

- Hooks fire live this session (proven: the sandbox guard blocked, then allowed, real edits).
- Sandbox guard: smoke-tested via stdin (3 cases pass, see above).
- Other guards validated by W1 at creation (stdin sample JSON → correct exit/JSON).
- Merge scripts: W6 validated against live `upstream/main`.
- No Rust source modified → no cargo build/test needed for this build.
- NOT yet run end-to-end in a fresh session: the Stop reminder hook and post-edit-fmt against an
  actual `.rs` edit (low risk; logic is straightforward and read-only / single-file).

## Next steps (ORDERED)

1. `mcp__wizard__analyze_git_diff` (required before any commit by the repo commit guard).
2. Selective stage — do **NOT** `git add -A`. Stage exactly:
   - `.claude/settings.json`
   - `.claude/hooks/block-debug-build.ps1 guard-codex-sandbox.ps1 guard-merge-debris.ps1 post-edit-fmt.ps1 post-turn-remind.ps1`
   - `.claude/agents/rust-conventions-reviewer.md core-boundary-guard.md merge-conflict-resolver.md`
   - `.claude/skills/app-server-api/SKILL.md new-crate/SKILL.md merge-upstream/SKILL.md`
   - `scripts/generate-merge-hotspot-map.ps1 scripts/detect-adapter-gaps.ps1`
   - `.codex/automation-build/BRIEF.md .codex/automation-build/HANDOFF.md`
   - (Optional) `docs/merge-hotspot-map.md` if the generator was run and the output looks good.
   Avoid staging any `*.orig` / `.codex/diff_*.patch` (the merge-debris guard will block it anyway).
3. Commit. Message must avoid the literal debug-build cargo tokens (the block guard scans
   `Bash|PowerShell`, not the commit body — but keep it clean: say "debug-profile builds"). End with
   the required `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>` trailer.
4. CP-1 `/compact` (clean boundary; everything banked to git).

## Deferred to the user (Part D — MCP, needs secrets)

Documented, NOT installed (each needs a user-supplied secret/endpoint):
- GitHub MCP: `claude mcp add --transport http github https://api.githubcopilot.com/mcp/ --header "Authorization: Bearer <PAT>" --scope project` (needs the user's GitHub PAT).
- context7: `claude mcp add --transport http context7 https://mcp.context7.com/mcp --scope project` (confirm the endpoint at install).

## Deferred to a future pre-merge boundary (Part F — structural extractions)

NOT in this session. Each is its own task at the next clean boundary, one restore-point commit each,
validated by `scripts/analyze-branch-conflict-surface.ps1` before/after:
1. `protocol/op.rs` → `protocol/op_fork.rs` re-export seam (high impact / low effort).
2. `session/turn.rs` → `session/hook_input_gate.rs` + `session/multi_task_runner.rs`.
3. `features/src/lib.rs` → `features/src/fork_features.rs` (`FORK_FEATURES` constant).
4. `session/input_queue.rs` + `session_mailbox.rs` → `codex-input-queue` multi-task module.

## Gotchas

- The active hooks now constrain THIS session too: debug-profile cargo commands get denied; edits to
  `.rs` adding the sandbox constants get denied. Use `--release` builds / the build scripts.
- Feed hook stdin via OS redirection or `echo '<json>' | pwsh -File <script>` — generate JSON so
  backslashes survive (bash `printf` collapses them).
- Subagents may run in a divergent worktree → always delegate edits via ABSOLUTE main-repo paths.
