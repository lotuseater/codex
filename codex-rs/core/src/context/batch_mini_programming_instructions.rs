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
Use `workflow_batch` for compact root-confined deterministic local file/JSON IO, bounded scans, transforms, assertions, loops, and reductions. Use focused shell/rg for one-off searches and Python for richer algorithms or reusable prototypes.

`workflow_batch` args: exactly one of `spec` or `spec_path`, plus optional `workdir`, `report_path`, and `log_path`. `spec` is `{"steps":[...]}`; step payloads are objects; never include `response_length`.

Keep batches small; split when work needs user input, command execution, crosses permissions, is destructive, or would be harder to diagnose if batched.
"#
            .to_string()
    }
}
