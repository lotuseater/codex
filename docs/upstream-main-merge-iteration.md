# Upstream Main Merge Iteration

Use this workflow before recurring merges from `upstream/main` into the working branch.

1. Fetch the latest upstream main:
   `git fetch upstream main`
2. Start from the current working branch and create a temporary rehearsal branch or worktree.
3. In the temporary branch, merge `upstream/main` without treating the result as final.
4. Record the actual conflicts and identify which ones come from local feature logic living in upstream-hot files.
5. Refactor the temporary branch to move that local logic into clearer owner crates/modules/files, leaving broad upstream-owned files as thin adapters.
6. Retry the merge rehearsal after the refactor and compare the conflict set.
7. When the merge result is clean and the refactor is better long-term, apply or merge the successful result back to the working branch.
8. Verify, build, deploy, commit, and push the working branch.

Do not use the temporary branch as a shortcut around maintainability problems. Its purpose is to discover conflict pressure, improve modularity, and only then bring the successful merge result back to the working branch.
