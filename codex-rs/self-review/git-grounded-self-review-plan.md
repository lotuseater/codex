# Git-Grounded Automatic Self-Review Plan

## Goal
Replace automatic self-review's current generic review-mode trigger with a normal queued user turn that is grounded in the git work slice since the last completed review.

Codex should be able to say exactly what changed, using commits and changed files, then review and improve those changes before resuming queued work.

## Required Behavior
- Track a review anchor after session start and after every completed explicit or automatic review.
- Capture anchor evidence from git: current `HEAD`, dirty tracked files, staged files, and untracked files.
- For files dirty at the anchor, keep bounded baseline snapshots outside the repo so later review can isolate changes made after the anchor.
- When automatic self-review is due, enqueue a normal `UserTurn` prompt at the front of the queue. Do not use `Op::Review` for the automatic path.
- Do not interrupt a running turn. The review prompt should run next.
- Preserve manual `/review` behavior, but refresh the git anchor after manual review completes.
- After automatic review completes, refresh the anchor and allow queued work to resume automatically.
- Merge pending natural-language queued prompts behind the review prompt with stable delimiters.
- Keep slash commands and shell commands as command actions; do not merge them into plain text.

## Prompt Requirements
The automatic review prompt should include:

- The review anchor commit, when available.
- Commits since the anchor, using a command such as `git log <anchor>..HEAD`.
- Changed files since the anchor.
- Exact diff commands the agent can run:
  - `git diff <anchor>..HEAD -- <files>`
  - `git diff --cached -- <files>`
  - `git diff -- <files>`
  - `git diff --no-index -- <baseline> <current>` for files that were already dirty at the anchor and have a baseline snapshot.
- A fallback section for non-git repos or missing anchors, clearly saying that evidence is limited.
- Clear instructions to review the changes for bugs, regressions, maintainability issues, and long-term design problems; fix actionable findings; then let Codex resume queued work.

## Implementation Targets
- `codex-rs/self-review/src/lib.rs`: keep cooldown/counting behavior, but add git evidence types and prompt generation.
- Prefer a new `codex-rs/self-review/src/git_evidence.rs` module if the implementation would make `lib.rs` large.
- `codex-rs/tui/src/chatwidget/turn_runtime.rs`: replace automatic `app_event_tx.review(ReviewTarget::Custom)` use with queued/submitted `UserTurn` behavior.
- `codex-rs/tui/src/chatwidget/input_restore.rs`: support front-of-queue automatic self-review and merging adjacent plain queued prompts with delimiters.
- `codex-rs/tui/src/chatwidget/user_messages.rs`: add delimiter-aware plain prompt merge helper if needed.
- `codex-rs/tui/src/bottom_pane/chat_composer.rs`: add an internal queued action only if needed to distinguish automatic review prompts from normal plain prompts.
- `codex-rs/tui/src/chatwidget/session_flow.rs`: refresh/capture anchor when session cwd/configuration becomes available.

## Test Requirements
- `codex-self-review` tests for prompt generation with commits, changed files, dirty baselines, untracked files, and non-git fallback.
- TUI tests proving automatic self-review emits `Op::UserTurn`, not `Op::Review`.
- TUI queue tests proving automatic review is ahead of pending plain prompts.
- TUI queue tests proving adjacent plain prompts merge with delimiters.
- TUI queue tests proving slash and shell queued actions stay separate.

## Verification Commands
Use release-only local lanes:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-self-review
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-tui -Filter self_review
powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package codex-tui -Filter queue
```

Then run:

```powershell
cd codex-rs
just fmt
just fix -p codex-self-review
just fix -p codex-tui
```

## Boundaries
- Do not change manual `/review` semantics except for refreshing the git anchor after completion.
- Do not add app-server or protocol wire changes in the first slice.
- Do not store baseline snapshots in the repo tree.
- Keep snapshot storage bounded by file size and best-effort cleanup old baseline directories when a new anchor replaces an old one.
