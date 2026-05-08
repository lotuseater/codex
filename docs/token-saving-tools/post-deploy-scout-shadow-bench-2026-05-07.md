# Replacement Shadow Benchmark Report

Generated: 2026-05-07T23:27:31.6257490+03:00

Log dir: `C:\Users\Oleh\.codex\log\replacement-shadow`

Since: 2026-05-07T20:20:00.0000000+00:00

Records: 24

Parse errors: 0

Thresholds: records >= 5, repos >= 2, saved tokens >= 32, saved percent >= 30

| Operation | Strategy | Command class | Records | Repos | Errors | Fallbacks | Gate passed | Avg saved tokens | Avg saved % | Recommendation |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| file_excerpt_digest | baseline_digest | file_read | 2 | 1 | 0 | 2 | 0 | 1142.5 | 70.1 | keep_collecting |
| git_diffstat_compact | baseline_digest | git_diff_stat | 4 | 2 | 0 | 0 | 3 | 178.2 | 13.8 | keep_collecting |
| select_string_digest | baseline_digest | $erroractionpreference='continue' | 1 | 1 | 0 | 0 | 0 | -27 | -11.3 | keep_collecting |
| select_string_digest | baseline_digest | $out | 1 | 1 | 0 | 1 | 0 | 2730 | 52.5 | keep_collecting |
| select_string_digest | baseline_digest | $since=[datetimeoffset]'2026-05-07t20:20:00+00:00' | 1 | 1 | 0 | 0 | 0 | -38 | -43.2 | keep_collecting |
| select_string_digest | baseline_digest | $since=[datetimeoffset]'2026-05-07t20:20:00+00:00'; | 1 | 1 | 0 | 0 | 0 | -11 | -36.7 | keep_collecting |
| select_string_digest | baseline_digest | select-string | 1 | 1 | 0 | 1 | 0 | 696 | 30.7 | keep_collecting |
| diff_hunk_summary | baseline_digest | git_diff_full | 1 | 1 | 0 | 1 | 0 | 2119 | 94.3 | keep_shadow_only |
| git_filtered_diff_digest | baseline_digest | $erroractionpreference='continue' | 1 | 1 | 0 | 1 | 0 | -33 | -8.7 | keep_shadow_only |
| git_status_compact | baseline_digest | git_status | 3 | 1 | 0 | 0 | 0 | -12.3 | -10.6 | keep_shadow_only |
| run_check_digest | baseline_digest | invoke-bounded | 2 | 1 | 0 | 2 | 0 | 144.5 | 74.6 | keep_shadow_only |
| search_text | context_op_rerun | rg_search | 2 | 1 | 0 | 2 | 0 | 170 | 27.3 | keep_shadow_only |
| file_outline | legacy_unknown | file_read | 3 | 1 | 0 | 0 | 3 | 476 | 75.1 | legacy_review_only |
| git_worktree_summary | legacy_unknown | git_status | 1 | 1 | 0 | 0 | 0 | -41 | -35.7 | legacy_review_only |

## Artifact Samples

- `file_excerpt_digest` / `file_read` / `keep_collecting`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202017105Z-call_5IWA6lJC3EskEu4NdcgV2Pvt-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202017105Z-call_5IWA6lJC3EskEu4NdcgV2Pvt-file_excerpt_digest.txt`
- `git_diffstat_compact` / `git_diff_stat` / `keep_collecting`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202103198Z-call_xbpir61Lb95IuuoevGwQxjFh-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202103198Z-call_xbpir61Lb95IuuoevGwQxjFh-git_diffstat_compact.txt`
- `select_string_digest` / `$erroractionpreference='continue'` / `keep_collecting`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202724072Z-call_Epgjw36aQamPxBEbWsJY5dsD-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202724072Z-call_Epgjw36aQamPxBEbWsJY5dsD-select_string_digest.txt`
- `select_string_digest` / `$out` / `keep_collecting`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202635029Z-call_6vnpk4Gu1dbvo7Hk7WMrOWg5-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202635029Z-call_6vnpk4Gu1dbvo7Hk7WMrOWg5-select_string_digest.txt`
- `select_string_digest` / `$since=[datetimeoffset]'2026-05-07t20:20:00+00:00'` / `keep_collecting`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202418636Z-call_TmlRbzzOfVyxx339MG3YIbso-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202418636Z-call_TmlRbzzOfVyxx339MG3YIbso-select_string_digest.txt`
- `select_string_digest` / `$since=[datetimeoffset]'2026-05-07t20:20:00+00:00';` / `keep_collecting`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202348259Z-call_l7BGTkb7lxG6ruiF1YKcTaBq-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202348259Z-call_l7BGTkb7lxG6ruiF1YKcTaBq-select_string_digest.txt`
- `select_string_digest` / `select-string` / `keep_collecting`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202314011Z-call_kv3opemDGBkwF4zOkJQvVd9A-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202314011Z-call_kv3opemDGBkwF4zOkJQvVd9A-select_string_digest.txt`
- `diff_hunk_summary` / `git_diff_full` / `keep_shadow_only`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202103036Z-call_7KgACfQRVytYUDXEaGe1jTHP-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202103036Z-call_7KgACfQRVytYUDXEaGe1jTHP-diff_hunk_summary.txt`
- `git_filtered_diff_digest` / `$erroractionpreference='continue'` / `keep_shadow_only`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202623197Z-call_h66iNh3iyM4ys6SOowfTgwFh-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202623197Z-call_h66iNh3iyM4ys6SOowfTgwFh-git_filtered_diff_digest.txt`
- `git_status_compact` / `git_status` / `keep_shadow_only`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202103081Z-call_AQ9K2IaDgNgtjDthBTZaw5vI-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202103081Z-call_AQ9K2IaDgNgtjDthBTZaw5vI-git_status_compact.txt`
- `run_check_digest` / `invoke-bounded` / `keep_shadow_only`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202541880Z-call_1E1T8YyTp4t0xKPwX4RWuxsV-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202541880Z-call_1E1T8YyTp4t0xKPwX4RWuxsV-run_check_digest.txt`
- `search_text` / `rg_search` / `keep_shadow_only`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202707371Z-call_esHiqjE55AAxQhw5ovjScwB0-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202707371Z-call_esHiqjE55AAxQhw5ovjScwB0-search_text.txt`
- `file_outline` / `file_read` / `legacy_review_only`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202148747Z-call_wgU272k6ejEqyRPCp3dyp8Ry-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202148747Z-call_wgU272k6ejEqyRPCp3dyp8Ry-file_outline.txt`
- `git_worktree_summary` / `git_status` / `legacy_review_only`
  - baseline: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202526219Z-call_6DPLRQeoUMU3PwzWkWsFxcXx-baseline.txt`
  - replacement: `C:\Users\Oleh\.codex\log\replacement-shadow\artifacts\20260507T202526219Z-call_6DPLRQeoUMU3PwzWkWsFxcXx-git_worktree_summary.txt`

## Post-Deploy Interpretation

Deployed build under test:

- Wrapper: `C:\Users\Oleh\.codex\local-builds\codex-custom-20260507-231503\codex.exe`
- Version: `codex-cli 0.0.0 (local build 2026-05-07T23:00:49+03:00)`
- Features: `context_ops_shadow=true`, `context_ops_replace=true`

Canary cost was high even with `codex exec --ignore-rules --ephemeral`:

| Canary | Input tokens | Cached input tokens |
|---|---:|---:|
| codex `rg -n replacement_shadow codex-rs/core` | 48,529 | 25,856 |
| codex `rg replacement_shadow -g "*.rs" codex-rs/core` | 48,506 | 26,880 |
| Serial `git diff --stat` | 41,095 | not captured in table output |
| Serial `git status --short` | 40,992 | not captured in table output |

This means shell-output replacement helps output/context growth after a tool
call, but it does not solve the large fresh-turn startup context. Reducing
startup context still needs scout/first-steps work, prompt compression, or
session/context pruning.

Promotion decision:

- Keep the single promoted exact candidate: `git_diffstat_compact` for `git diff --stat` and `git diff --shortstat`, with the existing savings gate. It is exact on the non-empty Serial diff and saved 241 model-visible tokens, 48.7%. It was longer on an empty codex diff, so unconditional replacement would be wrong.
- Do not promote `git_status_compact` or `git_worktree_summary` for status. Both are still larger than raw status on these repos because headers and counts outweigh the short baseline.
- Do not promote `search_text`. The path handling regression is fixed for positive path and glob searches, but both fresh records are capped with `fallback_reason: max_matches_per_file` and saved only 26-28%, below the promotion threshold.
- Keep `file_outline`, `file_excerpt_digest`, `diff_hunk_summary`, `git_filtered_diff_digest`, `run_check_digest`, and `select_string_digest` shadow-only. They are useful summaries, but they are lossy or marked fallback-required.

Scout shadow results:

| Repo | Records | Packet tokens | Observed changed-path coverage | Notes |
|---|---:|---:|---:|---|
| codex | 5 | 1,032-1,170 | 0 / 6 current changed paths | Selected shell/app-server/exec-policy files for command canaries, not the active multi-agent/script/doc edits. |
| Serial_to_Google_Doc_topdown | 2 | 724-749 | 3-4 / 12 current changed paths | Picked several relevant changed files, but also selected vendored `build_standalone/_deps` files. |

Scout should stay shadow/tool-only for now. It is bounded under the 1,200 token
packet target, but selection quality is not good enough for automatic first-turn
injection. Next scout improvement should prioritize dirty/changed paths, exclude
vendored/build dependency trees by default, and record a direct hit/miss score
against subsequent file reads.

Analyzer fix made during this run:

- `scripts/analyze-replacement-shadow.ps1` now treats `-Since` as
  `DateTimeOffset` and preserves `ConvertFrom-Json` timestamp objects instead
  of reparsing culture-formatted date strings. The previous report mixed old
  legacy rows into the deployed-build window.
