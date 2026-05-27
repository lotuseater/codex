$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: stage4_ext_protocol_infra'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.5', '-c', 'model_reasoning_effort=xhigh', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'exec', 'CONTEXT_AREA: live upstream/main merge conflict resolution after pre-merge conflict-reduction commit 1df5afe4be on branch slow-context-budget-mode.
DO_NOT_INSPECT: do not broadly scan unrelated repo areas; do not modify paths outside your OWNED_PATHS unless you first record a blocker in the handoff and stop.
SCOUT_EVIDENCE: root already fetched upstream/main, committed pre-merge refactors, ran the real merge, and partitioned git diff --name-only --diff-filter=U by path ownership.
WHY_AGENT / ROI: user explicitly requested external non-interactive delegated sessions. Your scope is independent and disjoint; resolving it in parallel reduces root context load. Do not spawn subagents.
TOOL_HINTS: use git show :1:path, :2:path, :3:path to inspect base/ours/theirs; use rg for conflict markers; use apply_patch for edits. You may run read-only git diff/status commands. Do not run cargo, rustc, just, bazel, npm, pnpm, yarn, schema generation, build scripts, tests, deploy, or activation commands.
TOKEN_TIP: focus only on conflict hunks and nearby APIs needed to make the merge coherent.
VERIFICATION: before handoff, run rg -n "^(<{7}|={7}|>{7})" on your OWNED_PATHS only, and git diff --check on your OWNED_PATHS only. Do not run builds/tests.
HANDOFF: write a concise markdown handoff to RUN_DIR/WORKER_NAME/.handoff/.md listing resolved paths staged, unresolved blockers, semantic choices, and any follow-up needed after all conflicts are resolved.

Important merge policy:
- You are not alone in the codebase. Other workers are editing and staging disjoint files. Do not revert or restage their work.
- Resolve only OWNED_PATHS. If a conflict requires a cross-file decision outside OWNED_PATHS, leave your owned file coherent if possible and record the dependency.
- Preserve both current-branch behavior and upstream/main behavior when compatible. Prefer semantic union over choosing one side blindly.
- Preserve the pre-merge refactor intent: frame requester no longer depends on ratatui/crossterm event types; config profile/config TOML naming was aligned with upstream template wording; obsolete direct ratatui/crossterm deps were removed from TUI where valid.
- Stage only your owned resolved paths with git add/rm. Do not commit.
WORKER_NAME: stage4_ext_protocol_infra
RUN_DIR: C:\Users\Oleh\AppData\Local\Temp\codex-merge-stage4-20260525-184928
FIRST_READS:
- C:\Users\Oleh\AppData\Local\Temp\codex-merge-stage4-20260525-184928\stage4_ext_protocol_infra\owned_paths.txt
- git status --short --untracked-files=no
- For each owned conflict: git show :1:<path>, git show :2:<path>, git show :3:<path> as needed.

TASK:
Resolve AGENTS, MODULE.bazel.lock, justfile, exec-server, extension-api/goal, features, network-proxy, protocol, thread-store, tools, and windows-sandbox conflicts.

OWNED_PATHS:
- AGENTS.md
- MODULE.bazel.lock
- codex-rs/exec-server/src/remote.rs
- codex-rs/ext/extension-api/src/contributors.rs
- codex-rs/ext/extension-api/src/contributors/turn_lifecycle.rs
- codex-rs/ext/extension-api/src/lib.rs
- codex-rs/ext/goal/src/accounting.rs
- codex-rs/ext/goal/src/extension.rs
- codex-rs/features/src/lib.rs
- codex-rs/features/src/tests.rs
- codex-rs/network-proxy/src/config.rs
- codex-rs/protocol/src/config_types.rs
- codex-rs/protocol/src/models.rs
- codex-rs/protocol/src/protocol.rs
- codex-rs/thread/thread-store/src/local/mod.rs
- codex-rs/thread/thread-store/src/local/search_threads.rs
- codex-rs/tools/Cargo.toml
- codex-rs/tools/src/json_schema.rs
- codex-rs/tools/src/tool_config_tests.rs
- codex-rs/tools/src/tool_output.rs
- codex-rs/windows-sandbox-rs/Cargo.toml
- justfile

Required final steps:
1. Resolve conflict markers only in OWNED_PATHS.
2. Stage only resolved OWNED_PATHS.
3. Write C:\Users\Oleh\AppData\Local\Temp\codex-merge-stage4-20260525-184928\stage4_ext_protocol_infra\.handoff\.md.
4. Stop. Do not commit, build, test, generate schemas, deploy, or edit other paths.
')
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\stage4_ext_protocol_infra.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
