# Worker Delegation And Commit Protocol

Use this protocol for external Codex worker sessions launched from this repo.

## Delegation

- Do not spawn additional Codex workers, helper sessions, or subagents from a
  worker session unless root explicitly changes this instruction in your prompt.
- Keep your lane bounded to the paths root assigned. If you find cross-lane
  work, write it in your handoff for root instead of expanding your scope.
- Keep handoffs compact: summarize findings, touched files, verification, commit
  hash, and blockers. Do not paste raw transcripts.

## Git Commit Protocol

- A coherent, verified worker slice should be committed when it is safe to do so.
- Before staging, run `git status --short --untracked-files=all` and `git ls-files -u`. Do not commit if unmerged files are present.
- Stage only explicit files from your owned lane and your handoff file, using pathspecs. Never use `git add .` or broad directory staging.
- Do not stage or commit root-owned files: `codex-rs/Cargo.toml`, `codex-rs/Cargo.lock`, Bazel files, merge-state files, shared boundary canaries, or unrelated workflow files.
- Before committing, run `git diff --cached --name-only` and verify every staged file belongs to your lane.
- Use a message like `solid-refactor: <lane>` or `tools: expose internal dab handlers`.
- Do not push. Root owns pushing and any final aggregate commit.
- If a clean commit is blocked by root-owned manifest work, unrelated dirty files, or incomplete verification, leave the changes unstaged and record the exact blocker plus suggested commit scope in your handoff.
