# Worker Delegation And Commit Protocol

Use this protocol for external Codex worker sessions launched from this repo.

## Delegation

- You are encouraged to spawn or reuse up to three focused helper/subagent sessions when it reduces context, speeds independent codebase questions, or improves review quality.
- Delegate only bounded tasks inside your lane. Give helpers exact first reads, forbidden paths, and the handoff detail you need back.
- Do not delegate across another worker's ownership lane. If you find cross-lane work, write it in your handoff for root.
- Ask helpers for compact findings or patches only; root remains responsible for final cross-lane integration.

## Git Commit Protocol

- A coherent, verified worker slice should be committed when it is safe to do so.
- Before staging, run `git status --short --untracked-files=all` and `git ls-files -u`. Do not commit if unmerged files are present.
- Stage only explicit files from your owned lane and your handoff file, using pathspecs. Never use `git add .` or broad directory staging.
- Do not stage or commit root-owned files: `codex-rs/Cargo.toml`, `codex-rs/Cargo.lock`, Bazel files, merge-state files, shared boundary canaries, or unrelated workflow files.
- Before committing, run `git diff --cached --name-only` and verify every staged file belongs to your lane.
- Use a message like `solid-refactor: <lane>` or `tools: expose internal dab handlers`.
- Do not push. Root owns pushing and any final aggregate commit.
- If a clean commit is blocked by root-owned manifest work, unrelated dirty files, or incomplete verification, leave the changes unstaged and record the exact blocker plus suggested commit scope in your handoff.
