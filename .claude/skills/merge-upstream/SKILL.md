---
name: merge-upstream
description: >-
  Run the fork's full upstream-merge runbook: sync openai/codex `upstream/main` into the
  current fork branch (`<FORK_BRANCH>`), reduce conflicts via pre-merge extractions, fan
  out the `merge-conflict-resolver` subagent on disjoint conflicted-file slices under a
  strict NO-BUILD rule, then verify, build, and deploy. Driven by the `scripts/merge-*` /
  `scripts/*-gate` automation (preflight, partition, residue/build gates, regen). Trigger
  when the user asks to
  "merge upstream", "sync upstream/main", "pull upstream into the fork", "do the upstream
  merge", or "update the fork from openai/codex". Side-effecting (mutating git + builds),
  so user-invoked only.
disable-model-invocation: true
---

# merge-upstream — upstream/main → fork merge runbook

You (the main agent) are the **orchestrator**. This skill codifies the practiced 6-step
merge process for the Codex Rust fork on Windows (pwsh). Repo root (use ABSOLUTE paths):
`C:\Users\Oleh\Documents\GitHub\open_ai\codex`. Branch: `<FORK_BRANCH>` — the current fork
branch you are merging into (run `git branch --show-current` to confirm; **currently
`claude-automation-toolkit`**, was `slow-context-budget-mode` on older runs). Substitute the
live branch name wherever `<FORK_BRANCH>` appears below.
Remotes: `origin` = push target (lotuseater/codex), `upstream` = openai/codex.

This runbook is driven by a suite of read-only/gate scripts under `scripts/` (each is
self-contained pwsh; pass `-Help`/read its `.SYNOPSIS` for flags):
`merge-preflight.ps1` · `detect-adapter-gaps.ps1` · `partition-conflict-slices.ps1` ·
`split-completeness-check.ps1` · `check-no-merge-residue.ps1` · `verify-cargo-log.ps1` ·
`regen-all.ps1`. Each step below calls the relevant one — don't reproduce their logic by hand.

Source-of-truth docs to consult (do not duplicate them — point resolvers at them):
- Process: `.codex/workflow/HANDOFF_merge.md`, `Main_Merge_Prompt.md`
- Resolver policy: `.codex/tmp/merge_resolution_brief.md`
- Fork features + owner crates + per-feature health checks: `docs/fork-feature-inventory.md`
- Resolver subagent: `.claude/agents/merge-conflict-resolver.md` (model: opus)

> ## ⛔ INVARIANTS (never violate)
> 1. **NO build until Step 5.** No `cargo build/check/test/clippy/nextest`, no
>    `scripts\build-*`, no `just test` until every resolver reports success and
>    `git diff --check` shows ZERO markers. Git restore points are the safety net.
> 2. **Resolvers own DISJOINT slices only.** Resolution runs as a **Claude Workflow of
>    opus `merge-conflict-resolver` agents — one per disjoint area slice** (slices from
>    `partition-conflict-slices.ps1`); slices never share a file. Interleaving edits are the
>    main conflict source.
> 3. **Preserve EVERY fork feature** whose surface a resolution touches, per
>    `docs/fork-feature-inventory.md`. **Union-preserve by default**; take-upstream only for
>    pure upstream refactors the fork doesn't own (then re-wire call-sites).
> 4. **Enable `git rerere` BEFORE rehearsing** (`git config rerere.enabled true`) so resolved
>    hunks are remembered across re-rehearsals/retries.
> 5. **Bank a handoff before any compaction.** Refresh `.codex/tmp/merge_preflight_<date>.md`
>    or `COMPACT_HANDOFF.md` with current state + ordered next steps + files to stage.
> 6. **Subagents may run in divergent worktrees** → always hand them ABSOLUTE main-repo
>    paths, or their edits never reach the live tree.
> 7. Commits are BLOCKED unless `mcp__wizard__analyze_git_diff` ran first — call it before
>    every commit.

Let `<date>` = `yyyy-MM-dd` throughout (e.g. logs and preflight notes are date-stamped).

---

## Step 1 — Preflight (READ-ONLY: no merge, no build)

1. **Enable `git rerere` before anything else** so the rehearsal (and later retries) remember
   resolved hunks:
   ```
   git config rerere.enabled true
   ```
2. **Run the one-shot preflight** — it fetches `upstream/main`, computes ahead/behind,
   rehearses the merge via `git merge-tree` (counting conflict markers WITHOUT touching the
   tree), regenerates the area table + hotspots, and runs the adapter-gap pass, writing a dated
   report under `.codex/tmp/`:
   ```
   powershell -ExecutionPolicy Bypass -File scripts\merge-preflight.ps1
   ```
   - Pass `-NoFetch` to reuse an already-fetched `upstream/main` (e.g. on a re-run).
   - This supersedes the old manual `git fetch` + `generate-merge-hotspot-map.ps1` +
     `git merge-tree | Select-String` sequence — `merge-preflight.ps1` does all of it in one
     read-only pass and records the baseline (HEAD sha, upstream tip, merge-base, ahead/behind,
     conflict-marker count, adapter-gap list, top hotspots) into its `.codex/tmp/` report.
     (`analyze-branch-conflict-surface.ps1` still gives an area-weighted second view if wanted.)
3. **Run the adapter-gap pass explicitly** (block-level — it now also flags fork logic still
   inlined inside an upstream-shaped function even when a sibling seam file exists), capturing
   the list for the user:
   ```
   powershell -ExecutionPolicy Bypass -File scripts\detect-adapter-gaps.ps1 -OutPath .codex\tmp\adapter-gaps_<date>.md
   ```
   - Flags upstream-hot, fork-heavy `.rs` files / blocks with no `*_local.rs`/`*_adapter.rs`/
     `*_fork.rs` seam (default `-MinForkCommits 3`). **Exit code 1 = gaps found** (it's a CI
     gate) — that's the signal, not an error.
4. **Present to the user**: the conflict-marker count and the adapter-gap list from the
   preflight report, then proceed.

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
1. **Verify the extraction dropped no code** — each extraction is code-motion, and split
   workers can silently drop a method body (keeping its doc/attr). For every file you split,
   compare the `fn` set before vs after:
   ```
   powershell -ExecutionPolicy Bypass -File scripts\split-completeness-check.ps1 -Ref HEAD -Original <original.rs> -After <seam1.rs>,<seam2.rs>
   ```
   (`-Ref` = the pre-extraction commit; non-zero exit = a fn went missing — fix before committing.)
2. **Re-run the adapter-gap pass — expect ZERO inline-fork gaps now** (the extractions should
   have moved every fork-only block behind a seam):
   ```
   powershell -ExecutionPolicy Bypass -File scripts\detect-adapter-gaps.ps1
   ```
   If gaps remain, the extraction was incomplete — finish it before merging.
3. Re-run `scripts\merge-preflight.ps1` (or just its `git merge-tree` rehearsal) and re-count
   markers — expect a reduction vs the Step-1 baseline.
4. `mcp__wizard__analyze_git_diff`, then commit a restore point:
   ```
   git commit -m "refactor(pre-merge): conflict-reduction extractions"
   ```

If the gate isn't tripped (or the user declines), skip straight to Step 3.

---

## Step 3 — Merge under the NO-BUILD rule

1. Start the merge, leave it uncommitted (rerere is already on from Step 1):
   ```
   git merge upstream/main --no-commit
   ```
2. **Partition the unmerged files into DISJOINT area slices** — `partition-conflict-slices.ps1`
   reads `git diff --name-only --diff-filter=U` and maps each unmerged file to exactly one
   disjoint area slice (one resolver worker per slice; never splits a file across two slices,
   keeps an interleaved region whole inside ONE slice):
   ```
   powershell -ExecutionPolicy Bypass -File scripts\partition-conflict-slices.ps1
   ```
   - Standard areas it buckets into:
     `core-session` · `core-tools` · `tui-app` · `tui-composer` · `protocol` ·
     `app-server-protocol` · `config-features` · `manifests` (Cargo.toml/lock, MODULE.bazel).
   - It splits DEEP — prefer several lean resolvers over a few heavy ones. Each emitted slice
     is the exact ABSOLUTE-path list for one resolver prompt.
3. **Fan out the `merge-conflict-resolver` agents on those slices as a Claude Workflow.**
   - Run a **Workflow of opus `merge-conflict-resolver` agents — one per disjoint slice**
     (`parallel` / `pipeline`, each phase-labeled) so they run concurrently and survive
     compaction. Resolution is **union-preserve by default** (per INVARIANT 3).
   - **Fall back** to concurrent `Agent` calls (multiple opus `merge-conflict-resolver`
     invocations in one message) if a workflow isn't set up.
   - Each resolver prompt MUST: list its exact conflicted paths (ABSOLUTE, from the partition
     output), point at
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
4. **Do NOT run any build** until EVERY resolver returns `HANDOFF_STATUS: success` AND the
   residue check is clean (zero conflict markers, no `*.orig`):
   ```
   powershell -ExecutionPolicy Bypass -File scripts\check-no-merge-residue.ps1
   ```
   (Non-zero exit = residue found; it prints each offending `file:line`. This is the same
   COMMIT GATE used in Step 4 — run it here unstaged to confirm resolvers finished.) If any
   resolver is `partial`/`blocked`, re-slice the leftover files and respawn a fresh resolver
   (never reuse one across batches).
5. **Regenerate union/generated files** flagged under `FILES_NEEDING_REGEN` (e.g.
   `app-server-protocol/src/export.rs` + schema JSON) — use `scripts\regen-all.ps1` (Step 5);
   do the regen, don't hand-finish generated files.

---

## Step 4 — Fix-up (still pre-build)

1. Catch stray unstaged edits the resolvers left:
   ```
   git diff --name-only
   ```
2. Stage everything for the merge:
   ```
   git add -A
   ```
3. **COMMIT GATE — run the residue check on the STAGED tree before committing.** Never commit
   the merge with conflict markers or `*.orig` files staged:
   ```
   powershell -ExecutionPolicy Bypass -File scripts\check-no-merge-residue.ps1 -Staged
   ```
   Non-zero exit = abort the commit and clean the offending files it prints. Only when it
   exits 0:
   ```
   mcp__wizard__analyze_git_diff
   git commit -m "fix(merge): post-resolution corrections"
   ```
   (Or keep staged and fold into the final merge commit in Step 6 — either is fine, but run
   the `-Staged` gate before whichever commit lands the merge.)
4. **Surface every `FILES_UNCERTAIN`** entry from the resolver reports to the user for human
   review before building.

---

## Step 5 — Verify (FIRST build allowed)

1. Workspace check, release-only, captured to a log:
   ```
   powershell -ExecutionPolicy Bypass -Command "cargo check --workspace --release --keep-going *>&1 | Tee-Object logs\merge-cargo-check-<date>.log"
   ```
2. **BUILD GATE — never trust a backgrounded cargo exit code.** A background `exit 0` is
   FALSE-GREEN (a trailing echo masks cargo's real exit). Confirm success only by scanning the
   LOG for `error[`/`error:` / nonzero `EXITCODE=`:
   ```
   powershell -ExecutionPolicy Bypass -File scripts\verify-cargo-log.ps1 -LogPath logs\merge-cargo-check-<date>.log
   ```
   Non-zero exit = the log contains errors / a nonzero `EXITCODE=` — treat as failure (do NOT
   proceed on the cargo process's own exit code).
3. If broken, **fan out compile-fixers per broken crate** (one fresh worker per crate,
   file-disjoint), then re-check. (`cargo check --release` skips `#[cfg(test)]`; if a test
   split was touched, verify with `cargo check --tests -p <crate>` AND
   `scripts\split-completeness-check.ps1` to confirm no test fn was dropped.)
4. **Regenerate all schemas/lockfiles whose inputs changed in one pass** — `regen-all.ps1`
   wraps the root `justfile` recipes (config schema, app-server schema, bazel lock):
   ```
   powershell -ExecutionPolicy Bypass -File scripts\regen-all.ps1 -Check   # report-only: what's stale
   powershell -ExecutionPolicy Bypass -File scripts\regen-all.ps1          # actually regenerate
   ```
   (`-Check` exits non-zero if any generated artifact is out of date — useful as a CI gate;
   omit it to write the regenerated files.) Re-run Step 5.1–5.2 until the log is clean.

---

## Step 5.5 — Post-merge build-fix loop (the part that eats the session)

Step 5.3 fans out compile-fixers; this section is the FULL runbook for that loop — the
post-merge compile-error triage that the conflict-marker tooling does NOT cover. Budget
**~6-9 iterations**, not one: `--keep-going` SKIPS any crate whose dependency failed, so
errors surface in **WAVES** — leaf/base crates clear first, the big aggregator
(`codex-core`) is last and largest. Each cleared layer unblocks the next.

### The iteration loop
```
loop (run cargo from INSIDE codex-rs/ — see toolchain-CWD gotcha):
  cargo check --workspace --release --keep-going  > logs\merge-check-release-iterN.log 2>&1
  scripts\verify-cargo-log.ps1 -LogPath logs\merge-check-release-iterN.log   # pass/fail GATE (grep, not exit 0)
  if clean: break
  scripts\merge-buildfix-triage.ps1 <log>   # triage/partition: groups errors by (error-code, missing-symbol) + owning crate/file, suggests a file→slice partition
  recon (1 worker): decide RESTORE/RENAME/REMOVE per root cause; emit file-disjoint slices
  fan out N concurrent fix-workers (one per slice); ORCHESTRATOR re-checks (iter N+1)
```
`verify-cargo-log.ps1` = the pass/fail gate (already in `scripts/`); `merge-buildfix-triage.ps1`
= the triage/partition helper (so recon starts from a table, not 100 hand-grepped errors).

### The dominant failure pattern (~80% of post-merge errors)
> Upstream ADDED a member (field / enum variant / trait method / fn / import path) and its
> USAGE. The 3-way merge resolved the DEFINITION toward the fork side (which lacked the new
> member, or had the old name), but upstream's USAGE files survived → usage refers to a
> member the fork-flavored definition doesn't have.

Variants — all the same root shape: dropped struct field, dropped enum variant arm
(non-exhaustive match), dropped trait method, dropped free fn / method, stale import path
(symbol moved crate/module), field/symbol RENAME (fork usage kept the OLD name), orphaned
DEAD code left after upstream removed a whole feature (dangling field-inits + an orphaned fn).

### The per-member decision — RESTORE-MEMBER vs ADOPT-RENAME vs REMOVE
Decide ONCE per root cause, GLOBALLY, BEFORE editing call sites (so two slices can't guess
differently on a shared type like `Session`/`Config` and diverge):
- **RESTORE-MEMBER (default, union-preserve)** — the member is a fork feature (or an upstream
  addition the fork wants) the merge dropped. Add it back to its OWNING definition → ALL
  external call sites auto-resolve with ZERO call-site edits. Cheapest fix; default.
- **ADOPT-RENAME** — upstream RENAMED a member and the fork should follow. The def already
  has the new name; fork USAGE kept the old one → rename the CALL SITES (not the def). Pick
  this when the new name already compiles at many OTHER sites (renaming the def back is the
  bigger blast radius).
- **REMOVE** — upstream DELETED the concept entirely → drop the fork's dangling usages.

Git-evidence method (decide which case): compare the owning type on each merge parent —
fork-parent = `<merge>^1`, upstream-parent = `<merge>^2` (`git rev-parse <mergesha>^1 ^2`):
```
git show <merge>^1:<path>   # fork side
git show <merge>^2:<path>   # upstream side
```
Heuristic: member on fork side & gone upstream → RESTORE (fork still uses it) or REMOVE
(upstream killed it AND no longer uses it) — tell apart by whether upstream still USES it.
New name present & compiling elsewhere → ADOPT-RENAME.

### Orchestration — recon → file-disjoint fan-out
1. **Recon (1 worker)** reads the full error log + chokepoint defs + the `^1`/`^2` git diffs,
   and emits a PLAN: per root cause the RESTORE/RENAME/REMOVE decision + the OWNING file, PLUS
   a **file-disjoint partition** where **each chokepoint def is owned by EXACTLY ONE slice**
   and the auto-resolving call-site files are listed (no other slice touches them).
2. **Fan-out (N concurrent fix-workers)**, one per slice, each editing only its owned files,
   each told: "call-site files of a member another slice restores → DO NOT edit; they compile
   after that owner's restore." Cross-slice deps are **compile-time only**, so concurrent
   editing of disjoint files is safe.
3. **Consolidated check (ORCHESTRATOR only)** — workers CANNOT verify (a hook force-backgrounds
   their cargo, and subagents aren't re-woken on bg completion). The orchestrator always runs
   the authoritative `cargo check` + `verify-cargo-log.ps1`.

### Gotchas (checklist — each cost real time)
- [ ] **Toolchain-CWD trap.** Toolchain selection follows the CWD, NOT `--manifest-path`. From
      repo root → rustup default (1.93.0) → instant bail "sqlx requires rustc 1.94.0" (~14-line
      log that LOOKS like a code error). FIX: always run cargo from INSIDE `codex-rs/`.
- [ ] **False-green exit 0.** A trailing echo / `EXITCODE=` line after cargo masks its real
      failure; bg "exit code 0" is unreliable. GREP the log (`error[`, `error:`, `^error`,
      `EXITCODE=`) via `verify-cargo-log.ps1` — never trust the notification.
- [ ] **`--release` skips `#[cfg(test)]`.** A green release-lib check leaves test code with
      hundreds of stale-API errors. Test repair is a SEPARATE `cargo check --tests -p <crate>`
      pass / deferred debt — green release is NOT "done".
- [ ] **Code-motion drops method bodies.** A worker MOVING a method (trait-impl relocation) can
      silently drop the body (keeping doc/attr). Verify moves: source-removed + target-added +
      body verbatim (`scripts\split-completeness-check.ps1`).
- [ ] **Compat-wrapper before deleting** a fn if a non-owned caller survives: grep ALL callers
      across slices first; if any are out of your slice, keep a thin wrapper METHOD (NOT a
      cross-crate re-export shim — those stay banned).
- [ ] **`PostToolUse` formatter re-sorts `use` blocks** after edits — cosmetic, ignore.

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
3. Finalize the merge commit and push. **Run the residue COMMIT GATE one last time on the
   staged tree** before the final commit:
   ```
   powershell -ExecutionPolicy Bypass -File scripts\check-no-merge-residue.ps1 -Staged
   ```
   Only when it exits 0:
   ```
   git commit --no-edit
   git push
   ```
   (Run `mcp__wizard__analyze_git_diff` before the commit if any content is still unstaged.)

---

## Quick checklist
- [ ] Step 1: rerere enabled; `merge-preflight.ps1` run (fetch/ahead-behind/merge-tree
      rehearsal/area table/hotspots/baseline); `detect-adapter-gaps.ps1` (block-level) run;
      count + gaps presented.
- [ ] Step 2: extracted only if gated; `split-completeness-check.ps1` (no fn dropped) +
      `detect-adapter-gaps.ps1` shows ZERO inline-fork gaps; restore-point commit; markers
      re-counted.
- [ ] Step 3: `--no-commit` merge; `partition-conflict-slices.ps1` → disjoint slices; Workflow
      of opus `merge-conflict-resolver` agents (union-preserve) fanned out; ALL `success` +
      `check-no-merge-residue.ps1` clean; regen files flagged.
- [ ] Step 4: stray edits staged; `check-no-merge-residue.ps1 -Staged` COMMIT GATE passed;
      `FILES_UNCERTAIN` surfaced.
- [ ] Step 5: first `cargo check`; BUILD GATE via `verify-cargo-log.ps1` (not trusting exit 0);
      `regen-all.ps1` for schemas/locks; clean.
- [ ] Step 5.5: build-fix LOOP (~6-9 waves, leaf crates → `codex-core` last);
      `merge-buildfix-triage.ps1` → recon decides RESTORE/RENAME/REMOVE per root cause →
      file-disjoint fix-workers → ORCHESTRATOR re-checks; gotchas (toolchain-CWD, false-green,
      `--release` skips tests, code-motion drops bodies, compat-wrapper non-owned callers).
- [ ] Step 6: build + deploy; per-crate smoke tests; `check-no-merge-residue.ps1 -Staged`;
      merge commit; push.
