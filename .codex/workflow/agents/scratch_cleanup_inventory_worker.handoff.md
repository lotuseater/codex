# Scratch Cleanup Inventory Worker Handoff

Status: complete. Inventory only; no workflow artifacts were deleted or edited. This handoff is the only file written by this worker.

## Ignore Coverage Summary

- `.gitignore` already covers the active scratch patterns under `.codex/workflow/agents/`:
  - `*.prompt.md`
  - `*.exec.marker.txt`
  - `*.marker.txt`
  - `*.reads.report.json`
  - `*.reads.log.jsonl`
  - `*.read-report.json`
  - `*.read-log.jsonl`
  - `*-report.json`
  - `*-log.jsonl`
  - `*.report.json`
  - `*.log.jsonl`
  - `*.tmp.txt`
  - `*help.txt`
- `.gitignore` also covers root workflow scratch outputs:
  - `.codex/workflow/*.log.jsonl`
  - `.codex/workflow/*.report.json`
  - `.codex/workflow/*-output.txt`
  - `.codex/workflow/*-latest.txt`
- `git status --ignored --short .codex/workflow` showed 40 ignored scratch entries: 18 prompt files, 18 exec marker files, and 4 read/report/log JSON artifacts.
- I did not find unignored untracked prompt/marker/report/tmp scratch files before creating this handoff.
- One tracked report-style JSON is outside the current ignore suffixes: `.codex/workflow/thread-store-api-copy-report.json`. It looks like a one-shot workflow-batch copy report and references an absent `.codex/workflow/thread-store-api-copy-log.jsonl`.

## Safe-Delete Candidates

### Worker launch prompts and exec markers

These are ignored correctly and should be safe to delete after root confirms the corresponding handoff/result has been preserved:

- `app_server_boundary_finish_worker.prompt.md`
- `app_server_boundary_finish_worker.exec.marker.txt`
- `app_server_boundary_resume_worker.prompt.md`
- `app_server_boundary_resume_worker.exec.marker.txt`
- `boundary_dependency_manifest_worker.prompt.md`
- `boundary_dependency_manifest_worker.exec.marker.txt`
- `codex_otel_compile_followup_worker.prompt.md`
- `codex_otel_compile_followup_worker.exec.marker.txt`
- `compaction_output_plan_worker.prompt.md`
- `compaction_output_plan_worker.exec.marker.txt`
- `config_connectors_boundary_worker.prompt.md`
- `config_connectors_boundary_worker.exec.marker.txt`
- `core_compile_config_permissions_worker.prompt.md`
- `core_compile_config_permissions_worker.exec.marker.txt`
- `core_compile_session_thread_worker.prompt.md`
- `core_compile_session_thread_worker.exec.marker.txt`
- `core_compile_tools_worker.prompt.md`
- `core_compile_tools_worker.exec.marker.txt`
- `core_protocol_dependency_followup_worker.prompt.md`
- `core_protocol_dependency_followup_worker.exec.marker.txt`
- `core_protocol_dependency_resume_worker.prompt.md`
- `core_protocol_dependency_resume_worker.exec.marker.txt`
- `core_tests_residual_router_worker.prompt.md`
- `core_tests_residual_router_worker.exec.marker.txt`
- `dirty_tree_ownership_mapper_worker.prompt.md`
- `dirty_tree_ownership_mapper_worker.exec.marker.txt`
- `manifest_dependency_inventory_worker.prompt.md`
- `manifest_dependency_inventory_worker.exec.marker.txt`
- `recent_worker_review_worker.prompt.md`
- `recent_worker_review_worker.exec.marker.txt`
- `request_permissions_gate_worker.prompt.md`
- `request_permissions_gate_worker.exec.marker.txt`
- `scratch_cleanup_inventory_worker.prompt.md`
- `scratch_cleanup_inventory_worker.exec.marker.txt`
- `verification_matrix_planner_worker.prompt.md`
- `verification_matrix_planner_worker.exec.marker.txt`

### Read/report/log scratch artifacts

These are ignored correctly and look disposable after root confirms no handoff still depends on their raw output:

- `.core_compile_tools_worker.reads.log.jsonl`
- `.core_compile_tools_worker.reads.report.json`
- `core_tests_residual_router_worker.initial-read.log.jsonl`
- `core_tests_residual_router_worker.initial-read.report.json`

### Tracked one-shot report to inspect before removal

- `.codex/workflow/thread-store-api-copy-report.json` is tracked, small, and report-like. Root should inspect whether its copy evidence is already represented in durable handoffs, then either remove it from tracking or keep it intentionally. If future files use this `*-copy-report.json` shape, consider whether the ignore rules should cover that naming too.

## Keep Candidates

### Durable workflow state

- `.codex/workflow/solid-refactor-handoff.md` should stay tracked. It is the current aggregate handoff and was modified before this inventory.
- `.codex/workflow/solid-refactor-delegation-director-plan.md`
- `.codex/workflow/solid-refactor-subagent-contract.md`
- `.codex/workflow/worker-delegation-commit-protocol.md`
- `.codex/workflow/compaction-max-output-plan.md`
- `.codex/workflow/agents/README.md`
- `.codex/workflow/scripts/Invoke-CodexWorker.ps1`
- `.codex/workflow/scripts/Start-CodexWorker.ps1`

### Durable worker handoffs

Keep tracked or add intentionally if they are new. These are the durable replacement for prompt/marker scratch state:

- `.codex/workflow/agents/*.handoff.md`
- `.codex/workflow/agents/scratch_cleanup_inventory_worker.handoff.md`

### Baseline evidence files

These are tracked `.txt` evidence snapshots, not generic tmp files. Keep unless the root handoff has absorbed their evidence:

- `.codex/workflow/agents/baseline-domain-leaks.txt`
- `.codex/workflow/agents/baseline-protocol-leaks.txt`
- `.codex/workflow/agents/baseline-thread-store-leaks.txt`

## Suspicious Or Strange Files

- No suspicious large files found under `.codex/workflow`. Largest items were the aggregate handoff at about 49.8 KB and normal handoffs below about 22.4 KB.
- `.codex/workflow/agents/baseline-domain-leaks.txt` is about 12.5 KB and intentionally looks like grep/baseline evidence.
- `.codex/workflow/thread-store-api-copy-report.json` is the only strange tracked artifact: report-shaped JSON, root-level, and paired with a missing log path.

## Suggested Root Cleanup Order

1. Read any newly finished `*.handoff.md` files and fold the durable state into `.codex/workflow/solid-refactor-handoff.md`.
2. Add/track the handoffs root wants to preserve, including this file if the scratch inventory should remain.
3. Decide the fate of `.codex/workflow/thread-store-api-copy-report.json` before bulk cleanup.
4. Delete ignored `*.prompt.md`, `*.exec.marker.txt`, read/report/log JSON, tmp, and helper scratch files only after the corresponding handoffs are preserved.
5. Re-run `git status --ignored --short .codex/workflow` to confirm only durable workflow files remain visible and ignored scratch debris is gone.
