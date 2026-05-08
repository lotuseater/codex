# Local Change Verification Policy

This Windows checkout is release-only for practical Codex validation. Debug
Cargo lanes and broad release test filters have repeatedly consumed too much
disk and time, so use the narrowest proof that matches the change.

## Default Order

1. Inspect live build state first:
   `powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode Status`.
2. Do not start another Codex Cargo build while repo-local `cargo`, `rustc`, or
   `link` processes are active.
3. For pure policy/helper crates, run only the focused release crate test first.
   Example: `cargo test -p codex-agent-policy --release -j 1`.
4. For DAB/tool experiments, prove the behavior in
   `C:\Users\Oleh\Documents\GitHub\context-reducer-lab` before copying the
   result into Codex. Example:
   `cargo run --release --bin dab_canary -- --tool dab_find_window --limit 5 --compact`.
5. Run `just fmt` after Rust edits. Run scoped `just fix -p <crate>` only after
   the relevant checks are otherwise green.
6. Treat `scripts\build-local-codex.ps1 -Mode FastRelease` as the single
   expensive compile/deploy gate. Use `LowMemRelease -Jobs 1` when C: free space
   or memory pressure is low.
7. After deploy, smoke the actual wrapper with `codex --version`,
   `codex features list`, native DAB, and the targeted MultiAgentV2 canary.

## When To Avoid Heavy Tests

- Avoid routine `cargo test -p codex-tui --release` lanes unless a TUI unit or
  snapshot test is the only practical proof.
- Avoid broad `cargo test -p codex-core --release <filter>` lanes. If a core
  unit test is needed, prefer `cargo test -p codex-core --release --lib <filter>
  -j 1`.
- Prefer live installed-app smoke tests for interaction behavior after the
  deploy build has already paid the compile cost.

## Native DAB

- Use `context-reducer-lab` as the fast native DAB canary home.
- The lab canary should pass `dab_find_window` before a Codex build is spent on
  DAB changes.
- Use `CODEX_DAB_LIVE_TEST=1 cargo test -p codex-desktop-automation --release
  execute_dab_find_window_live_canary_when_enabled -j 1` for the Codex crate
  live canary when the release target is available.
- If native DAB fails, keep the failure concrete: tool name, status code,
  stdout, stderr, and whether PowerShell launched.

## Prototype First

Use the `prototype-first-automation` skill when a small script, fixture runner,
or lab canary would shorten a risky or repeated improvement loop. The default
Codex tool lab is `C:\Users\Oleh\Documents\GitHub\context-reducer-lab`.

Typical trigger cases:

- native DAB, hooks, skills, MCP, first-moves, repo-context-scout, cache,
  shadow/reducer operations, or MultiAgentV2 loop/supervision changes;
- GUI/live verification with screenshots, OCR, visible terminals, or real-data
  smoke paths;
- expensive Codex release builds, CMake/Ninja app builds, wrapper deployment,
  or repeated build-status checks;
- systematic migrations, parser/reducer work, output truncation, or repeated
  current-vs-alternative comparisons.

The repo-owned advisory hook lives at `.codex/hooks/prototype_first_hint.py`.
Install or refresh the system-wide copy with:

`powershell -ExecutionPolicy Bypass -File scripts\install-prototype-first-hook.ps1`

The hook is non-blocking. It emits sparse model-visible context only when its
ROI score crosses threshold, and it exits silently for one-off exact reads,
single `git status` / `git diff` / `git log`, tiny docs edits, or commit/push
finalization. Disable it through `/hooks` or set the corresponding
`hooks.state.<key>.enabled = false` in `C:\Users\Oleh\.codex\config.toml`.
