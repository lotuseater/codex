# SOLID Refactor Overseer Memo

Date: 2026-05-21

This memo is for the root/overseer session. The overseer does not act as the
refactor director and does not edit product or refactor source. The overseer
keeps exactly one interactive director session alive, gives it follow-up prompts,
and lets the director coordinate visible worker waves.

## Roles

- Overseer: launch one director, monitor its behavior, and send concise
  follow-ups when it drifts, stalls, or needs a new wave.
- Director: read the SOLID handoff/docs and fresh worker handoffs, update
  workflow handoffs/prompts, classify work, and launch visible `codex-workers`
  waves.
- Workers: perform source analysis and source edits. Workers own narrow files or
  modules and write concise handoffs with changed files, fallout, and exact
  verification commands.

## Hard Stops

- Do not let the overseer or director edit `codex-rs/**`, Cargo manifests,
  lockfiles, Bazel files, generated schemas, or product source directly.
- Do not run broad builds, broad tests, schema generation, Bazel lock refresh,
  release builds, activation, deploy, or commits until the architecture
  refactor from the current SOLID docs is genuinely complete.
- Keep checks source/static only while the architecture refactor is open:
  `git diff --check`, targeted `rg`, PowerShell parser checks for changed
  `.ps1` files, and
  `scripts/check-cargo-dependency-boundaries.ps1 -SolidRefactor -Json`.
- Do not let the director do broad automatic review itself. If broad review is
  needed, it must delegate that review to a spawned worker/scout session.

## Director Follow-Up Prompts

Use these prompts in the visible director window as situations arise.

Startup or resume:

```text
Go on. Continue as SOLID refactor director, not implementer. Reread .codex\workflow\solid-refactor-handoff.md, docs\current-project-architecture-solid-refactor-plan.md, and docs\current-project-architecture-solid-review.md. Read fresh worker handoffs under .codex\workflow\agents\. Do not edit product/source code yourself. Update the handoff, classify fresh workers, and spawn actual visible codex-workers sessions for the next narrow refactor wave.
```

If the director was just launched and has not started visible work yet:

```text
Continue the active SOLID refactor now. Keep director context compact and avoid broad code exploration in this session. Reread the compact handoff and fresh worker handoffs, update the handoff if stale, then spawn scouts/workers for broad review, source analysis, and source edits. Before 50 percent context, update the handoff and compact.
```

When the director starts reviewing too broadly:

```text
Stop broad review in this director session. Delegate review or source analysis to a spawned worker/scout. Keep yourself to orchestration, handoff updates, worker prompts, source-only checks, and worker launch/monitoring.
```

When the director proposes broad builds or tests too early:

```text
Do not run broad builds, broad tests, schema generation, Bazel, release builds, activation, deploy, or commits yet. The architecture refactor is still open. Use only source/static checks: git diff --check, targeted rg, PowerShell parser checks for changed ps1 files, and scripts/check-cargo-dependency-boundaries.ps1 -SolidRefactor -Json.
```

After worker windows have finished:

```text
Read fresh worker handoffs, classify each as accepted, root-wiring-needed, repair-needed, or conflict/blocked. Update .codex\workflow\solid-refactor-handoff.md with the fresh classification and remaining gaps, then choose the next worker wave from the SOLID plan/review docs.
```

When no worker wave is active:

```text
Think of the next wave of narrow subtasks from the SOLID architecture docs. Prefer tests split by topic into separate Cargo test binaries with abstract and narrow dependencies, and core dependency inversion away from broad concrete crates such as codex-tools. Write worker prompts with clear ownership, then launch them with codex-workers in visible terminals.
```

When the director nears context pressure:

```text
Before compacting, update .codex\workflow\solid-refactor-handoff.md with current worker status, remaining gaps, source-only checks, and next wave candidates. Then compact around 50 percent context. After compacting, reread the handoff and fresh worker handoffs before continuing.
```

When the director finds a real source conflict:

```text
Do not repair source in the director session. Mark the handoff as repair-needed or conflict/blocked, write a focused repair-worker prompt with exact ownership, and launch that worker visibly.
```

When the director claims architecture is complete:

```text
First prove the architecture state with source-only evidence and the SOLID dependency-boundary JSON check. Do not begin broad verification until the handoff names the completed boundaries and the overseer/user explicitly authorizes the broad verification phase.
```

## Launcher

Use the visible interactive director launcher:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\start-solid-refactor-director.ps1
```

The launcher always invokes Codex with `--loop`. Resume must target the recorded
director session id when possible; do not rely on `resume --last` after the
overseer has been active, because that can resume the overseer instead of the
director. A resumed director is stale by default, so the launcher sends the
usual resume/post-compaction reminder automatically unless
`-NoResumeReminder` is explicitly passed.

Singleton rule: always use the launcher above. It stops the remembered director
process tree first, then launches one new visible director and records its root
PID in `solid_refactor_director.state.json`. Do not launch raw Codex director
windows by hand unless you immediately refresh the state file.

Dry run without launching:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\start-solid-refactor-director.ps1 -DryRun
```

Stop the current remembered director immediately:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\stop-solid-refactor-director.ps1
```

If the state file is missing but a stray director window remains, use the scan
fallback:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\stop-solid-refactor-director.ps1 -ScanFallback
```

Send a follow-up to the remembered director window:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\send-solid-refactor-director-followup.ps1 -Message "Go on. Keep director context compact, avoid broad code exploration, update the SOLID handoff, and delegate source analysis/edits to visible workers."
```

The follow-up script activates the remembered director window, copies the
message to the clipboard, pastes it, and presses Enter.

Resume the latest director Codex session in a fresh singleton director window:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\start-solid-refactor-director.ps1 -Mode Resume
```

Refresh the remembered director state after manual recovery or if a new PID was
launched outside the normal starter:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\remember-solid-refactor-director.ps1 -RootPid <PID>
```

The launcher writes state, marker, and transcript files:

- `.codex\workflow\solid-refactor-director-prompt.md`
- `.codex\workflow\agents\run-solid-refactor-director.ps1`
- `.codex\workflow\agents\solid_refactor_director.state.json`
- `.codex\workflow\agents\solid_refactor_director.exec.marker.txt`
- `.codex\workflow\agents\solid_refactor_director.exec.visible.log`

## Overseer Compaction Note

Before compacting the overseer session, make sure this memo contains the latest
director-control commands. After compaction, resume from these files only:

- `.codex\workflow\solid-refactor-overseer-memo.md`
- `.codex\workflow\solid-refactor-handoff.md`
- fresh worker handoffs under `.codex\workflow\agents\`

The compacted overseer should not become the director. It should keep context
small, keep only one director window, send follow-ups with the paste+Enter
script, and stop the director fast whenever it is no longer needed.

Compact the overseer early, around 30% context, not at the last moment. Before
compacting, send the director an Esc interrupt if it is mid-action, ask it to
update its handoff, and update this memo only with durable control facts.

## Director Oversight Strategy

Do not watch every terminal line from the director. That is expensive and turns
the overseer into a second director. Instead:

- Check the director periodically at coarse checkpoints: after a worker wave,
  before compaction, after a long idle period, or when the user asks for status.
- Use the checkpoint script first, and only escalate if its action line says to:
  `powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\check-solid-refactor-director.ps1`.
- The checkpoint reads only the director session tail, not the whole transcript,
  and reports the latest context-token percentage so the overseer can compact
  the director in time.
- When the checkpoint reports the director is at or above 30% context, do not
  merely ask the director to compact "when safe" and then wait indefinitely. Use
  the last few director messages from the checkpoint to judge whether the moment
  is good enough; it usually is. Then send `/compact` directly with:
  `powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\compact-solid-refactor-director.ps1`.
  That script sends Esc first, then `/compact`; sending `/compact` while the
  director is still mid-action often does not work. It then waits for the
  session log to become quiet and sends the post-compaction reminder.
- Prefer repo artifacts over terminal streaming: read only fresh worker handoffs,
  the director handoff, prompt files, marker files, and recent director talk that
  the checkpoint points to.
- Do not re-check singleton director processes during normal oversight. The
  start/stop/relaunch scripts own that. Only use those checks when a duplicate
  or relaunch problem is visible to the user.
- Spot-check rule compliance only: no source edits by director, no broad
  builds/tests before refactor completion, workers launched visibly, handoffs
  updated, and next wave prompts scoped.
- Keep the normal oversight budget to one checkpoint command and, if needed,
  one concise follow-up. Avoid ad hoc `Get-Process`, whole-session transcript
  reads/searches, broad git status, or source/doc exploration unless a checkpoint
  exposes a concrete problem.
- After every director interaction, run the checkpoint once and check the actual
  result. For compaction, verify that context-token percentage dropped
  substantially; quiet logs or a submitted prompt are not sufficient evidence.
- If recent director talk suggests it launched only one or two workers for broad
  remaining work, treat that as possible under-delegation. Include a brief note:
  "maybe you spawned too few sessions for current broad work; think of more
  possible subtasks and spawn more sessions according to your handoff."
- If the director drifts, send Esc with
  `.codex\workflow\agents\interrupt-solid-refactor-director.ps1`, then send one
  concise redirect follow-up. Do not start reading every line.
- If context reaches roughly 30%, interrupt if needed, ask the director to
  update its handoff, update this memo, compact, then reread only this memo plus
  `.codex\workflow\solid-refactor-handoff.md` and fresh worker handoffs.

## Paste+Enter Safety Gate

Do not aim automated paste+Enter at Codex until the disposable terminal canary
has passed in the current Windows session:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\test-terminal-paste-enter-canary.ps1
```

The canary launches a temporary visible PowerShell window, activates it, pastes a
one-line clipboard message, presses Enter, and verifies the received line through
`.codex\workflow\tmp\*.result.txt`. If the canary fails, fix
`.codex\workflow\agents\terminal-paste-enter.ps1` first; do not experiment on
the Codex director window.

Prove fast Esc delivery the same way before using it to interrupt Codex:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\test-terminal-escape-canary.ps1
```

Prove real Codex submit, including Enter after the first prompt and follow-up,
with:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\test-codex-director-submit-canary.ps1
```

That canary launches one director, pastes the initial prompt, presses Enter,
sends a follow-up, presses Enter again, then searches only for the unique canary
markers in `~\.codex\sessions\*.jsonl`. A marker hit means Codex received a
submitted turn, not just pasted text.

## Director Singleton Scripts

Use these scripts from the repo root:

```powershell
# Start exactly one visible director, stopping remembered/scan-found stale ones first.
# It launches interactive Codex first, then pastes the initial director prompt.
# It also records the actual visible window handle because Codex may change the
# terminal title after startup.
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\start-solid-refactor-director.ps1

# Resume the latest Codex conversation in exactly one visible director.
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\start-solid-refactor-director.ps1 -Mode Resume

# Stop the remembered director immediately; scan fallback also removes stale runner windows.
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\stop-solid-refactor-director.ps1 -ScanFallback

# Fast interrupt the remembered director's current action with Esc.
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\interrupt-solid-refactor-director.ps1

# Submit already-pasted text in the remembered director with a few Enter keys.
# Use this only as one scripted recovery action, not as manual key juggling.
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\submit-solid-refactor-director.ps1 -Repeat 3

# Recreate state if a live director was started manually and its root PID is known.
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\remember-solid-refactor-director.ps1 -RootPid <pid>
```

After the canary passes, send follow-ups with:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\send-solid-refactor-director-followup.ps1 -Message "go on, please continue refactoring; keep only source/static checks until the architecture refactor is complete"
```

The paste helpers use native `Ctrl+V`, then wait for the paste to settle based
on message length, then send Enter three times by default. Treat paste+Enter as
one atomic scripted action; do not paste first and manually submit later.

If Codex startup is slow, increase the initial prompt delay instead of sending
manual input into the window:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\start-solid-refactor-director.ps1 -InitialPromptDelaySeconds 10
```

If two directors appear, run the stop script with `-ScanFallback` immediately,
then restart one director. Do not manually juggle multiple director windows or
let the overseer become the director.

Use the interrupt script before `/compact`, urgent redirection, or stopping an
unwanted action. It prefers the remembered window handle, uses an 800 ms default
activation wait only as fallback, and sends only Esc; it is meant to be a fast
control operation, not a conversational follow-up.

## Director Drift Corrections

If the director starts a broad automatic review itself, interrupt it fast and
redirect. The director should not become the reviewer; it should delegate review
work to a fresh visible Codex worker session with a scoped prompt and a handoff.

Use one scripted interrupt:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\interrupt-solid-refactor-director.ps1
```

Then send one concise redirect:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\send-solid-refactor-director-followup.ps1 -Message "Stop broad self-review. Delegate review to a spawned visible codex-workers session with a scoped prompt and handoff. You are the director only: read handoffs, assign workers, integrate short handoffs, and keep context compact."
```

Do not stream every director line to detect this. Check at coarse checkpoints
from artifacts and visible state. Interrupt only when there is clear drift:
broad self-review, direct source edits by the director, hidden/background worker
launches, broad builds/tests before architecture completion, or duplicate
director windows.

## Director Post-Compaction Reminder

After the director compacts or resumes from compaction, send a reminder before
letting it continue. The reminder should restate the details the director tends
to forget:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\send-solid-refactor-director-followup.ps1 -Message "Post-compaction reminder: reread .codex\\workflow\\solid-refactor-handoff.md, docs\\current-project-architecture-solid-refactor-plan.md, docs\\current-project-architecture-solid-review.md, and fresh worker handoffs under .codex\\workflow\\agents\\. Continue as director only. Spawn real separate visible Codex worker windows via codex-workers as described in the handoff; do not do broad review or source edits yourself. Refactor and clear architecture boundaries first. Until the SOLID architecture refactor is genuinely complete, avoid broad builds/tests/schema generation/formatters/Bazel/lock refresh/release builds. Allowed checks are source/static checks such as rg, git diff --check, PowerShell parser checks for changed ps1, and scripts/check-cargo-dependency-boundaries.ps1 -SolidRefactor -Json. Keep worker prompts scoped, require short handoffs, and update your handoff before compacting again."
```

When compaction is triggered from the checkpoint, prefer the wrapper instead of
manual separate messages:

```powershell
powershell -ExecutionPolicy Bypass -File .codex\workflow\agents\compact-solid-refactor-director.ps1
```

The wrapper sends Esc first, then `/compact`, waits for the director session log
to become quiet, then sends a short post-compaction reminder that also warns
about spawning too few sessions for broad work.

If a new operational problem appears while overseeing, it is acceptable for the
overseer to change or add scripts and update this memo, then verify only that
workflow slice. Keep those fixes separate from product/refactor source.
