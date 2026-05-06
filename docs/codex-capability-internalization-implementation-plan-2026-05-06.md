# Codex Capability Internalization Implementation Plan

Date: 2026-05-06

## Implemented In This Change

### Auto-loop after self-review

- Track automatic self-review separately from manual review.
- Mark the continuation ready only after `ExitedReviewMode`.
- Request app-level loop continuation once the review turn is complete.
- Submit the configured loop message immediately if loop mode is enabled and
  the composer, approvals, task state, and queues are clear.
- Do not queue a hidden message when the UI is blocked.

### Cache scope metadata

- Add repo-aware metadata to Codex operation-cache bridge events.
- Export the same metadata to bridge subprocess env:
  `CODEX_PROJECT_ROOT`, `CODEX_PROJECT_NAME`, and
  `CODEX_PROJECT_CACHE_NAMESPACE`.
- Use a namespace shaped as `<safe-repo-folder-name>-<stable-root-hash>` so
  system-wide caches can include the repo folder name without colliding on
  common leaf paths such as `src`.
- Compute repo cache scope once per bridge action and pass it to both the JSON
  event and subprocess environment, avoiding duplicate `.git` ancestor walks.

### Native desktop automation

- Add a `codex-desktop-automation` crate with a Windows native DAB bridge and
  repo/app harness detector.
- Expose built-in tools:
  `automation_harness_detect`, `dab_find_window`, `dab_window_check`,
  `dab_screenshot`, `dab_ocr`, `dab_visual_scan`, `dab_element_map`,
  `dab_navigate`, `dab_smart_click`, `dab_click`, `dab_bg_click`, and
  `dab_send_keys`.
- Gate tools with `[desktop_automation] enabled = true` by default and the
  existing `[features] computer_use` hard-disable.
- Keep click/key tools mutating and non-cacheable; keep detection, window,
  screenshot, OCR, visual scan, and element-map tools read-only.
- Mark the DAB tool family as non-parallel so live GUI operations are sequenced.
- Bundle a system `visual-app-inspector` skill that tells Codex to detect
  app-native harnesses first, inspect before input, and capture evidence after
  GUI actions.

### Native first-moves

- Add a `codex-first-moves` crate with repo scanning, intent routing, SQLite
  prediction/hit telemetry, and bounded file excerpt prewarming.
- Inject first-moves context only for fresh turns that have no previous user
  message in the thread.
- Drop legacy Wizard first-moves hook context when native context is present so
  the model does not receive duplicate opening advice.
- Expose native `first_moves_predict` and `first_moves_stats` tools.
- Record hits when later successful tool calls mention predicted paths.
- Resolve the enclosing git root before computing system-wide first-moves cache
  namespaces, so launches from `src` still use the repo folder name.

### Automatic self-review

- Generate automatic code self-review as a custom review prompt grounded in
  `git status --short`, targeted `git diff`, and `git show` when the tree is
  clean.
- Carry compact work notes with counts, recent commands, changed paths, and
  plan updates so review remains useful after compaction without expanding the
  conversation history.
- Keep the existing cooldown to prevent repeated automatic reviews.
- Add a separate Plan-mode self-review trigger after the full plan is shown and
  no modal/question/queued steer is active.
- Suppress ordinary code self-review during and immediately after planning.
- Consume the revised-plan turn once; the second plan is not reviewed again.

### Remembered session choices

- Persist approval policy changes to root config or the active profile.
- Persist legacy-compatible permission profile changes as `sandbox_mode`; root
  full-access saves clear `default_permissions` so the legacy full-access choice
  takes effect on the next launch.
- Forward current in-memory reasoning effort and summary into app-server
  thread start/resume/fork overrides so restored sessions keep the selected
  effort immediately, not only after config reload.

## Next Native Surfaces

### Optimization rule

Internalization should not copy sidecar behavior one-for-one. Each capability
should get a native decision point that can skip redundant work, expose why it
ran, and use telemetry to improve future choices. Examples:

- avoid rescanning skills/hooks when watcher state says nothing changed;
- use first-moves only during cold repo/context openings, not every turn;
- use cache hit/miss telemetry to decide which operation classes are worth
  storing;
- prefer app-native GUI harnesses before DAB so DAB is not invoked when a
  cheaper direct contract exists;
- report disabled/missing providers explicitly instead of rediscovering them
  through repeated failed tool calls.

### Capability status

Add one status surface that reports:

- system skills root and repo skills root;
- hook config files in effect;
- native app-tools cache path/state;
- operation-cache bridge enabled/disabled state;
- first-moves DB presence and hit stats when available;
- DAB provider availability when exposed by MCP.

The lowest-risk API shape is a read-only app-server method first, then TUI
status rendering once the payload is stable.

### Cache creation policy

Codex should recognize existing Wizard-created caches, but new Codex-created
system-wide caches should live under `~/.codex/cache`. Repo-wide caches should
live under the repo root and include repo identity in any system-wide index.

Required rule: never derive a system-wide path from only the current folder
leaf. Use a path-safe repo folder component plus a stable root hash, and keep
the raw repo root/name in metadata for display and diagnostics.

### DAB provider contract

Do not hardcode Wizard paths into ordinary GUI behavior. Codex now exposes
native DAB directly and should discover app-level providers before generic
desktop control, then report:

- provider name;
- available actions;
- whether native DAB is enabled or disabled by config/feature gates;
- whether screenshots/OCR/click/send-keys are available.

Runtime GUI work should prefer app-native harnesses first, then native DAB,
then Wizard compatibility, then shell-only fallbacks.

### First-moves telemetry follow-up

The native first-moves surface now records predictions and tool-use hits. The
next useful increment is a compact status/debug display that reports repo DB
presence, system DB path, hit counts, and whether a fresh turn received injected
context.

## Verification Plan

- Focused TUI tests for automatic self-review loop continuation and manual
  review suppression.
- Focused app test that the app layer submits the configured loop message.
- Core operation-cache unit tests for repo-root cache identity.
- First-moves release tests for prediction, repo-root namespace resolution, and
  hit tracking.
- Desktop automation unit tests for harness detection and tool exposure gates.
- TUI tests for plan self-review, regular self-review suppression during plan
  review, remembered permission choices, and restored reasoning effort.
- Real Windows smoke tests for GitHub GUI harness detection, terminal,
  PowerShell, Calculator, Notepad++, Google Chrome, and Paint.
- `just fmt` after Rust edits.
- Release-safe focused Rust test lanes only; avoid debug-profile workspace
  tests on this Windows checkout.
