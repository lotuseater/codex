use super::ContextualUserFragment;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BatchMiniProgrammingInstructions;

impl ContextualUserFragment for BatchMiniProgrammingInstructions {
    const ROLE: &'static str = "developer";
    const START_MARKER: &'static str = "<batch_mini_programming_instructions>";
    const END_MARKER: &'static str = "</batch_mini_programming_instructions>";

    fn body(&self) -> String {
        "When the `workflow_batch` tool is available, treat it as Codex's command-free batch mini-programming surface for dependent deterministic local workflows.\n\
Use it aggressively when later steps depend on earlier results, or when root-confined file/JSON IO, edits, assertions, conditions, loops, or reductions can replace several separate tool calls without user input.\n\
Do not use it for simple read-only probes, one-off searches, or broad repo scans where normal focused tools are shorter and easier to inspect.\n\
It supports inline `spec` JSON or `spec_path`, optional `workdir`, report/log paths, variables, expressions, `if`, `for_each`, `while`, `set`/`set_vars`, `read_file`, `read_json`, `write_file`, `write_json`, `copy_file`, `edit_file`, and `assert` steps. Do not use it for command execution unless the tool schema explicitly exposes command support.\n\
Prefer inline `spec` for one-shot batches. Use `spec_path` only for reusable, large, or already-existing canaries.\n\
Keep batches compact: include only the necessary dependent steps, concise variables, assertions or early exits, and rely on the compact tool summary plus report/log artifacts. Split when an operation needs user input, command execution, crosses the active permission boundary, is destructive or irreversible, or would be materially harder to diagnose if batched."
                .to_string()
    }
}
