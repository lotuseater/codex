# Batch Programming Metrics Demo

Generated: 2026-06-16T00:08:15Z

Status: closed for the scoped synthetic fixture in
`cases/batch_programming_metrics_demo_20260615.json`.

This completed report is a deterministic prompt/demo simulation. It compares
variant shape, prompt overhead, diagnosability, quality risk, and simulated
wall-clock behavior; it does not replace live paired model runs.

## Variant Summary

| Rank | Variant | Avg elapsed ms | Avg est. tokens | Avg quality | Avg composite | Avg repair turns |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 1 | Checked-in workflow_batch spec file | 5220.4 | 1866.0 | 94.4 | 78.67 | 1.0 |
| 2 | Hybrid scout plus local batch | 5375.6 | 1891.0 | 94.6 | 77.91 | 1.0 |
| 3 | Inline workflow_batch spec | 5440.8 | 1841.0 | 93.6 | 77.22 | 1.0 |
| 4 | Purpose-built Python script | 5614.0 | 1861.0 | 93.6 | 77.04 | 1.0 |
| 5 | Focused shell/rg batch | 7525.0 | 1826.0 | 90.8 | 70.31 | 1.0 |
| 6 | Interactive sequential tools | 11706.4 | 1813.0 | 91.0 | 69.07 | 1.6 |
| 7 | Delegated worker batch | 9835.2 | 1950.0 | 87.2 | 64.71 | 2.0 |

## Decision Cases

The table below records the selected winner for each prompt case and the
simulated comparison rows that drove that selection.

| Case | Task | Winner | Compared against | Composite delta | Winner elapsed ms | Compared elapsed ms | Winner tokens | Compared tokens |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| batching_overhead | One-off direct file check | Interactive sequential tools (90.99) | Inline workflow_batch spec (85.06) | 5.93 | 1732 | 3054 | 773 | 801 |
| batching_win | Repo-visible file inventory with summaries | Checked-in workflow_batch spec file (79.07) | Interactive sequential tools (64.17) | 14.9 | 5086 | 12226 | 1926 | 1873 |

## Task Rows

| Task | Variant | Sim elapsed ms | Est. tokens | Quality | Composite | Notes |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Repo-visible file inventory with summaries | Checked-in workflow_batch spec file | 5086 | 1926 | 95 | 79.07 | Fastest repeat path once the operation is stable; overkill for one-off probes. |
| Repo-visible file inventory with summaries | Hybrid scout plus local batch | 5240 | 1951 | 95 | 78.33 | Best general default when the file set is uncertain but the operation becomes deterministic after routing. |
| Repo-visible file inventory with summaries | Purpose-built Python script | 5394 | 1921 | 94 | 77.66 | Best for reusable metrics and richer data structures; initial setup cost is higher. |
| Repo-visible file inventory with summaries | Inline workflow_batch spec | 5356 | 1901 | 94 | 77.36 | Best canary path for repeated deterministic local work before promoting a reusable spec. |
| Repo-visible file inventory with summaries | Focused shell/rg batch | 7550 | 1886 | 91 | 69.71 | Good for read-only probes; weaker for structured transforms and per-step assertions. |
| Repo-visible file inventory with summaries | Delegated worker batch | 9534 | 2010 | 88 | 64.56 | Useful for parallel review or long-running tests; inefficient for tiny local deterministic work. |
| Repo-visible file inventory with summaries | Interactive sequential tools | 12226 | 1873 | 89 | 64.17 | Transparent, but slow and token-heavy on repeated deterministic operations. |
| Normalize JSON fixtures and verify schema keys | Checked-in workflow_batch spec file | 6094 | 1526 | 95 | 77.73 | Fastest repeat path once the operation is stable; overkill for one-off probes. |
| Normalize JSON fixtures and verify schema keys | Hybrid scout plus local batch | 6232 | 1551 | 95 | 76.81 | Best general default when the file set is uncertain but the operation becomes deterministic after routing. |
| Normalize JSON fixtures and verify schema keys | Purpose-built Python script | 6276 | 1521 | 94 | 76.54 | Best for reusable metrics and richer data structures; initial setup cost is higher. |
| Normalize JSON fixtures and verify schema keys | Inline workflow_batch spec | 6574 | 1501 | 94 | 75.36 | Best canary path for repeated deterministic local work before promoting a reusable spec. |
| Normalize JSON fixtures and verify schema keys | Focused shell/rg batch | 9650 | 1486 | 91 | 66.87 | Good for read-only probes; weaker for structured transforms and per-step assertions. |
| Normalize JSON fixtures and verify schema keys | Delegated worker batch | 10836 | 1610 | 88 | 65.46 | Useful for parallel review or long-running tests; inefficient for tiny local deterministic work. |
| Normalize JSON fixtures and verify schema keys | Interactive sequential tools | 15754 | 1473 | 89 | 65.07 | Transparent, but slow and token-heavy on repeated deterministic operations. |
| Markdown evidence audit and gap report | Checked-in workflow_batch spec file | 5266 | 2326 | 95 | 77.55 | Fastest repeat path once the operation is stable; overkill for one-off probes. |
| Markdown evidence audit and gap report | Hybrid scout plus local batch | 5417 | 2351 | 95 | 76.81 | Best general default when the file set is uncertain but the operation becomes deterministic after routing. |
| Markdown evidence audit and gap report | Purpose-built Python script | 5552 | 2321 | 94 | 76.14 | Best for reusable metrics and richer data structures; initial setup cost is higher. |
| Markdown evidence audit and gap report | Inline workflow_batch spec | 5574 | 2301 | 94 | 76.02 | Best canary path for repeated deterministic local work before promoting a reusable spec. |
| Markdown evidence audit and gap report | Focused shell/rg batch | 7925 | 2286 | 91 | 67.53 | Good for read-only probes; weaker for structured transforms and per-step assertions. |
| Markdown evidence audit and gap report | Delegated worker batch | 9766 | 2410 | 88 | 63.48 | Useful for parallel review or long-running tests; inefficient for tiny local deterministic work. |
| Markdown evidence audit and gap report | Interactive sequential tools | 12856 | 2273 | 89 | 63.09 | Transparent, but slow and token-heavy on repeated deterministic operations. |
| Small mechanical code/doc patch with assertions | Checked-in workflow_batch spec file | 6154 | 2726 | 95 | 74.45 | Fastest repeat path once the operation is stable; overkill for one-off probes. |
| Small mechanical code/doc patch with assertions | Hybrid scout plus local batch | 6291 | 2751 | 95 | 73.75 | Best general default when the file set is uncertain but the operation becomes deterministic after routing. |
| Small mechanical code/doc patch with assertions | Purpose-built Python script | 6328 | 2721 | 94 | 73.26 | Best for reusable metrics and richer data structures; initial setup cost is higher. |
| Small mechanical code/doc patch with assertions | Inline workflow_batch spec | 6646 | 2701 | 94 | 72.3 | Best canary path for repeated deterministic local work before promoting a reusable spec. |
| Small mechanical code/doc patch with assertions | Focused shell/rg batch | 9775 | 2686 | 91 | 63.81 | Good for read-only probes; weaker for structured transforms and per-step assertions. |
| Small mechanical code/doc patch with assertions | Delegated worker batch | 10914 | 2810 | 88 | 62.4 | Useful for parallel review or long-running tests; inefficient for tiny local deterministic work. |
| Small mechanical code/doc patch with assertions | Interactive sequential tools | 15964 | 2673 | 89 | 62.01 | Transparent, but slow and token-heavy on repeated deterministic operations. |
| One-off direct file check | Interactive sequential tools | 1732 | 773 | 99 | 90.99 | Transparent, but slow and token-heavy on repeated deterministic operations. Micro-probe winner because direct inspection has less setup overhead. |
| One-off direct file check | Inline workflow_batch spec | 3054 | 801 | 92 | 85.06 | Best canary path for repeated deterministic local work before promoting a reusable spec. Batch setup dominates a one-check task. |
| One-off direct file check | Checked-in workflow_batch spec file | 3502 | 826 | 92 | 84.56 | Fastest repeat path once the operation is stable; overkill for one-off probes. Spec-file setup is not justified for one fact path. |
| One-off direct file check | Hybrid scout plus local batch | 3698 | 851 | 93 | 83.87 | Best general default when the file set is uncertain but the operation becomes deterministic after routing. Scouting overhead is unnecessary when the exact path is known. |
| One-off direct file check | Focused shell/rg batch | 2725 | 786 | 90 | 83.62 | Good for read-only probes; weaker for structured transforms and per-step assertions. Slightly overbuilt for one direct fact. |
| One-off direct file check | Purpose-built Python script | 4520 | 821 | 92 | 81.62 | Best for reusable metrics and richer data structures; initial setup cost is higher. Script setup is reusable only after repeated probes. |
| One-off direct file check | Delegated worker batch | 8126 | 910 | 84 | 67.66 | Useful for parallel review or long-running tests; inefficient for tiny local deterministic work. Delegation overhead dominates tiny deterministic work. |

## Task Prompt Templates

### Repo-visible file inventory with summaries

Inventory repo-visible paths from {path_manifest}, reduce them by extension and ownership, assert no duplicate paths, and write a compact evidence summary with artifact paths.

### Normalize JSON fixtures and verify schema keys

Normalize {fixture_paths} as JSON, preserve schema keys, emit changed artifact paths, and assert required keys plus row counts before reporting success.

### Markdown evidence audit and gap report

Audit {markdown_paths} against {evidence_paths}, separate verified facts from inferences, cite artifact paths, and produce a gap report with concrete closure criteria.

### Small mechanical code/doc patch with assertions

Apply the same mechanical edit pattern to {target_paths}, keep unrelated changes intact, then run bounded assertions and summarize changed files plus verification commands.

### One-off direct file check

Check {single_path} for one exact symbol or key, report whether it exists, and cite the line or JSON path using the simplest direct command that returns the fact.


## Prompt Packets

### Interactive sequential tools

Inspect each file or query one at a time. Summarize after every result and decide the next command interactively.

### Focused shell/rg batch

Use one focused shell batch for independent reads/searches, then inspect only the returned files that matter.

### Inline workflow_batch spec

Use workflow_batch for deterministic file/JSON reads, transforms, bounded scans, assertions, and a compact report.

### Checked-in workflow_batch spec file

Run a reviewed workflow_batch spec_path with named reports and assertions; update the spec only after a canary passes.

### Purpose-built Python script

Write or reuse a small Python runner for richer algorithms, fixture generation, or reusable metrics; emit JSON and Markdown reports.

### Delegated worker batch

Delegate an isolated implementation, review, or test triage packet with exact files, verification, and a short handoff.

### Hybrid scout plus local batch

Use first-moves/context scouting to choose exact files, then run workflow_batch or a script for deterministic operations.
