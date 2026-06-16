# Batch Programming Metrics R3

Generated: `2026-06-16T13:16:14+00:00`

This is a deterministic lab benchmark. It should be used to choose live paired Codex runs, not as a live model-performance claim.

## Headline

workflow_batch_spec_file leads by avg composite score 77.86 across 30 rows.

Run more live paired trials before changing defaults; too many deterministic wins are narrow.

## Inputs

- Manifest: `cases/batch_programming_metrics_demo_20260615.json`
- Scenarios: `6`
- Tasks: `5`
- Variants: `7`
- Rows: `210`

## Scenario Matrix

| Scenario | Evidence | Purpose |
| --- | --- | --- |
| Manifest nominal | `Original 2026-06-15 demo manifest.` | Keeps the previous simulated workload as the comparability baseline. |
| Portable apply-patch fixture matrix | `codex-rs/apply-patch/tests/fixtures/scenarios/README.md` | Models simple input/patch/expected fixture directories that should be portable across implementations. |
| Operation-cache cold route | `docs/operation-cache-status.md and operation_cache symbol search.` | Adds first-run routing and cache-namespace setup cost before reuse is available. |
| Operation-cache warm rerun | `docs/operation-cache-status.md and operation_cache symbol search.` | Models the repeated deterministic run after route and namespace data are already present. |
| Dirty worktree guardrail | `Existing lab repo status with generated artifacts and reports.` | Rewards local diagnosability and deterministic assertions when unrelated files are already dirty. |
| Docs/report audit | `README Batch Programming Metrics Demo section and reports/*.md outputs.` | Models broad markdown/report evidence checks after benchmark output generation. |

## Ranking

| Rank | Variant | Avg score | Avg elapsed ms | Avg tokens | Wins | Avg repairs |
| ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 1 | `workflow_batch_spec_file` | 77.86 | 5772.13 | 2018.03 | 29 | 1.0 |
| 2 | `hybrid_scout_batch` | 76.32 | 5961.53 | 2046.37 | 0 | 1.0 |
| 3 | `python_script` | 75.99 | 6059.17 | 2013.87 | 0 | 1.0 |
| 4 | `workflow_batch_inline` | 75.72 | 6142.5 | 1992.2 | 0 | 1.0 |
| 5 | `focused_shell_batch` | 68.82 | 8826.77 | 1975.53 | 0 | 1.0 |
| 6 | `interactive_sequential` | 66.87 | 13965.93 | 1959.2 | 1 | 1.6 |
| 7 | `delegated_worker_batch` | 64.37 | 10599.37 | 2104.53 | 0 | 2.0 |

## Decision Cases

| Scenario | Task | Winner | Runner up | Score delta |
| --- | --- | --- | --- | ---: |
| `apply_patch_fixture_matrix` | `inventory_reduce` | `workflow_batch_spec_file` | `hybrid_scout_batch` | 2.09 |
| `apply_patch_fixture_matrix` | `json_normalize` | `workflow_batch_spec_file` | `python_script` | 1.65 |
| `apply_patch_fixture_matrix` | `md_audit` | `workflow_batch_spec_file` | `python_script` | 2.09 |
| `apply_patch_fixture_matrix` | `mechanical_patch` | `workflow_batch_spec_file` | `python_script` | 1.87 |
| `apply_patch_fixture_matrix` | `one_off_micro_probe` | `workflow_batch_spec_file` | `workflow_batch_inline` | 0.62 |
| `dirty_worktree_guardrail` | `inventory_reduce` | `workflow_batch_spec_file` | `python_script` | 2.16 |
| `dirty_worktree_guardrail` | `json_normalize` | `workflow_batch_spec_file` | `python_script` | 1.72 |
| `dirty_worktree_guardrail` | `md_audit` | `workflow_batch_spec_file` | `workflow_batch_inline` | 2.12 |
| `dirty_worktree_guardrail` | `mechanical_patch` | `workflow_batch_spec_file` | `python_script` | 1.72 |
| `dirty_worktree_guardrail` | `one_off_micro_probe` | `workflow_batch_spec_file` | `workflow_batch_inline` | 0.32 |
| `docs_report_audit` | `inventory_reduce` | `workflow_batch_spec_file` | `hybrid_scout_batch` | 0.97 |
| `docs_report_audit` | `json_normalize` | `workflow_batch_spec_file` | `hybrid_scout_batch` | 1.15 |
| `docs_report_audit` | `md_audit` | `workflow_batch_spec_file` | `hybrid_scout_batch` | 0.97 |
| `docs_report_audit` | `mechanical_patch` | `workflow_batch_spec_file` | `hybrid_scout_batch` | 1.15 |
| `docs_report_audit` | `one_off_micro_probe` | `workflow_batch_spec_file` | `hybrid_scout_batch` | 1.19 |
| `manifest_nominal` | `inventory_reduce` | `workflow_batch_spec_file` | `hybrid_scout_batch` | 0.74 |
| `manifest_nominal` | `json_normalize` | `workflow_batch_spec_file` | `hybrid_scout_batch` | 0.92 |
| `manifest_nominal` | `md_audit` | `workflow_batch_spec_file` | `hybrid_scout_batch` | 0.74 |
| `manifest_nominal` | `mechanical_patch` | `workflow_batch_spec_file` | `hybrid_scout_batch` | 0.7 |
| `manifest_nominal` | `one_off_micro_probe` | `interactive_sequential` | `workflow_batch_inline` | 1.43 |
| `operation_cache_cold_route` | `inventory_reduce` | `workflow_batch_spec_file` | `hybrid_scout_batch` | 0.74 |
| `operation_cache_cold_route` | `json_normalize` | `workflow_batch_spec_file` | `python_script` | 0.75 |
| `operation_cache_cold_route` | `md_audit` | `workflow_batch_spec_file` | `hybrid_scout_batch` | 0.92 |
| `operation_cache_cold_route` | `mechanical_patch` | `workflow_batch_spec_file` | `python_script` | 0.97 |
| `operation_cache_cold_route` | `one_off_micro_probe` | `workflow_batch_spec_file` | `workflow_batch_inline` | 0.21 |
| `operation_cache_warm_rerun` | `inventory_reduce` | `workflow_batch_spec_file` | `python_script` | 1.63 |
| `operation_cache_warm_rerun` | `json_normalize` | `workflow_batch_spec_file` | `python_script` | 1.19 |
| `operation_cache_warm_rerun` | `md_audit` | `workflow_batch_spec_file` | `python_script` | 1.41 |
| `operation_cache_warm_rerun` | `mechanical_patch` | `workflow_batch_spec_file` | `python_script` | 1.41 |
| `operation_cache_warm_rerun` | `one_off_micro_probe` | `workflow_batch_spec_file` | `workflow_batch_inline` | 0.61 |

## Promotion Notes

Keep this as the deterministic gate and replace simulated fields with paired live-run observations before Codex rollout decisions.

Required live fields:

- `wall_time_ms`
- `model_turns`
- `tool_calls`
- `input_tokens`
- `output_tokens`
- `cache_namespace`
- `operation_cache_hit_rate`
- `repair_turns`
- `human_intervention_notes`

Limitations:

- Scores are formula-driven and intentionally deterministic.
- Scenario costs are calibrated from manifest-relative stressors, not measured Codex wall time.
- Operation-cache fields are reserved for live runs; this synthetic prototype does not populate namespace or hit-rate data.
