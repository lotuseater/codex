# Replacement Benchmark Results

Date: 2026-05-07

Project: `C:\Users\Oleh\Documents\GitHub\open_ai\codex`

Script: `scripts/measure-operation-replacements.ps1`

## Commands

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\measure-operation-replacements.ps1 -Task GitSummary,SessionFind,FileOutline -RecentDays 3 -Limit 5 -FilePath codex-rs/core/src/tools/handlers/shell.rs -MaxOutlineItems 200
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\measure-operation-replacements.ps1 -Task SearchText -Pattern first_moves -MaxFiles 50 -MaxMatchesPerFile 5 -Json
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\measure-operation-replacements.ps1 -Task FileOutline -FilePath codex-rs/tui/src/chatwidget.rs -MaxOutlineItems 700 -Json
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\measure-operation-replacements.ps1 -Task RunCheck -Json
```

## Results

| Operation | Verdict | Baseline tokens | Candidate tokens | Savings | Quality notes |
|---|---|---:|---:|---:|---|
| `git_worktree_summary` | `pass` | 92,308 | 193 | 99.8% | Preserved the full changed-file set from porcelain status. |
| `session_find` | `pass` | 212 | 210 | 0.9% | Correctness passed; script now reads `state_5.sqlite` before JSONL fallback. Native thread-store or DAB lookup is still needed for millisecond latency. |
| `search_text` | `pass` | 2,820 | 1,445 | 48.8% | Preserved all 26 baseline files for `first_moves` with `MaxFiles=50`. |
| `file_outline` on `shell.rs` | `pass` | 6,952 | 969 | 86.1% | Preserved all 58 detected definitions. |
| `file_outline` on `chatwidget.rs` | `pass` | 113,277 | 5,957 | 94.7% | Preserved all 599 detected definitions with `MaxOutlineItems=700`. |
| `run_check_digest` | `pass` | 4,965 | 95 | 98.1% | Stored full noisy output under `logs/operation-replacement-artifacts` and returned diagnostics plus artifact path. |

## Gate Lessons

- `git_worktree_summary`, `search_text`, and `file_outline` are ready for native
  shadow-mode prototypes because they showed large token savings without missed
  required facts in these runs.
- `session_find` should not be promoted for token savings yet. It should be
  ported to the native thread store or DAB live lookup to avoid PowerShell and
  sqlite process startup, with JSONL scans kept only as fallback.
- Outline caps must be quality-gated. `chatwidget.rs` with
  `MaxOutlineItems=500` omitted 99 definitions and correctly required fallback;
  `MaxOutlineItems=700` preserved all detected definitions and still saved more
  than 94 percent.
- `run_check_digest` is the artifact-backed chain-cache prototype. It saves
  prompt tokens because later turns can carry a digest and artifact handle
  instead of replaying the raw command output.

## Session Token Examples

`scripts/find-codex-sessions.ps1` now reads `tokens_used` from `state_5.sqlite`
when available. In the final verification pass, it found these high-spend live
or recently active sessions without reading full JSONL logs:

| Project | Session | Tokens used | Source |
|---|---|---:|---|
| `open_ai\codex` | current token-saving conversation | 37,057,253 | `state_5.sqlite` |
| `open_ai\codex` | older status-line/build conversation | 572,512,533 | `state_5.sqlite` |
| `Serial_to_Google_Doc_topdown` | C++ transfer conversation | 1,212,424,510 | `state_5.sqlite` |

These numbers explain why session/log discovery must start from indexed thread
state: the largest conversations can be identified from metadata before any
large transcript is opened.
