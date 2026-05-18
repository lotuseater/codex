use super::ContextualUserFragment;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BatchMiniProgrammingInstructions;

impl ContextualUserFragment for BatchMiniProgrammingInstructions {
    const ROLE: &'static str = "developer";
    const START_MARKER: &'static str = "<batch_mini_programming_instructions>";
    const END_MARKER: &'static str = "</batch_mini_programming_instructions>";

    fn body(&self) -> String {
        r#"
`workflow_batch` tool is available for dependent deterministic local workflows.
Use it proactively for root-confined file/JSON IO, metadata/listing, transforms, edits, assertions, loops, and reductions. For those workflows, choose `workflow_batch` before shell commands or separate file-edit calls. Use normal focused tools for single read-only probes, one-off searches, and unbounded repo-wide scans. Use `workflow_batch` for bounded recursive conditional scans that chain listing, filtering, reads, assertions, or generated outputs.

Command execution remains on the normal approval path outside this batch surface. Prefer Python when a task needs arbitrary algorithms, richer data structures, generated fixtures, or reusable prototypes. Prefer cmd only for cmd/batch-specific Windows behavior.

Top-level arguments: provide exactly one of inline `spec` JSON or `spec_path`; optional `workdir`, `report_path`, and `log_path` keep execution root-confined.
Spec shape: `{"steps":[...]}`. Steps may have `id` and `if`; branch steps may use `then`/`else`.
Step keywords: `set`, `set_vars`, `ensure_dir`, `stat_path`, `list_files`, `read_file`, `read_json`, `write_file`, `append_file`, `write_json`, `copy_file`, `edit_file`, `assert`, `for_each`, and `while`.

PowerShell substitutions: use `stat_path` for metadata, `list_files` for constrained directory scans, `ensure_dir` for directory creation, `read_file`/`write_file`/`append_file` for text files, and `read_json`/`write_json` plus expressions for JSON.

Expression/composite types: expressions are JSON values; use refs such as `{"ref":"name"}` or `{"ref":"steps.step_id"}`. Wrap object records via literal, e.g. `{"literal":{"sum":10}}`. Sets are arrays with `unique`, `set_union`, `set_intersection`, `set_difference`, and `set_includes`.
Functional collection usage: prefer `map`, `filter`, `reduce`, `scan`, quantifiers, object helpers, and comparisons inside `set`, `write_json`, `assert`, or loop conditions instead of spilling temporary files.
Assertion usage: prefer structured expressions plus concise `message` diagnostics.

Keep batches compact: include only necessary dependent steps and split when work needs user input, command execution, crosses permissions, is destructive, or would be harder to diagnose if batched.
"#
            .to_string()
    }
}
