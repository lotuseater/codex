# Shadow Alternatives Expansion, 2026-05-07

## Summary

This lane adds a second shadow-only expansion pack for high-token shell outputs.
It does not promote new replacements, change context-op schemas, or build/deploy
`codex.exe`.

The implementation is baseline-derived: each new candidate parses the shell
output Codex already captured and writes replacement-bench artifacts for
measurement. It must not rerun git, rg, process, listing, build, test, or diff
commands.

## Sources Inspected

- `codex-rs/core/src/tools/handlers/replacement_shadow.rs`
- `codex-rs/core/src/tools/handlers/replacement_shadow/classify.rs`
- `codex-rs/core/src/tools/handlers/replacement_shadow/baseline_digest.rs`
- `codex-rs/core/src/tools/handlers/shell.rs`
- `docs/token-saving-tools/operation-replacement-study.md`
- `docs/token-saving-tools/codex-fork-token-saving-plan.md`
- `logs/core-replacement-shadow-*.log`
- Current dirty tree, including the in-progress replacement-shadow refactor

## Added Shadow Operations

The expansion adds these baseline-only candidates after the first shadow pack so
existing behavior wins when command classes overlap:

- `file_excerpt_digest`
- `select_string_digest`
- `rg_count_digest`
- `rg_file_set_digest`
- `rg_json_digest`
- `git_name_status_compact`
- `git_numstat_compact`
- `git_filtered_diff_digest`
- `git_history_digest`
- `directory_listing_compact`
- `process_table_compact`

Shell-control commands are classified only when the candidate can be rendered
from the captured baseline output without spawning a process, such as filtered
`git diff ... | rg ...` output or bounded read/list pipelines.

## Fallback Policy

Lossy or capped digests emit `fallback_required: true` plus a concrete
`fallback_reason`, including `max_paths`, `max_lines`, `max_matches`,
`max_processes`, `lossy_diff_filter`, `lossy_git_history`,
`lossy_directory_listing`, `lossy_process_table`, or `json_parse_error`.

Small complete summaries may omit the fallback marker when the digest preserves
the baseline rows it summarizes.

## Bench Record

`replacement_bench` JSONL records now include `shadow_strategy`:

- `context_op_rerun` for existing candidates that rerun a typed context op.
- `baseline_digest` for baseline-derived candidates.

This is internal telemetry only and is not a model-facing schema or context-op
schema change.

## Non-Promotion Rules

The expansion candidates are shadow-only. `classify_promoted_replacement` must
continue to reject them, and standard replacement behavior remains limited to
the existing promoted diffstat path.

No exe build/deploy belongs to this lane. Build verification is left to the
release/build owner.
