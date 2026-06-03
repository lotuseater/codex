# Cargo duplicate-dependency audit — codex-rs (2026-05-29)

**Goal:** decide whether deduplicating crates compiled at 2+ versions can meaningfully
cut build time and `target/` size on this disk-/RAM-constrained Windows machine.

**Verdict: No meaningful win is available.** The workspace is already about as
deduplicated as it can be without forking more third-party crates. Every duplicate
we *directly control* is already pinned to the newest version; the rest are
**transitive** (pinned by external crates) or **already addressed via vendor forks**.
Chasing the remainder would force full release rebuilds (30–90 min, ~20 GB) to verify
edits that mostly *won't* change the locked graph — poor ROI given the build fragility.

Recommendation: **do not pursue crate-dedup as a build-optimization lever.** The real
levers for build time/size live in the compile profile and feature set (see last
section), and those are already largely tuned.

---

## What we control — and it's already done

| Lever (workspace `Cargo.toml`) | Line | State |
|---|---|---|
| `windows-sys = { version = "0.61", … }` | 580 | newest; 0.52/0.60 copies are transitive |
| `base64 = "0.22"` | 398 | newest; 0.21.7 is pinned by `age`/`age-core` |
| `itertools = "0.14.0"` | 443 | newest; 0.11/0.13 pinned by lalrpop/bindgen/ratatui |
| `crossterm` → `nornagon` fork | 674 | forked to bump deps |
| `ratatui` → `lotuseater` fork | 675 | forked |
| `allocative` → starlark-rust `deps/bump-2026-05-on-0.13.0` | 688 | the **syn 1→2** bump lever, already in flight |

There is no `[patch]` lever left unused for the high-impact crates; the maintainer has
already pinned-newest and forked the crates that matter.

---

## Duplicates and why they persist

| Crate | Versions | Root cause | Dedupable |
|---|---|---|---|
| **syn** | 1.0.109 + 2.0.117 | v1 dragged by `cached_proc_macro` (+ build-dep `lalrpop`/`bindgen`); v2 is everything else. `allocative` fork already moved its half to syn 2. | **Maybe (in flight)** — only fully removable once the last syn-1 proc-macro (`cached`) is bumped/dropped. Requires full rebuild to verify. |
| **windows-sys** | 0.52 / 0.60 / 0.61 | We pin 0.61; 0.60 from `arboard`/`keyring`/`notify`, 0.52 transitive. | **No (transitive)** — only changes if those external crates bump *their* pins. |
| **windows-targets** | 0.52.6 / 0.53.5 | Mirrors windows-sys. | No (same root) |
| **getrandom** | 0.2 / 0.3 / 0.4 | 0.2 pinned by `ring`; 0.3 by `ahash`. | No (crypto-pinned) |
| **base64** | 0.21.7 / 0.22.1 | 0.21.7 pinned by `age`/`age-core` (encryption). | No (security-pinned) |
| **bitflags** | 1.3.2 / 2.11.1 | v1 via `portable-pty`→`lsp-types`. | No (transitive) |
| **darling** | 0.20.11 / 0.23.0 | 0.20 via `cached_proc_macro`. | Maybe (couples to syn-1 removal) |
| **itertools** | 0.11 / 0.13 / 0.14 | older via lalrpop/bindgen/ratatui. | No (transitive) |
| crypto-common, block-buffer, cpufeatures, const-oid, untrusted | 2 each | ring / aws-lc / SHA-family version drift. | No (crypto-pinned) |
| foldhash, fixedbitset, flume, nom, memoffset, matchit, regex-syntax, rustc-hash, similar, supports-color, self_cell, cfg_aliases | 2 each | small/leaf crates pinned by lagging parents (sqlx, petgraph, lalrpop, x509). | No / not worth it (each ≤ ~10 MB) |

~28 crates appear at multiple versions; **0 are removable by a low-risk workspace edit
alone.** The only non-transitive one (syn) is already half-migrated and blocked on a
single transitive proc-macro (`cached`).

---

## Speculative edits (low expected payoff, must rebuild to verify)

If pursued anyway, in rough priority — each needs a full rebuild + `cargo tree -d` to
confirm it actually collapses a version, and each may break API:

1. Bump `arboard "3"` / `keyring "3.6"` / `notify "8"` to latest — *might* pull
   windows-sys 0.61, collapsing 0.60. Likely won't (they pin internally). Est. ≤50 MB.
2. Bump `portable-pty "0.9"` → 0.10+ if it moves `lsp-types`/bitflags to v2. Est. ~10 MB.
3. Find who pulls `cached` and bump/drop it to finish the syn 1→2 unification (the only
   genuinely worthwhile one, ~200–300 MB intermediate + 2–3 min — but coupled to darling
   0.20→0.23 and needs every derive macro re-tested).

None are "quick wins"; all gate on external-crate cooperation and a verification rebuild.

---

## Where build time/size actually comes from (the real levers)

Since the goal is faster/smaller builds, these move the needle far more than dedup —
and most are already applied in `scripts/build-local-codex.ps1 -Mode LowMemRelease`:

- **opt-level=1, LTO=off** (already) — biggest build-time win; LTO is the #1 release cost.
- **debug=0 / `/DEBUG:NONE` MSVC link arg** (already) — biggest `target/` size win.
- **sccache** (already) — caches across rebuilds.
- **`-p codex-cli` scoped build** (already) — don't build the whole workspace.
- Remaining options: prune default features on heavy deps; raise `codegen-units` for
  dev iteration; periodic `target/` GC to keep disk under control. `panic="abort"` would
  shrink/speed further but changes unwinding semantics — **not** recommended for the TUI.

**Bottom line:** dependency-dedup is a dead end here; the profile is already tuned. The
highest-value next action is unrelated to deps — **build + deploy the already-repaired
binary** (the merge-repair fix is verified by `cargo check` but not yet in a deployed
build; the live wrapper still points at the pre-repair `codex-custom-20260525-084110`).
