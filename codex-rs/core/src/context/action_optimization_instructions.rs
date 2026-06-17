use super::ContextualUserFragment;
use crate::config::ActionOptimizationInstructionsConfig;
use crate::config::ActionOptimizationInstructionsVariant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionOptimizationInstructions {
    variant: ActionOptimizationInstructionsVariant,
    custom_text: Option<String>,
    max_tokens: usize,
}

impl ActionOptimizationInstructions {
    pub(crate) fn from_config(config: &ActionOptimizationInstructionsConfig) -> Option<Self> {
        (config.max_tokens > 0).then(|| Self {
            variant: config.variant,
            custom_text: config.custom_text.clone(),
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
        let body = match self.custom_text.as_deref() {
            Some(custom) if !custom.trim().is_empty() => custom,
            _ => match self.variant {
                ActionOptimizationInstructionsVariant::ActionRouteSelection => {
                    "Answer directly when evidence suffices. Else: one focused read/command to decide the branch; batch repeated deterministic work; split risky/opaque steps; verify before reporting."
                }
                ActionOptimizationInstructionsVariant::Routing => {
                    "Route each next action by shape: direct answer, targeted read/search, batch/script, prototype, delegate, or verify. Keep simple tasks simple, use the smallest useful probe, split risky or opaque work, preserve diagnosis, use waits for non-contending work, respect permissions, and pick the lowest-overhead route that still produces reliable evidence."
                }
                ActionOptimizationInstructionsVariant::Verbose => {
                    "Keep simple tasks simple: answer directly when enough evidence is present, or run one focused command/read when that decides the next branch. Select the lightest route that still verifies the work: use shell/rg for one-off facts; use a tiny script or workflow_batch for repetitive deterministic local file/JSON work; reproduce before repair; plan, delegate, or batch only when ordering, ambiguity, or repeated work justifies the overhead."
                }
            },
        };

        body.split_whitespace()
            .take(self.max_tokens)
            .collect::<Vec<_>>()
            .join(" ")
    }
}
