$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: self-review-feature-main-worker'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: Repository `C:\Users\Oleh\Documents\GitHub\open_ai\codex`. Main task memo is `self-review feature.md`. Implement the main self-review feature improvement in this checkout.

DO_NOT_INSPECT: Do not read broad session logs, old prompt-reducer artifacts, unrelated repos, or unrelated workflow metadata unless directly needed. Do not revert unrelated dirty work. Do not make the final manual git commit or deploy exe; root owns final verification/build/deploy. You are not alone in the codebase: avoid conflicts and accommodate existing dirty files.

SCOUT_EVIDENCE: Prior routing identified likely relevant files: `codex-rs/core/src/session/review.rs`, `codex-rs/core/src/tasks/review.rs`, `codex-rs/core/src/session/turn.rs`, `codex-rs/core/src/state/turn.rs`, `codex-rs/core/src/turn_diff_tracker.rs`, `codex-rs/turn-diff/src/lib.rs`, `codex-rs/self-review/src/lib.rs`, `codex-rs/self-review/src/git_evidence.rs`, `codex-rs/core/src/session/checkpoint_git.rs`, TUI flow files under `codex-rs/tui/src/chatwidget/`, and tests such as `codex-rs/tui/src/chatwidget/tests/review_mode.rs`.

WHY_AGENT / ROI: User explicitly requested delegation, non-interactive worker sessions, top model/effort, and 5-minute check intervals. Main implementation belongs to this worker; root will check logs/handoff every 5 minutes and handle final integration/verification/build/deploy.

FIRST_READS: Read `AGENTS.md` first and follow it. Then read `self-review feature.md`. Then read the relevant Rust files listed above directly. Use `rg` only for exact symbols after those reads.

IMPLEMENTATION REQUIREMENTS:
1. Current-agent self-review scope is tracked by code, not LLM judgment. Each file path changed by the current agent/session is remembered since the last review; after the review it is forgotten. Same for git commits by this agent. Review only agent-specific tracked changes/commits.
2. Include files modified by the current session even if they were already dirty before the modifying shell command and no exact file event exists. The goal is to include current-session modifications while avoiding older/unattributed work.
3. Review is suggestive: it should behave as if a user prompt is inserted/sent in session, instead of a disruptive broad review mode.
4. Before the review prompt, insert/send this prompt: `please 1. sum-up your recent changes, 2. write your next plans 3. next actions to do`. The system remembers the LLM answer, and after review plus actions on the review are done reinserts/sends: `Please resume your before-review tasks. Here is the reminder about them: <...>`.
5. Review should be acted upon. If Codex does not fix review findings automatically, insert/send another prompt to fix review findings after the review findings.
6. Self-review auto-commit behavior should prefer committing all changed files before review, then all changed files after review. The product code should include untracked code files for Python, Rust, C/C++, `.bat`, `.ps`, JavaScript, PHP, Java, Kotlin, Scala, Swift, Objective-C, C#, and Prolog. Include common extensions such as `py`, `rs`, `c`, `cc`, `cpp`, `cxx`, `h`, `hh`, `hpp`, `hxx`, `bat`, `ps1`, `psm1`, `psd1`, `js`, `jsx`, `ts`, `tsx`, `mjs`, `cjs`, `php`, `java`, `kt`, `kts`, `scala`, `sc`, `swift`, `m`, `mm`, `cs`, `pl`, and `pro`.

TOOL_HINTS: Use `apply_patch` for source edits. Keep changes localized. Prefer focused tests. If repeated inspection is needed, use a small script only when it saves time and does not write source files. Do not create unrelated documentation churn.

VERIFICATION: Run `cargo fmt` or crate formatting for touched Rust files if feasible. Run focused tests for changed crates or modules; if blocked/slow, return exact command and observed failure. Do not run final release build/deploy; root will.

HANDOFF: Write a concise handoff to `.codex/workflow/agents/handoffs/self-review-feature-main-worker.md` with sections `Files changed`, `Behavior implemented`, `Tests`, and `Residual risks`. Also include the same summary in your final response. Do not include long diffs.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\self-review-feature-main-worker.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
