# Batch Programming Metrics Research

Generated: 2026-06-16

## Closed Status

This deliverable is closed for the scoped synthetic benchmark and R3 lab
prototype. It ties the batch-programming research memo to the repo-local fixture
and generated reports in `cases/` and `reports/`, including
`reports/batch-programming-metrics-r3-2026-06-16.md` and
`reports/batch-programming-metrics-r3-2026-06-16.json`.

It does not claim live model throughput wins. Live paired Codex runs remain the
next validation gate before any default-on rollout.

## Scope

This memo compares batch-programming variants for Codex-style local work by
quality, speed, token use, repeatability, and task fit. It focuses on practical
agent operations rather than general build-system benchmarking.

The comparison covers four lanes:

- In-agent batching: the model plans several reads, edits, or checks before the
  next user-visible step.
- `workflow_batch`: deterministic file, JSON, assertion, loop, and edit work
  executed through the local batch tool.
- TUI and bottom-pane flows: interactive batching in the Codex UI, including
  composer, footer, and bottom-pane behavior.
- External scripts and wrappers: PowerShell, Rust helpers, and other reusable
  command-line automation outside the agent turn.

## Sources Reviewed

This pass is based on local repository context and existing Codex workflow
surfaces. No network research was performed.

Local files and areas considered:

- `.codex/workflow/agents/start-codex-workers.ps1`
- `.codex/workflow/agents/start-codex-interactive.ps1`
- `scripts/build-local-codex.ps1`
- `scripts/test-local-codex-release.ps1`
- `codex-rs/tui/src/bottom_pane/mod.rs`
- `codex-rs/tui/src/bottom_pane/chat_composer.rs`
- `codex-rs/tui/src/bottom_pane/footer.rs`
- `codex-rs/tui/src/app.rs`
- `codex-rs/core/config.schema.json`
- `codex-rs/config/src/config_toml.rs`
- `codex-rs/core/src/config/config_loaders.rs`
- `codex-rs/core/src/config/config_struct.rs`
- `codex-rs/core/src/config/config_tests/loading_and_parsing.rs`
- `codex-rs/core/src/session/initial_context.rs`
- `codex-rs/core/src/session/tests/turn_flow_tests/session_context_tests.rs`
- `app-server-protocol/src/protocol/common.rs`
- `app-server-protocol/src/protocol/v2.rs`
- `docs/merge-metrics.csv`

The specifically requested review handoffs
`.codex/workflow/agents/metrics_negative_review.handoff.md` and
`.codex/workflow/agents/metrics_gap_review.handoff.md` were unavailable during
this edit pass, so this memo does not attribute any conclusions to them.

Current audit note: this pass additionally inspected the R3 external worker
handoffs
`.codex/workflow/agents/ext_batch_metrics_doc_closure_r2.handoff.md`,
`.codex/workflow/agents/ext_batch_metrics_lab_benchmark_r3.handoff.md`, and
`.codex/workflow/agents/ext_batch_metrics_codex_impl_r3.handoff.md`. That
review gap is closed for the scoped synthetic benchmark by the canonical prompt
pack and worked fixture in `cases/batch_programming_metrics_demo_20260615.json`,
the generated reports in `reports/batch-programming-metrics-demo-2026-06-15.*`,
and the independent demo-lab rerun in
`C:/Users/Oleh/Documents/GitHub/context-reducer-lab/reports/batch-programming-metrics-r3-2026-06-16.md`.

The demo artifacts are prompt/demo simulation evidence, not live model benchmark
measurements.

## Definitions

`BTE` means batch tool efficiency: the degree to which a tool can perform many
small deterministic operations with one compact instruction, one structured
result, and low conversational overhead.

`SEQ` means sequential orchestration efficiency: the degree to which a workflow
lets the agent or wrapper decide the next step from the previous result without
asking the model to re-read, re-plan, or restate context after every operation.

`BTE + SEQ` is the useful target. A high-BTE tool that cannot branch or carry
state still forces the agent to spend turns on orchestration. A high-SEQ plan
running through noisy one-off commands still burns tokens and creates fragile
parsing work. The two dimensions are coupled: BTE reduces the cost of each
operation, while SEQ reduces the number of model-visible decision points.

## Direct Comparison

| Lane | Best use | Strengths | Weaknesses | Primary metric signal |
| --- | --- | --- | --- | --- |
| In-agent batching | Small clusters of reads, decisions, and edits where judgment matters | Preserves semantic context, adapts quickly, no extra artifact needed | Easy to over-pack unrelated work; deterministic steps still consume model attention | Fewer turns with equal or better final quality |
| `workflow_batch` | File/JSON transforms, bounded scans, assertions, safe repeated edits | Compact, deterministic, structured reports, reusable logs | Poor fit for ambiguous reasoning, command execution, live process state, or high-level design choices | Lower token use and lower wall time for repeated local operations |
| TUI/bottom pane | Human-in-the-loop sessions, command composition, visible state, approval and interruption flows | Optimizes operator ergonomics and keeps interaction state visible | Harder to benchmark as pure batch work because human latency and UI state dominate | Lower correction rate and fewer abandoned or duplicated interactions |
| External scripts/wrappers | Stable workflows that must be rerun, shared, or launched outside one agent turn | Fast reruns, explicit dependencies, easy CI or shell integration | Can hide context, drift from agent assumptions, and need maintenance | Repeatability across changed inputs and clean exit/status reporting |

The lanes are not substitutes for one another. In-agent batching decides what
should happen. `workflow_batch` executes deterministic local work. TUI batching
keeps interactive state usable. External scripts preserve workflows that have
proven stable enough to reuse outside a single conversation.

## Metric Schema

Use the same fixture and record these fields for every lane:

- `quality_score`: result correctness against a hand-checked expected output.
- `wall_time_ms`: elapsed time from start of lane execution to usable result.
- `model_turns`: number of model-visible decision turns required.
- `tool_calls`: number of tool invocations or external command launches.
- `tokens_total`: prompt, context, payload, and continuation tokens.
- `repeatability`: whether the same lane can rerun after changed input without
  manual reconstruction.
- `diagnosability`: how quickly a failed step points to a specific file,
  assertion, command, or user action.
- `task_fit`: whether the lane naturally matches the work shape.

The key derived metrics are:

- `BTE`: useful deterministic operations per model-visible tool call.
- `SEQ`: useful branch decisions completed before the next model turn.
- `BTE+SEQ score`: normalized combination of BTE, SEQ, quality, and
  diagnosability, with a penalty for hidden failure modes.
- `evidence_class`: provenance for the row, using one of
  `observed_local_result`, `tool_contract_inference`, `benchmark_hypothesis`, or
  `operator_observation`. Do not compare a measured row and a hypothesis row as
  if they have the same weight.

## Demo Benchmark

A useful demo benchmark should distinguish in-agent thought batching from
`workflow_batch` deterministic execution instead of treating both as "do more at
once."

Fixture:

1. Create three small Markdown files and one JSON file under a temporary
   benchmark directory.
2. Ask each lane to extract headings, count TODO markers, normalize one JSON
   field, and assert that every Markdown file has exactly one H1.
3. Introduce a second run where one Markdown file is malformed and the JSON
   field has a changed name.
4. Record raw results and logs for each lane.

Lane setup:

- In-agent batching: the model plans and sequences the reads/checks directly,
  using ordinary shell reads or targeted commands only as needed.
- `workflow_batch`: one inline spec performs reads, map/filter/reduce style
  checks, JSON normalization, and assertions, returning a compact report.
- TUI/bottom pane: the same task is run as an interactive operator flow, with
  corrections and visible state transitions counted.
- External script/wrapper: a checked-in or temporary script performs the same
  fixture work and reports machine-readable output.

Expected differentiation:

- In-agent batching should score well when the changed-input case requires
  interpretation or a new rule.
- `workflow_batch` should score well when the work is already deterministic and
  bounded.
- TUI flow should expose usability costs that pure command metrics miss.
- External scripts should win on rerun speed once the workflow stabilizes, but
  lose if setup time or hidden assumptions are included.

The benchmark should report both the clean success case and the changed-input
case. A lane that is fast only when the input is perfect is not generally better
for Codex work.

### Canonical Demo And Simulation Prompts

Use the same normalized task for every lane, with only the execution wrapper
changed. That keeps quality, speed, token use, and coordination overhead
comparable.

Base task prompt:

```text
In the temporary benchmark directory, inspect three Markdown files and one JSON
file. Return a compact report with: each Markdown H1, TODO count per Markdown
file, the normalized JSON owner field, and a pass/fail assertion that every
Markdown file has exactly one H1. For the changed-input run, diagnose the first
schema or heading mismatch without repairing unrelated content.
```

Variant wrappers:

- In-agent batching: "Plan the minimum reads first, then execute the related
  checks in as few model-visible turns as you can while preserving failure
  detail."
- `workflow_batch`: "Use one inline batch spec for file reads, JSON parsing,
  TODO counting, H1 assertions, and report emission. Stop at the first failed
  assertion with path-level detail."
- TUI/bottom-pane flow: "Run the same task through the interactive operator
  path. Count prompt revisions, visible state corrections, and any duplicated
  action caused by composer or footer ambiguity."
- External script/wrapper: "Write or invoke a temporary reusable script that
  performs the same checks, emits JSON, and can rerun unchanged against the
  changed-input fixture."
- Hybrid scout-plus-batch: "Use one short scouting turn to identify branch
  conditions, then hand deterministic reads and assertions to `workflow_batch`
  or a script. Count both scout cost and batch execution cost."

Worked fixture shape:

```text
benchmark/
  alpha.md     # one H1, two TODO markers
  beta.md      # one H1, no TODO markers
  gamma.md     # clean run: one H1; changed run: zero or two H1s
  config.json  # clean run: {"owner":"ops"}; changed run: {"maintainer":"ops"}
```

Expected clean result:

- Quality passes only if all three H1s, all TODO counts, and owner normalization
  are correct.
- Speed and token metrics start when the lane receives the base task prompt and
  stop when a usable report is available.
- Diagnosability passes only if the report points to the exact malformed
  Markdown file or changed JSON field in the changed-input run.

Evaluation rubric:

- 40% quality: correct extraction, normalization, assertions, and changed-input
  diagnosis.
- 20% speed: wall-clock time to a usable report, including setup and rerun time.
- 15% token cost: total prompt, context, payload, and correction tokens.
- 15% diagnosability: path-level or field-level failure evidence without log
  archaeology.
- 10% coordination overhead: model turns, prompt revisions, duplicated actions,
  and handoff friction.

Evidence separation:

- Facts: raw elapsed time, tool calls, report paths, produced assertions, and
  observed failure messages from an actual run.
- Assumptions: expected lane strengths before running the fixture.
- Recommendations: lane choices made after reading both the raw metrics and the
  changed-input failure behavior.

### Demo Lab Result

The repo-local demo fixture generated a simulated 7-variant, 4-task comparison
on 2026-06-15 at 18:23:26 local time. It produced 28 rows in
`reports/batch-programming-metrics-demo-2026-06-15.json`. The manifest is
`cases/batch_programming_metrics_demo_20260615.json`.

Average simulated results, sorted by composite score:

| Variant | Avg elapsed ms | Avg tokens | Avg quality | Avg composite | Repair turns |
| --- | ---: | ---: | ---: | ---: | ---: |
| Checked-in `workflow_batch` spec file | 5650 | 2126 | 95 | 77.20 | 1 |
| Hybrid scout plus local batch | 5795 | 2151 | 95 | 76.42 | 1 |
| Small Python script batch | 5887.5 | 2121 | 94 | 75.90 | 1 |
| Inline `workflow_batch` | 6037.5 | 2101 | 94 | 75.26 | 1 |
| Focused shell batch | 8725 | 2086 | 91 | 66.98 | 1 |
| Delegated worker batch | 10262.5 | 2210 | 88 | 63.97 | 2 |
| Interactive sequential tools | 14200 | 2073 | 89 | 63.59 | 2 |

The simulated run ranked the checked-in `workflow_batch` spec file first for all
four fixture tasks: file inventory reduction, JSON normalization, Markdown
audit, and mechanical patching. The margin over inline `workflow_batch` is small
enough that the practical recommendation should not be "always write a spec
file." Instead, the demo supports this rule:

- Use inline `workflow_batch` for one-off deterministic reads, reductions,
  assertions, and bounded scans.
- Promote to a checked-in spec only after the same operation repeats, becomes
  part of a workflow, or needs reproducible failure evidence.
- Use a small script when the operation needs richer algorithms, libraries,
  command execution, or durable behavior outside Codex.
- Delegate to a worker when the expensive part is judgment, review, or test
  triage rather than deterministic file transformation.

### Observed Merge-Scale Signal

`docs/merge-metrics.csv` provides a real local batch-work observation that is
not directly comparable to the simulated demo rows, but it is useful for
calibrating delegated-worker and external-wrapper expectations.

Facts:

- The 2026-06-05 and 2026-06-10 rows both record 117 conflicts: 100 content
  conflicts, 17 modify/delete cases, and 17 slices.
- The 2026-06-10 row records a successful result after 8 build-fix waves and
  829 wall-clock minutes.
- The same row notes 78 build errors handled by Fable workers, legacy-core
  deduplication, schema regeneration, and a session restart that added roughly
  150 minutes.

Inference:

- Delegated worker batching is viable for large judgment-heavy repair work, but
  its metric profile is dominated by coordination latency, repair waves, and
  interruption recovery. That makes it a poor baseline for deterministic BTE
  comparisons against `workflow_batch` or a small script.

Recommendation:

- Treat merge-scale worker runs as `operator_observation` evidence unless each
  slice emits raw per-lane timings, prompt counts, tool calls, and failure
  reports. Use the observations to design live validation, not to claim a
  deterministic throughput win.

## Interpretation

The main finding is that batching has two different jobs:

1. Reduce mechanical overhead for deterministic work.
2. Preserve enough context to make the next decision correctly.

`workflow_batch` is strongest for the first job. It compresses repeated local
operations into one structured execution, which improves BTE. It can also
improve SEQ when assertions and branches are encoded directly in the spec, but
it should not be treated as a reasoning engine.

In-agent batching is strongest for the second job. It improves SEQ by allowing
the agent to carry intent across several related steps, but it does not
automatically improve BTE if the agent still performs noisy one-off reads or
commands.

External scripts and wrappers become attractive after a pattern repeats. They
can combine BTE and SEQ by making both the operation and the branch policy
explicit. Their downside is that they create another artifact that must stay in
sync with the repo and with the agent's assumptions.

TUI and bottom-pane work should be judged separately from pure backend batch
execution. Their value is not just throughput. They reduce operator mistakes,
make interruption and continuation visible, and can prevent duplicated work when
the user and agent share live state.

## Negative And Caveat Cases

Batching can make outcomes worse in several common cases:

- Ambiguous tasks: batching too early can encode the wrong interpretation and
  multiply the cost of a bad assumption.
- Large unbounded scans: a batch can hide excessive file reads or produce a
  compact report that lacks the evidence needed to debug the result.
- Live processes: command output, timing, ports, windows, and interactive state
  often need incremental observation.
- Destructive edits: repeated moves, deletes, or broad rewrites need narrow
  guardrails and explicit path checks.
- Review work: batching can bury the exact line evidence reviewers need.
- UI work: the TUI may look correct in static output while the actual operator
  flow is broken.
- Stable wrappers: scripts can outlive their assumptions and keep succeeding
  locally while no longer measuring the real workflow.

The practical rule is to batch deterministic work after the branch conditions
are known. Use smaller agent-visible steps while the problem is still being
understood.

## Recommendations

- Use in-agent batching for related reasoning steps where the agent must keep
  intent, constraints, and tradeoffs in view.
- Use `workflow_batch` for deterministic file, JSON, assertion, and bounded
  repeated edit work.
- Use the hybrid scout plus local batch lane when routing is uncertain but the
  discovered operation is deterministic.
- Promote a `workflow_batch` spec or shell prototype into an external script only
  after it has been rerun successfully on changed input.
- Keep TUI and bottom-pane metrics tied to operator outcomes: correction rate,
  duplicated action rate, interruption recovery, and visible state quality.
- Report BTE and SEQ together. A lane with high BTE and poor SEQ is a fast
  mechanical primitive, not a complete workflow improvement.
- Keep failure evidence close to the result. Compact summaries are useful only
  when they preserve enough path, assertion, and branch detail to fix failures.

## Implementation Consequences For Codex

The research is now tied to a small Codex rollout consequence rather than only a
standalone memo:

- Add a `batch_mini_programming_instructions` config surface with `off` as the
  default and `always` as an explicit opt-in. This keeps the instruction pack
  from adding default prompt cost until a profile deliberately enables it.
- Keep the instruction text scoped to deterministic local file, JSON,
  assertion, loop, and reduction work. It should steer agents toward
  `workflow_batch` when the operation is already clear, not replace reasoning,
  shell commands, scripts, or user clarification.
- Inject the instructions through the session initial-context path only when
  the config asks for them and the tool surface can support the workflow.
- Cover both config parsing and session-context injection with focused tests so
  the rollout can stay default-off while still being product behavior, not just
  a document.

This scoped implementation does not add live productivity telemetry, an
auto-batching scheduler, or TUI-specific metrics. Those are separate product
features that need their own design. The consequence landed here is smaller and
actionable: make the guidance configurable, testable, and ready for dogfood
profiles that already use the local batch tool.

## Promotion Boundaries

Treat the repo-local demo as closed simulation evidence for this scoped
deliverable. The remaining rows below are promotion boundaries for future
production claims and are outside this memo's completed scope.

| Gate | Required Evidence | Decision |
| --- | --- | --- |
| Local fixture | Repo-local rows include quality, elapsed time, token estimate, tool calls, and recovery counts for every lane. | Done for the 2026-06-15 fixture. |
| Changed input | The same runner succeeds on at least one changed manifest without rewriting the scorer. | Outside this deliverable; required before promoting any script or template. |
| Codex dogfood | A default-off config profile injects the mini-programming guidance and focused tests prove off/always behavior. | Outside this deliverable; first guarded rollout slice only. |
| Live operator flow | TUI or external-worker runs emit comparable raw timings, prompt counts, tool calls, and failure detail. | Outside this deliverable; no live throughput claim. |

## Validation Status

The original design-only gap is closed for a narrow, local fixture by the
repo-local prompt/demo simulation:

- Fixture:
  `cases/batch_programming_metrics_demo_20260615.json`.
- Raw report:
  `reports/batch-programming-metrics-demo-2026-06-15.json`.
- Human-readable report:
  `reports/batch-programming-metrics-demo-2026-06-15.md`.
- Merge-scale corroboration: `docs/merge-metrics.csv`.

That evidence is enough to support the operating rule above: use local
deterministic batching first when paths and operations are known; add scout
context when routing is uncertain; escalate to durable scripts only after
repeated or changed-input runs justify the promotion.

The prototype evidence is sufficient for a default-off dogfood path: it covers
the prompt variants, deterministic repo and demo-lab tasks, quality gates,
elapsed time, and token accounting that this memo set out to compare. The
2026-06-16 R3 demo-lab rerun is the follow-up prototype evidence for this
decision. Broader product instrumentation remains a separate rollout gate. The
still-unmeasured cases are a TUI/bottom-pane operator flow, a fully live
external worker run, and a promoted script rerun on changed input.

The Codex-side implementation consequence is intentionally narrow: config,
schema, initial-context injection, and focused tests for the default-off
instruction pack. That is enough to make batch mini-programming guidance
dogfoodable while preserving the memo's caveat that broad speed and quality
claims still require live task evidence before a default-on rollout.
