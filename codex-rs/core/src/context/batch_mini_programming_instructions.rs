use super::ContextualUserFragment;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BatchMiniProgrammingInstructions;

impl ContextualUserFragment for BatchMiniProgrammingInstructions {
    fn role(&self) -> &'static str {
        "developer"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        (
            "<batch_mini_programming_instructions>",
            "</batch_mini_programming_instructions>",
        )
    }

    fn body(&self) -> String {
        r#"
Use `workflow_batch` when several deterministic local file/JSON reads, bounded scans, transforms, assertions, loops, or reductions are already clear and one diagnosable batch beats repeated tool calls. Use focused shell/rg for one-off searches and Python for richer algorithms or reusable prototypes.

`workflow_batch` args: exactly one of `spec` or `spec_path`, plus optional `workdir`, `report_path`, and `log_path`. `spec` is `{"steps":[...]}`; step payloads are objects; never include `response_length`.

Keep simple tasks simple. Once the branch conditions are known and the work is deterministic, prefer ONE larger combined batch/script per step over many small calls: each extra separate call re-pays the fixed per-turn tool-call envelope while doing no additional verified work, so group all clear reads, scans, transforms, assertions, loops and reductions into a single diagnosable workflow_batch to do more verified work in fewer turns and fewer total tokens. This is purely a token-and-turn win when the steps are deterministic and the failing step is still reported, never a correctness trade-off. Keep steps small only while the problem is still being understood (you need an intermediate result to choose the next action), or when a step needs user input, command execution, crosses a permission boundary, is destructive, or would hide which file/query/step failed.
"#
            .to_string()
    }
}
