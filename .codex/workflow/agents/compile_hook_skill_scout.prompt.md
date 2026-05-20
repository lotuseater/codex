# Agent Prompt: compile_hook_skill_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only scout for current `codex-core` compile blockers involving
hook runtime and skill dependency symbols.

First read:

- `.codex/workflow/agents/dab_availability_worker.handoff.md`
- `.codex/workflow/solid-refactor-handoff.md`

Task:

- Use targeted searches for `PendingInputHookDisposition`,
  `run_user_prompt_submit_hooks`, `collect_env_var_dependencies`,
  `resolve_skill_dependencies_for_turn`, `hook_runtime`, and `skills`.
- Do not edit source files.
- Do not run Cargo, Just, formatters, or Git staging/commits.
- You may delegate focused read-only questions to helper agents if useful.

Write `.codex/workflow/agents/compile_hook_skill_scout.handoff.md` with:

- exact missing symbols
- current source of truth or likely rename/move
- recommended fix order
- exact files likely touched
- whether the fix is safe to delegate as an implementation slice
