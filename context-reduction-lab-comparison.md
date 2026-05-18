# Context Reduction Lab Comparison

Date: 2026-05-18

Source lab: `C:\Users\Oleh\Documents\GitHub\context-reducer-lab`

## Answer

Yes, the newer context reduction is better than the previous reducer on the same whole-prompt packet matrix, but the measured gain over the previous reducer is modest. The important correction is that the earlier weak result was measured with the wrong packet shape.

The misleading run treated each packet as one recent text item, so `preserve_recent_items=4` protected almost everything:

| Run | Cases | Original tokens | Saved tokens | Saved % | Artifacts | Forbidden reductions |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Old single-item matrix, preserve 4 | 12 | 31,904 | 107 | 0.34% | 1 | 0 |
| Old single-item matrix, preserve 0 | 12 | 31,904 | 517 | 1.62% | 2 | 0 |

Rerunning the lab with `matrix_prompt_packets.json` as fixtures preserves the real multi-item packet structure:

| Reducer | Preserve recent | Cases | Original tokens | Saved tokens | Saved % | Artifacts | Forbidden reductions |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Previous HEAD | 4 | 8 | 16,613 | 9,742 | 58.64% | 16 | 0 |
| New/current | 4 | 8 | 16,613 | 9,928 | 59.76% | 18 | 0 |
| Previous HEAD | 0 | 8 | 16,613 | 11,622 | 69.96% | 18 | 0 |
| New/current | 0 | 8 | 16,613 | 11,865 | 71.42% | 21 | 0 |

## Interpretation

The new version is proven better on this lab matrix:

- Default safety mode improves by 186 saved tokens, from 58.64% to 59.76%.
- Preserve-recent-0 improves by 243 saved tokens, from 69.96% to 71.42%.
- Both current runs had zero forbidden reductions.
- The whole-prompt result is now in the same range as the tool-output run, which saved 11,690 of 16,613 tokens (70.37%).

The main conclusion changed from "not proven" to "proven better, but only slightly better than the previous reducer." The big apparent jump comes from measuring whole prompt packets correctly instead of wrapping each packet as one recent item.

## Reproduction Notes

Correct current run:

```powershell
target\debug\prompt_reduce_lab.exe `
  --fixtures reports\current-codex-batch-workflow-2026-05-17\parsed\matrix_prompt_packets.json `
  --report-name codex_current_matrix_fixtures `
  --artifact-dir reports\codex_current_matrix_fixtures_artifacts `
  --json
```

Correct current preserve-0 run:

```powershell
target\debug\prompt_reduce_lab.exe `
  --fixtures reports\current-codex-batch-workflow-2026-05-17\parsed\matrix_prompt_packets.json `
  --report-name codex_current_matrix_fixtures_preserve0 `
  --artifact-dir reports\codex_current_matrix_fixtures_preserve0_artifacts `
  --preserve-recent-items 0 `
  --json
```

Previous reducer was measured in a detached `context-reducer-lab` worktree at `HEAD`, using the same fixture file and shared lab target cache.
