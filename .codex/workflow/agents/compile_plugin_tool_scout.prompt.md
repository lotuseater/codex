# Agent Prompt: compile_plugin_tool_scout

Work in `C:\Users\Oleh\Documents\GitHub\open_ai\codex`.

You are a read-only scout for current compile blockers involving plugin tool
symbols exported from `codex-tools`.

First read:

- `.codex/workflow/agents/dab_availability_worker.handoff.md`
- `.codex/workflow/solid-refactor-handoff.md`

Task:

- Use targeted searches for `LIST_AVAILABLE_PLUGINS_TO_INSTALL_TOOL_NAME`,
  `ListAvailablePluginsToInstallResult`, `list_available_plugins`,
  `RequestPluginInstall`, and plugin install/list tool definitions.
- Do not edit source files.
- Do not run Cargo, Just, formatters, or Git staging/commits.
- You may delegate focused read-only questions to helper agents if useful.

Write `.codex/workflow/agents/compile_plugin_tool_scout.handoff.md` with:

- exact missing symbols
- current replacement/source of truth
- recommended fix
- exact files likely touched
- whether the fix is safe to delegate as an implementation slice
