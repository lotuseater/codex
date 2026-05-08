# Context Reducer Lab Results - 2026-05-08

## Scope

The sibling lab repo at `C:\Users\Oleh\Documents\GitHub\context-reducer-lab`
was extended from individual prototypes into an aggregate reducer suite. The
suite covers `first_moves`, repo scout/map, search reducers, read reducers,
git/diff reducers, run-check digests, session/process discovery, artifact-chain
handles, distillates, and scoped instruction shard selection.

Primary generated report:

`C:\Users\Oleh\Documents\GitHub\context-reducer-lab\reports\context-reducer-suite-2026-05-08.md`

Current-operation comparison report:

`C:\Users\Oleh\Documents\GitHub\context-reducer-lab\reports\op-compare-2026-05-08-current-merge2.md`

Strategy/use-case comparison report:

`C:\Users\Oleh\Documents\GitHub\context-reducer-lab\reports\strategy-compare-2026-05-08.md`

Verification commands:

```powershell
cd C:\Users\Oleh\Documents\GitHub\context-reducer-lab
cargo fmt
cargo test --release -j 1
cargo test --release --bins -j 1
cargo run --release --bin run_suite -- --report-name context-reducer-suite-2026-05-08
cargo run --release --bin op_compare -- --report-name op-compare-2026-05-08-current-merge2
cargo run --release --bin strategy_compare -- --report-name strategy-compare-2026-05-08
```

The corrected rerun produced 193 suite records. It includes the report-correctness
fixes from the review cycle: `required_paths` labels no longer score
`first_moves` candidates, prompt-scoped file scoring is recomputed per case,
path/count digest lines and top-level repo files now count toward coverage,
`??` git status entries no longer count as both unstaged and untracked,
`op_compare` preserves non-zero `rg` stdout instead of turning partial
baselines into empty 100% baselines, and `rg_count_digest` uses the shared
path/count parser for coverage. Current scout packets now also respect
`max_packet_tokens`, so over-budget scout rows are fallback-marked instead of
looking usable.

The strategy comparison scanned the 80 most recent Codex session files and
paired 37,251 shell commands with outputs. It treats GSD2/Graphify/Serena-style
ideas as use-case strategies, not command-output replacements.

The current-operation comparison was expanded with additional `first_moves`
alternatives, then pruned back to retained variants only. Removed variants were
the changed-first/topic-prior reranks, search-only seeding, and code-bias
duplicates that did not beat the baseline.

The suite now also carries a `codex_shadow_matrix` sanity check. All current
Codex replacement shadows are represented in the lab (`rg_file_set_digest`,
`rg_count_digest`, `rg_json_digest`, `select_string_digest`, `rg_files_compact`,
read reducers, git/diff shadows, run-check, process, and listing compaction);
none of those matrix records are evidence for prompt-visible promotion by
themselves.

## Decisions

| family | operation | variant | decision | corrected evidence |
| --- | --- | --- | --- | --- |
| `first_moves` | `repo_context_prior` | retained direct-comparison variants | shadow only / collect more | After removing the answer-key scoring leak, the suite still rejects old lab promotion. The expanded direct comparison keeps `path_lexical` at 57.5% average coverage and `search_seeded_component_merge` at 76.7%, both better than current source extraction at 30.0% and the `first_moves + scout` merge at 25.0%. Because the merge is still inconsistent and one search-seeded row is fallback-marked, these are useful shadow candidates, not prompt-visible replacements. |
| `repo_map` | `project_file_index` | `required_path_overlay` | lab-only promote candidate | 4/4 cases reached 100% coverage, but this is an oracle overlay using `required_paths`. It is useful as an upper-bound target, not direct runtime evidence. |
| `instruction_scope` | `scoped_instruction_shards` | `prompt_terms` | promote candidate | 4/4 cases passed, about 99% average savings. Needs a real scoped-instruction source layout before runtime promotion. |
| `session` | `session_find` | `recent_timestamp_tail` | shadow only | Single local record saved about 89%; needs more repos/tasks plus state DB and DAB validation before promotion. |
| `process` | `process_table_compact` | `codex_cargo_rustc_filter` | shadow only | Single local record saved about 98%; one process-table sample is not enough for standard replacement. |
| `search` | `rg_file_set_digest` | `baseline_digest` | keep shadow and collect more | The current-operation comparison preserved 100% file-set coverage across 12 records and averaged about 86.8% savings, but 3/12 records are fallback-marked because the raw `rg` baseline exited non-zero with useful stdout. The suite averaged 96.8% coverage. Keep this as the strongest search shadow candidate, not a direct replacement yet. |
| `search` | `rg_count_digest` | `baseline_digest` / `path_count_digest` | shadow only | Coverage is now measured from `path: count` lines. The suite averaged 96.8% coverage and the direct comparison preserved 100% file coverage, but every record is fallback-marked because counts are intentionally lossy. |
| `search` | `rg_json_digest` | `baseline_digest` | shadow only | Coverage is now measured correctly at 83.1% average, but it remains lossy and fallback-marked. |
| `search` | `rg_files_compact` | `baseline_digest` | shadow only | Newly represented Codex shadow. It saves about 94.8% in both reports, but the 80-path cap preserves only about 6.2% file coverage. Keep as observe-only inventory telemetry until it has artifact continuation or a prompt-targeted expansion path. |
| `search` | `search_text` | `grouped_cap_plus_one` | needs artifact | Suite coverage averaged 89.2%, while the direct comparison averaged 82.6% and 10/12 records required fallback/capping. Do not promote without artifact continuation. |
| `listing` | `directory_listing_compact` | `grouped_path_digest` | shadow only | Newly represented Codex shadow. It saves about 94.9%, but broad listings preserve only about 5.9% coverage under the current cap, so it is useful only as a compact scout-style signal. |
| `read` | `file_outline`, `file_excerpt_digest`, `read_file_slice` | current variants | shadow only | Good savings, but all are lossy or fallback-marked on large files. Use as first-pass or with explicit raw follow-up only. |
| `diff` | `diff_hunk_summary` | `hunk_headers_counts` | needs artifact | Intentionally lossy. Needs full diff artifact handles. |
| `diff` | `git_filtered_diff_digest` | `baseline_digest` | shadow only | Newly represented Codex shadow. It preserved file coverage in the current comparison, but most records are fallback-marked and savings can be negative on tiny diffs, so it should stay telemetry-only until raw diff artifacts exist. |
| `run_check` | `run_check_digest` | `first_errors_artifact` | needs artifact | Preserves diagnostic summaries with log handles. Needs native artifact read/search before replacement. |
| `artifact_chain` | `artifact_handle_digest` | `gsd2_style_log_handle` | needs artifact | Confirms the artifact-handle direction; not promotable until artifact store APIs exist. |
| `distillate` | `conversation_research_distillate` | `doc_heading_digest` | needs artifact | Large compression, but source artifacts and required-fact checks are mandatory. |
| `git` | `git_status_compact`, `git_worktree_summary` | parsed baseline | discard | Status/worktree replacements remain rejected. Do not revive without new live evidence. |
| `git` | diff/name/numstat compact variants | parsed baseline | shadow only | Current dirty diffs still show negative savings; keep relying on live shadow records for narrow gates. |
| `git` | `git_history_digest` | `baseline_digest` | shadow only | Newly represented Codex shadow. The suite showed 100% required-path coverage with fallback on every record; the direct comparison averaged 78.6% coverage. Keep shadow-only and use it to judge artifact-backed history reducers. |

## Strategy Comparison

| strategy | borrowed idea | current state | why |
| --- | --- | --- | --- |
| `artifact_chain` | GSD2 | shadow/artifact-required | Recent sessions show large search, diff, history, session, and check outputs. Digests are useful only if `artifact_read`/`artifact_search` can recover raw output. |
| `context_index` | Graphify, CodeSight, Aider repo map | first-moves/scout shadow | Best fit is context narrowing: prompt-to-path packets, dirty-file overlays, and architecture/review orientation. Freshness and raw-read fallback are the blockers. |
| `semantic_symbol` | Serena | explicit tool or shadow | Useful for routing Rust/code edits to definitions and references, but not for editing without exact source reads. |
| `context_compiler` | SR2 | prompt-scope candidate | Useful for stable prompt layers and budget traces; cannot remove mandatory instructions without a hard safety gate. |
| `instruction_shards` | Aspens | prompt-scope candidate | Useful for repo/path scoped instructions and skill activation; always-on policy text must remain separate. |
| `distillate` | BMAD | needs artifact | Useful for research/session handoff, but source transcript/artifact handles and required-fact checks are mandatory. |
| `snapshot_preview` | Repomix | shadow tool candidate | Useful for handoff/review bundles; freshness metadata and raw file fallback are required before stronger promotion. |

## Runtime Implications

- Do not use this lab to promote `first_moves` candidate ranking. The corrected
  data says the prior promotion result was caused by validation labels leaking
  into ranking.
- Do not promote the current `first_moves + scout` merge from these
  experiments. It performed worse than source-extracted current `first_moves`.
  The better shadow direction is lexical path scoring plus search-seeded
  component merge, with continued telemetry.
- The next Codex-side quality improvement should be real discovered-path
  prioritization: dirty files, scout output, explicitly mentioned paths, or
  artifact references. Do not substitute `required_paths` labels for runtime
  signals.
- `rg_file_set_digest` is the best current shadow candidate for search
  compression because it preserves the compared file set in the direct
  comparison. It should remain observe-only because partial `rg` baselines now
  correctly taint some records with fallback.
- External systems such as GSD2, Graphify, Serena, BMAD, SR2, Aspens, and
  Repomix should be borrowed as Codex-native shadow strategies and artifact
  flows. They should not be installed as mandatory runtime dependencies or
  benchmarked as simple raw-output replacements.
- `search_text`, session discovery, process discovery, and read reducers should
  remain shadow/tool-only until more live samples or artifact continuation make
  them exact enough.
- `rg_files_compact`, `directory_listing_compact`, `git_history_digest`, and
  `git_filtered_diff_digest` are now represented in the lab because Codex
  already has or classifies those shadows. The evidence keeps them observe-only:
  file/listing variants are heavily capped, history is lossy, and diff needs raw
  artifact continuation.
- Artifact-backed `diff`, `run_check`, and distillate reducers are still the
  largest token-saving opportunities, but they require `artifact_read` and
  `artifact_search` before replacing raw output.

## Harness Notes

- The lab now tolerates `rg` returning useful stdout with a non-zero code, which
  happens in the Serial repo because `compile_commands.json` points at a missing
  path.
- `op_compare` now keeps parseable non-zero `rg` stdout and marks downstream
  search records as fallback instead of scoring an empty baseline as 100%
  coverage.
- `rg_count_digest` coverage now uses the same `path: count` parser as other
  digest coverage checks.
- Top-level repo files such as `Cargo.toml` are accepted in count digests and
  file-set outputs instead of being dropped by slash-only path heuristics.
- File inventory scoring is prompt-scoped, so multiple cases in the same repo
  no longer reuse stale `prompt_hits` from the first case.
- Current scout comparison rows now use the same packet budget gate as lab
  alternatives; over-budget packets are marked fallback and summarized as
  `needs_artifact_or_caps`.
- The native DAB bridge canary rejects targeted foreground click/send-keys when
  the requested window is missing, preventing accidental actions in the wrong
  foreground app.
- The `codex_shadow_matrix` rows are representation checks. They should catch
  missing lab coverage for Codex shadows, but they should not be counted as
  direct quality evidence.
- Decision summaries are intentionally strict: a variant promotes only when all
  records in that group pass, no fallback is present, there are at least two
  records, and coverage is effectively complete.
- The 2026-05-08 report should be read from the corrected rerun, not the earlier
  pre-fix run that promoted `first_moves`.
