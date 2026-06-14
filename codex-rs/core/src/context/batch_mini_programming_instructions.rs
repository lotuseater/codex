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

Keep simple tasks simple; keep batches small. Split when work needs user input, command execution, crosses permissions, is destructive, uncertain, or would hide which file, query, or step failed.
"#
            .to_string()
    }
}
