use super::ContextualUserFragment;
use crate::config::ActionOptimizationInstructionsConfig;
use crate::config::ActionOptimizationInstructionsVariant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActionOptimizationInstructions {
    variant: ActionOptimizationInstructionsVariant,
    max_tokens: usize,
}

impl ActionOptimizationInstructions {
    pub(crate) fn from_config(config: &ActionOptimizationInstructionsConfig) -> Option<Self> {
        (config.max_tokens > 0).then_some(Self {
            variant: config.variant,
            max_tokens: config.max_tokens,
        })
    }
}

impl ContextualUserFragment for ActionOptimizationInstructions {
    fn role(&self) -> &'static str {
        "developer"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        (
            "<action_optimization_instructions>",
            "</action_optimization_instructions>",
        )
    }

    fn body(&self) -> String {
        let body = match self.variant {
            ActionOptimizationInstructionsVariant::ActionRouteSelection => {
                "Keep simple tasks simple: answer directly when enough evidence is present, or run one focused command/read when that decides the next branch. Select the lightest route that still verifies the work: use shell/rg for one-off facts; use a tiny script or workflow_batch for repetitive deterministic local file/JSON work; reproduce before repair; plan, delegate, or batch only when ordering, ambiguity, or repeated work justifies the overhead."
            }
        };

        body.split_whitespace()
            .take(self.max_tokens)
            .collect::<Vec<_>>()
            .join(" ")
    }
}
