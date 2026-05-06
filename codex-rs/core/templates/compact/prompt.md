You are performing a CONTEXT CHECKPOINT COMPACTION. Create a handoff summary for another LLM that will resume the task.

Include:
- The active user goal/request, preserving wording for important constraints
- The current plan/checklist with completed and pending status when present
- Current progress and key decisions made
- Important context, constraints, or user preferences
- What remains to be done (clear next steps)
- Build/test/deploy status, unresolved blockers, and any critical data, examples, or references needed to continue

If task memory is provided separately in a `<task_memory>` item, do not repeat the full prompt or plan verbatim in the summary; preserve only the surrounding progress, decisions, status, and next actions needed to use that task memory correctly.

Be concise, structured, and focused on helping the next LLM seamlessly continue the work.
