# MultiAgentV2 Test Plan

This plan covers the default-on MultiAgentV2 delegation slice: richer root and
subagent guidance, quality-first model/effort selection, same-terminal subagent
activity, compact/restart supervisor controls, and completed-plan follow-up
planning.

## Fast Manual Smoke After Deploy

1. Start a fresh Codex session in a small scratch repo.
2. Ask for a task that naturally splits into three independent read-only pieces:
   "Use subagents to inspect README, package config, and tests. Each worker must
   report files read, findings, and verification. Then merge the result."
3. Confirm in the terminal:
   - subagent activity is inline in the main terminal, indented, color-marked, and
     labeled with name plus `model`, `effort`, and token-used percent or placeholders.
   - root output remains visually distinct from child activity.
   - the root waits, reviews child outputs, and closes unneeded agents.
4. Ask the root to compact one idle agent and restart another with a short
   follow-up message. Confirm compact/restart rows appear in history and agent
   status remains coherent.
5. Complete a visible `update_plan` checklist. Confirm the follow-up planning
   checkpoint runs only after the current plan is done, and after self-review if
   self-review is triggered.

## WizardErasmus-Style Real Task Canary

WizardErasmus team-app probes use a fixture repo, one named worker output file
per worker, status polling, and concrete artifact checks. Adapt that as a Codex
MultiAgentV2 canary:

1. Create a temp project containing:
   - `README.md` with the target task.
   - `src/a.txt`, `src/b.txt`, `src/c.txt` with distinct facts.
   - expected output paths `worker_1_output.md`, `worker_2_output.md`,
     `worker_3_output.md`, and `final_report.md`.
2. Prompt Codex:
   - spawn three workers with stable task names.
   - each worker may inspect only one assigned source file.
   - each worker writes exactly one assigned output file with evidence and a
     verification line.
   - root reads the three outputs and writes `final_report.md`.
3. Verify:
   - all worker files exist and mention only their assigned source file.
   - the final report combines all three facts.
   - root transcript contains subagent evidence rows with labels and token use.
   - no worker broad-scans the repo unless explicitly justified.

## Automated Coverage To Keep

- `codex-core`:
  - v2 spawn model/effort override and inheritance behavior.
  - compact/restart success paths.
  - persisted spawn-edge resume/root resolution.
  - replacement-shadow automation classifiers.
- `codex-tools`:
  - v2 tool registry includes spawn/send/followup/wait/list/close/resume plus
    compact/restart.
  - default usage hints include context contracts, automation guidance, and
    quality-first model/effort selection.
- `codex-tui`:
  - subagent activity row indentation and labels.
  - inactive child command output/token updates mirrored into root history.
  - completed-plan follow-up survives blockers and clears if a plan reopens.
- `codex-app-server-protocol` and `codex-exec`:
  - compact/restart event mapping, history reconstruction, schema export, and
    JSON event names.

## Test Gaps To Add Next

- Negative `compact_agent` tests:
  - rejects root target.
  - rejects a running agent.
  - emits a clear tool error when target resolution fails.
- Negative `restart_agent` tests:
  - rejects root target.
  - validates model/effort override failure without losing the target state.
  - returns coherent status if shutdown/resume/follow-up fails.
- TUI replay tests:
  - restored sessions rebuild compact child evidence from app-server history.
  - placeholder labels update when model/effort/token metadata later arrives.
- Live canary harness:
  - scripted scratch repo and prompt runner, inspired by WizardErasmus
    `tmp_real_codex_probe_live.py` / `tmp_real_codex_late_probe.py`.
  - artifact assertions for worker output files and final root report.
  - transcript assertions for agent labels, indentation, compact/restart rows,
    and plan-completion follow-up.

## Build Notes From This Pass

- Release-only local builds are required on this checkout.
- `codex-core --release --lib replacement_shadow -j 1` passed 17 focused tests.
- Compile issues found and fixed during release verification:
  - `codex-state` parent-thread row mapping needed an `anyhow::Result` mapping.
  - rollout and rollout-trace needed explicit compact/restart event arms.
  - one multi-agent test compared `Option<ReasoningEffort>` to bare effort.
- Current warning to clean next: `ReplacementCandidate::GitWorktreeSummary` is
  used only through tests/strategy expectations right now, and the helper
  functions in `context_ops/git_worktree_summary.rs` are not reached by
  production code.
- Current warning to clean next: the old item-level inactive-agent mirror
  (`on_inactive_collab_agent_item` / `subagent_activity_history_cell`) is unused
  now that notification-level mirroring is wired. Either remove that path or add
  replay coverage that intentionally exercises it.
