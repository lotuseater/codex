# Context Reducer Lab Results - 2026-05-07

## Summary

Created a sibling Rust lab at `C:\Users\Oleh\Documents\GitHub\context-reducer-lab` to test reducer algorithms before changing Codex runtime behavior. The lab writes JSON/Markdown reports under its own `reports/` directory.

## Results

| Tool | Result | Runtime decision |
| --- | --- | --- |
| `search_text` | Correct path/glob/fallback behavior. Prefix compression plus repeated-line grouping passed on the Serial `PyImport_ImportModule` case: 78 files preserved, no fallback, 36.6% token savings. A smaller Codex search preserved files but saved only 25.0%. | Keep shadow/tool-only. Port correctness, repeated-line grouping, and `path_prefix`; do not promote as standard replacement. |
| `file_outline` | Passed as lossy first-pass output on Rust, PowerShell, C/C++, Python, and TypeScript after C/C++ depth-aware anchors. | Keep fallback-required and shadow/tool-only. |
| `repo_context_scout` | Changed-first selection reached 100% changed-path coverage on Codex and stayed at the 1200-token budget after compact packet rendering and generated-log skips. Serial canary passed within 372 tokens. | Port generated-dir skips, PowerShell/C/C++ anchors, changed-first ranking, compact packet, and selected/missed changed telemetry. |
| `shadow_digest` | Parsed 684 replacement records. `git_status_compact` averaged negative savings; `git_worktree_summary` was also negative for status and name-only records. | Mark both `removed_rejected`; keep exact `git_diffstat_compact` as the only promoted family, with further records needed for broader promotion. |

## Key Evidence

- `search_serial_pyimport`: baseline 3378 estimated tokens, candidate 2143, saved 1235 tokens, no missing files, no fallback.
- `search_codex_replacement_shadow`: baseline 619, candidate 464, saved 155 tokens, no missing files, no fallback, but below the 30% promotion gate.
- `outline_codex_mixed` and `outline_serial_mixed`: both `pass_shadow_only`; outlines are useful only as lossy first-pass reducers.
- `scout_codex_reducers`: 25 changed paths, 25 selected, 0 missed, packet exactly 1200 estimated tokens.
- `shadow_digest_current`: `git_status_compact` 29 records at -10.8 average saved tokens; `git_worktree_summary` status records at -45.9 average saved tokens.

## Follow-Up Gate

Do not promote `search_text`, `file_outline`, lossy digests, or scout injection automatically from this batch. After the next build, collect live shadow records with the improved formats and promote only exact uncapped candidates with zero errors/fallbacks and at least 30% plus 32-token savings across multiple repos.
