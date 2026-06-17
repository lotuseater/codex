use super::ContextualUserFragment;
use crate::config::BatchMiniProgrammingInstructionsConfig;
use crate::config::BatchMiniProgrammingInstructionsVariant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BatchMiniProgrammingInstructions {
    variant: BatchMiniProgrammingInstructionsVariant,
    custom_text: Option<String>,
}

impl BatchMiniProgrammingInstructions {
    pub(crate) fn from_config(config: &BatchMiniProgrammingInstructionsConfig) -> Self {
        Self {
            variant: config.variant,
            custom_text: config.custom_text.clone(),
        }
    }
}

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
        if let Some(custom) = self.custom_text.as_deref()
            && !custom.trim().is_empty()
        {
            return custom.to_string();
        }

        match self.variant {
            BatchMiniProgrammingInstructionsVariant::Current => r#"
Use `workflow_batch` when several deterministic local file/JSON reads, bounded scans, transforms, assertions, loops, or reductions are already clear and one diagnosable batch beats repeated tool calls. Use focused shell/rg for one-off searches and Python for richer algorithms or reusable prototypes.

`workflow_batch` args: exactly one of `spec` or `spec_path`, plus optional `workdir`, `report_path`, and `log_path`. `spec` is `{"steps":[...]}`; step payloads are objects; never include `response_length`.

Keep simple tasks simple. Once the branch conditions are known and the work is deterministic, prefer ONE larger combined batch/script per step over many small calls: each extra separate call re-pays the fixed per-turn tool-call envelope while doing no additional verified work, so group all clear reads, scans, transforms, assertions, loops and reductions into a single diagnosable workflow_batch to do more verified work in fewer turns and fewer total tokens. This is purely a token-and-turn win when the steps are deterministic and the failing step is still reported, never a correctness trade-off. Keep steps small only while the problem is still being understood (you need an intermediate result to choose the next action), or when a step needs user input, command execution, crosses a permission boundary, is destructive, or would hide which file/query/step failed.
"#
            .to_string(),
            BatchMiniProgrammingInstructionsVariant::Aggressive => {
                "Maximize completed local work per tool call by batching every deterministic scan, transform, assertion, reduction, and loop that can share one root-confined workflow. Prefer a single `workflow_batch` unless user input, destructive work, or command execution is required."
                    .to_string()
            }
            BatchMiniProgrammingInstructionsVariant::Compact => {
                "Use `workflow_batch` for compact root-confined deterministic local file/JSON IO, bounded scans, transforms, assertions, loops, and reductions. Use focused shell/rg for one-off searches and Python for richer algorithms or reusable prototypes. `workflow_batch` args: exactly one of `spec` or `spec_path`, plus optional `workdir`, `report_path`, and `log_path`. `spec` is {\"steps\":[...]}; step payloads are objects; never include `response_length`. Keep batches small; split when work needs user input, command execution, crosses permissions, is destructive, or would be harder to diagnose if batched."
                    .to_string()
            }
        }
    }
}
