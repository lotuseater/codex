Here is the context left by another LLM model. Reduce it for the next model that will continue the same task. 

Remove from the context all not needed for further task implementation by the model. Preserve all that may be useful.

Preserve everything needed to continue implementation without rediscovery:
- The user's goal and any explicit constraints, preferences, or requested workflow.
- The active plan, its current implementation stage, which items are completed, which item is in progress, and which items are not started.
- The next concrete actions to take, including file paths, commands, tests, artifacts, or logs needed to resume.
- Important decisions, assumptions, blockers, risks, verification results, and dirty or user-owned worktree changes that must not be overwritten.
- Any important code/session details that would be costly or unsafe to rediscover.

Remove obsolete exploration, repeated tool output, dead ends, and low-signal narration. Do not omit unresolved work or collapse it into vague phrases such as "continue the plan"; name the exact remaining steps.

Return only the reduced context. Do not explain your method.
