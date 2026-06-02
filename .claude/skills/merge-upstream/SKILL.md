---
name: merge-upstream
description: >-
  Run the fork's full upstream-merge runbook: sync openai/codex `upstream/main` into the
  fork branch `slow-context-budget-mode`, reduce conflicts via pre-merge extractions, fan
  out the `merge-conflict-resolver` subagent on disjoint conflicted-file slices under a
  strict NO-BUILD rule, then verify, build, and deploy. Trigger when the user asks to
  "merge upstream", "sync upstream/main", "pull upstream into the fork", "do the upstream
  merge", or "update the fork from openai/codex". Side-effecting (mutating git + builds),
  so user-invoked only.
disable-model-invocation: true
---

# merge-upstream — upstream/main → fork merge runbook

You (the main agent) are the **orchestrator**. This skill codifies the practiced 6-step
merge process for the Codex Rust fork on Windows (pwsh). Repo root (use ABSOLUTE paths):
`C:\Users\Oleh\Documents\GitHub\open_ai\codex`. Branch: `slow-context-budget-mode`.
Remotes: `origin` = push target (lotuseater/codex), `upstream` = openai/codex.

Source-of-truth docs to consult (do not duplicate them — point resolvers at them):
- Process: `.codex/workflow/HANDOFF_merge.md`, `Main_Merge_Prompt.md`
- Resolver policy: `.codex/tmp/merge_resolution_brief.md`
- Fork features + owner crates + per-feature health checks: `docs/fork-feature-inventory.md`
- Resolver subagent: `.claude/agents/merge-conflict-resolver.md` (model: opus)

> ## ⛔ INVARIANTS (never violate)
> 1. **NO build until Step 5.** No `cargo build/check/test/clippy/nextest`, no
>    `scripts\build-*`, no `just test` until every resolver reports success and
>    `git diff --check` shows ZERO markers. Git restore points are the safety net.
> 2. **Resolvers own DISJOINT slices only.** One `merge-conflict-resolver` per slice;
>    slices never share a file. Interleaving edits are the main conflict source.
> 3. **Preserve EVERY fork feature** whose surface a resolution touches, per
>    `docs/fork-feature-inventory.md`. Union-by-default; take-upstream only for pure
>    upstream refactors the fork doesn't own (then re-wire call-sites).
> 4. **Use `git rerere`** so resolved hunks are remembered across re-rehearsals/retries.
> 5. **Bank a handoff before any compaction.** Refresh `.codex/tmp/merge_preflight_<date>.md`
>    or `COMPACT_HANDOFF.md` with current state + ordered next steps + files to stage.
> 6. **Subagents may run in divergent worktrees** → always hand them ABSOLUTE main-repo
>    paths, or their edits never reach the live tree.
> 7. Commits are BLOCKED unless `mcp__wizard__analyze_git_diff` ran first — call it before
>    every commit.

Let `<date>` = `yyyy-MM-dd` throughout (e.g. logs and preflight notes are date-stamped).

---

## Step 1 — Preflight (READ-ONLY: no merge, no build)

1. Fetch upstream:
   ```
   git fetch upstream main
   ```
2. Regenerate the hotspot map and detect un-seamed hot files (both read-only):
   ```
   powershell -ExecutionPolicy Bypass -File scripts\generate-merge-hotspot-map.ps1
   powershell -ExecutionPolicy Bypass -File scripts\detect-adapter-gaps.ps1 -OutPath .codex\tmp\adapter-gaps_<date>.md
   ```
   - `generate-merge-hotspot-map.ps1` rewrites `docs/merge-hotspot-map.md` (top 60 by
     `HotspotScore = (MergeTouches+1)*(UpstreamChurn+1)*RiskWeight`; pass `-Top N` /
     `-JsonOut <path>` for more).
   - `detect-adapter-gaps.ps1` flags upstream-hot, fork-heavy `.rs` files with no
     `*_local.rs`/`*_adapter.rs`/`*_fork.rs` seam (default `-MinForkCommits 3`). **Exit code
     1 = gaps found** (it's a CI gate) — that's the signal, not an error.
3. Rehearse the merge WITHOUT touching the tree, and count conflict markers:
   ```
   git merge-tree $(git merge-base HEAD upstream/main) HEAD upstream/main
   ```
   Pipe the output through a marker count (PowerShell):
   `... | Select-String '^(<<<<<<<|>>>>>>>|=======)$' | Measure-Object` — record the count.
   (`analyze-branch-conflict-surface.ps1` gives an area-weighted risk score for the same
   divergence if you want a second view.)
4. Verify rerere is on (enable if the user agrees):
   ```
   git config rerere.enabled
   git config rerere.enabled true        # if it returns nothing / false
   ```
5. **Record the baseline** to `.codex/tmp/merge_preflight_<date>.md`: HEAD sha, upstream tip,
   merge-base, ahead/behind, conflict-marker count, adapter-gap list, top hotspots.
6. **Present to the user**: the conflict count and the adapter-gap list, then proceed.

---

## Step 2 — Pre-merge extraction (OPTIONAL, gated; still NO build)

**Gate:** only if the Step-1 conflict count exceeds the threshold (**default ~80**) OR
`detect-adapter-gaps.ps1` flagged un-seamed hot files.

When gated in, surface the top targets and **ask the user before extracting** (this rewrites
fork files). Candidate structural extractions (the deferred ones — check with Glob whether
the seam already exists before proposing it):
- `protocol/src/protocol/op.rs` → `op_fork.rs` (move fork-only `Op` variants behind a seam)
- `core/src/session/turn.rs` → `hook_input_gate.rs` / `multi_task_runner.rs`
- `features/src/lib.rs` → `fork_features.rs` (group fork `Feature` variants)
- `core/src/session/input_queue.rs` + `session_mailbox.rs` → extract toward `codex-input-queue`

Each extraction isolates fork divergence into a seam file so future upstream edits land in
the upstream-shaped file with no conflict. After extracting:
1. Re-rehearse Step 1's `git merge-tree` and re-count markers — expect a reduction.
2. `mcp__wizard__analyze_git_diff`, then commit a restore point:
   ```
   git commit -m "refactor(pre-merge): conflict-reduction extractions"
   ```

If the gate isn't tripped (or the user declines), skip straight to Step 3.

---

## Step 3 — Merge under the NO-BUILD rule

1. Start the merge, leave it uncommitted:
   ```
   git merge upstream/main --no-commit
   ```
2. Enumerate conflicted files:
   ```
   git diff --name-only --diff-filter=U
   ```
3. **Partition into DISJOINT area slices** (one resolver per slice; never split a file
   across two slices). Standard areas:
   `core-session` · `core-tools` · `tui-app` · `tui-composer` · `protocol` ·
   `app-server-protocol` · `config-features` · `manifests` (Cargo.toml/lock, MODULE.bazel).
   Keep an interleaved region whole inside ONE slice. Split DEEP — prefer several lean
   resolvers over a few heavy ones.
4. **Fan out the `merge-conflict-resolver` subagent on the slices.**
   - **PREFER a Workflow script** (`parallel` / `pipeline` over the slices, one resolver per
     slice, each phase-labeled) so they run concurrently and survive compaction.
   - **Fall back** to concurrent `Agent` calls (multiple `merge-conflict-resolver`
     invocations in one message) if a workflow isn't set up.
   - Each resolver prompt MUST: list its exact conflicted paths (ABSOLUTE), point at
     `.codex/tmp/merge_resolution_brief.md` + `docs/fork-feature-inventory.md`, and require
     the **HANDOFF CONTRACT** block back:
     ```
     HANDOFF_STATUS: success | partial | blocked
     FILES_RESOLVED:
       - <path> | <strategy: union|take-fork|take-upstream|structural> | fork_feature_preserved: <name|none> | uncertainty: <low|med|high>
     FILES_NEEDING_REGEN:
       - <path>            # e.g. export.rs / schema JSON — orchestrator regenerates post-merge
     FILES_UNCERTAIN:
       - <path> | <why>    # needs human review
     MARKERS_REMAINING: <int>   # must be 0 for success
     ```
5. **Do NOT run any build** until EVERY resolver returns `HANDOFF_STATUS: success` AND:
   ```
   git diff --check
   ```
   shows zero conflict markers. If any resolver is `partial`/`blocked`, re-slice the leftover
   files and respawn a fresh resolver (never reuse one across batches).
6. **Regenerate union/generated files** flagged under `FILES_NEEDING_REGEN` (e.g.
   `app-server-protocol/src/export.rs` + schema JSON) — see Step 5 regen commands; do the
   regen, don't hand-finish generated files.

---

## Step 4 — Fix-up (still pre-build)

1. Catch stray unstaged edits the resolvers left:
   ```
   git diff --name-only
   ```
2. Stage everything for the merge and bank a correction commit:
   ```
   git add -A
   ```
   `mcp__wizard__analyze_git_diff`, then:
   ```
   git commit -m "fix(merge): post-resolution corrections"
   ```
   (Or keep staged and fold into the final merge commit in Step 6 — either is fine.)
3. **Surface every `FILES_UNCERTAIN`** entry from the resolver reports to the user for human
   review before building.

---

## Step 5 — Verify (FIRST build allowed)

1. Workspace check, release-only, captured to a log:
   ```
   powershell -ExecutionPolicy Bypass -Command "cargo check --workspace --release --keep-going *>&1 | Tee-Object logs\merge-cargo-check-<date>.log"
   ```
2. **Grep the log — a background `exit 0` is FALSE-GREEN** (a trailing echo masks cargo's
   real exit). Confirm success only by inspecting the log:
   ```
   Select-String -Path logs\merge-cargo-check-<date>.log -Pattern 'EXITCODE=|error\[|error:|warning: unused'
   ```
   Treat any `error[`/`error:` or non-zero `EXITCODE=` as failure.
3. If broken, **fan out compile-fixers per broken crate** (one fresh worker per crate,
   file-disjoint), then re-check. (`cargo check --release` skips `#[cfg(test)]`; if a test
   split was touched, verify with `cargo check --tests -p <crate>`.)
4. Regenerate schemas/lockfiles whose inputs changed:
   ```
   just write-config-schema        # if ConfigToml changed
   just write-app-server-schema    # app-server-protocol schema (TS/JSON fixtures)
   just bazel-lock-update          # if Cargo.toml/Cargo.lock changed
   ```
   Re-run Step 5.1–5.2 until the log is clean.

---

## Step 6 — Deploy

1. Build + deploy the local binary (memory-aware):
   ```
   powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode FastRelease
   ```
   Use `-Mode LowMemRelease` under memory pressure on this machine.
2. Smoke-test the changed owner crates (per `docs/fork-feature-inventory.md`):
   ```
   powershell -ExecutionPolicy Bypass -File scripts\test-local-codex-release.ps1 -Package <crate>
   ```
   (`-Package` is mandatory; run once per changed owner crate.)
3. Finalize the merge commit and push:
   ```
   git commit --no-edit
   git push
   ```
   (Run `mcp__wizard__analyze_git_diff` before the commit if any content is still unstaged.)

---

## Quick checklist
- [ ] Step 1: fetched, hotspot map + adapter gaps regenerated, markers counted, rerere on,
      baseline noted, count + gaps presented.
- [ ] Step 2: extracted only if gated; restore-point commit; markers re-counted.
- [ ] Step 3: `--no-commit` merge; disjoint slices; resolvers fanned out; ALL `success` +
      `git diff --check` clean; regen files flagged.
- [ ] Step 4: stray edits staged; `FILES_UNCERTAIN` surfaced.
- [ ] Step 5: first `cargo check`; log greppped (not trusting exit 0); schemas/locks regen;
      clean.
- [ ] Step 6: build + deploy; per-crate smoke tests; merge commit; push.
