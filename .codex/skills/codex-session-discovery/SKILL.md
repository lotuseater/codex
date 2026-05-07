---
name: codex-session-discovery
description: Use when Codex needs to find recent or live Codex sessions/logs on this Windows machine by project, token use, title, or transcript path without broad JSONL scans.
---

# Codex Session Discovery

Use this before manual filesystem archaeology over `~/.codex/sessions`.

## Fast Path

From `C:\Users\Oleh\Documents\GitHub\open_ai\codex`, run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\find-codex-sessions.ps1 -Project <project-root> -Limit 5 -RecentDays 3 -Json
```

The script checks `~/.codex/state_5.sqlite` first when `sqlite3` is available,
so it can return recent sessions by `cwd`, `updated`, `path`, `title`, and
`tokens_used` without opening full JSONL transcripts. It falls back to bounded
date-folder and modified-file scans only when indexed state is missing or
insufficient.

## Live Terminal Path

If the task is about currently visible PowerShell or Windows Terminal sessions,
prefer app-native harnesses or native `dab_*` tools first. If this session does
not expose `dab_*`, check Wizard MCP/config wiring before assuming DAB is
unavailable. Use JSONL scans only after the live/session-index paths fail.

## Rules

- Do not read full session JSONL files until a specific session path has been
  selected.
- Sort and reason by `updated` and `tokens_used` before inspecting transcript
  content.
- For token-burn audits, start from `tokens_used` in indexed state, then open
  only the highest-spend session tails or targeted excerpts.
- For exact handoff/control of a live terminal, pair this with
  `session-handoff-operator`.
