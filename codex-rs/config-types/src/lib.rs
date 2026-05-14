use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use ts_rs::TS;

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
pub enum ContextBudgetMode {
    Standard,
    #[default]
    Slow,
}

impl fmt::Display for ContextBudgetMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Standard => "standard",
            Self::Slow => "slow",
        };
        f.write_str(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_budget_mode_defaults_to_slow() {
        assert_eq!(ContextBudgetMode::Slow, ContextBudgetMode::default());
    }

    #[test]
    fn context_budget_mode_wire_values_stay_lowercase() {
        assert_eq!("slow", ContextBudgetMode::Slow.to_string());
        assert_eq!("standard", ContextBudgetMode::Standard.to_string());
        assert_eq!(
            "\"slow\"",
            serde_json::to_string(&ContextBudgetMode::Slow).expect("serialize slow mode")
        );
        assert_eq!(
            ContextBudgetMode::Standard,
            serde_json::from_str("\"standard\"").expect("deserialize standard mode")
        );
    }
}
