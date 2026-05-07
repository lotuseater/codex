# Replacement Shadow Benchmark Report

## Waiting-Window Context

This report was generated before the next exe deploy. The active wrapper still
uses the 08:41 local build, so all current benchmark rows are legacy telemetry:
they do not include `shadow_strategy` or `replacement_gate_passed`.

Post-deploy workflow:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run-replacement-shadow-canaries.ps1 -OutFile logs\replacement-shadow-canaries-dry-run-20260507.md
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run-replacement-shadow-canaries.ps1 -Execute
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\analyze-replacement-shadow.ps1 -Since "2026-05-07T16:46:22Z" -OutMarkdown docs\token-saving-tools\post-deploy-scout-shadow-bench-2026-05-07.md
```

Only use `-Execute` after `codex --version` shows the new build and
`codex features list` shows both `context_ops_shadow=true` and
`context_ops_replace=true`.

Generated: 2026-05-07T20:00:06.2363974+03:00

Log dir: `C:\Users\Oleh\.codex\log\replacement-shadow`

Records: 471

Parse errors: 0

Thresholds: records >= 5, repos >= 2, saved tokens >= 32, saved percent >= 30

| Operation | Strategy | Command class | Records | Repos | Errors | Fallbacks | Gate passed | Avg saved tokens | Avg saved % | Recommendation |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| file_outline | legacy_unknown | file_read | 123 | 4 | 6 | 6 | 106 | 1991.9 | 76.1 | legacy_review_only |
| git_worktree_summary | legacy_unknown | git_diff_name_only | 8 | 3 | 0 | 0 | 5 | 252.6 | -159.4 | legacy_review_only |
| git_worktree_summary | legacy_unknown | git_diff_stat | 87 | 4 | 0 | 0 | 49 | 194.6 | -33.1 | legacy_review_only |
| git_worktree_summary | legacy_unknown | git_status | 231 | 4 | 0 | 0 | 0 | -46.3 | -87.6 | legacy_review_only |
| search_text | legacy_unknown | rg_search | 22 | 4 | 11 | 13 | 6 | 1363.9 | 56.7 | legacy_review_only |

## Artifact Samples

- `file_outline` / `file_read` / `legacy_review_only`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T060123583Z-call_QDxHKosaq88Xi64zOp7HEIJM-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T060123583Z-call_QDxHKosaq88Xi64zOp7HEIJM-file_outline.txt`
- `git_worktree_summary` / `git_diff_name_only` / `legacy_review_only`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T122008939Z-call_GYppuvtRHkLj8e1QLWFX0LL5-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T122008939Z-call_GYppuvtRHkLj8e1QLWFX0LL5-git_worktree_summary.txt`
- `git_worktree_summary` / `git_diff_stat` / `legacy_review_only`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T060126218Z-call_ryooWZIftHvN5qP2EV4RekW7-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T060126218Z-call_ryooWZIftHvN5qP2EV4RekW7-git_worktree_summary.txt`
- `git_worktree_summary` / `git_status` / `legacy_review_only`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T055248695Z-call_PH4e3MCO97WQugFLGMH1ybyi-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T055248695Z-call_PH4e3MCO97WQugFLGMH1ybyi-git_worktree_summary.txt`
- `search_text` / `rg_search` / `legacy_review_only`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T060119364Z-call_hYb1hWhhayBzU4ndFY3kWQpM-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T060119364Z-call_hYb1hWhhayBzU4ndFY3kWQpM-search_text.txt`
