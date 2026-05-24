use crate::ResponsesApiTool;
use crate::ToolSpec;
use codex_agent_policy::MAIN_AGENT_PLAN_DELEGATION_PROMPT;
use codex_tool_schema::JsonSchema;
use std::collections::BTreeMap;

pub fn create_update_plan_tool() -> ToolSpec {
    let plan_item_properties = BTreeMap::from([
        ("step".to_string(), JsonSchema::string(/*description*/ None)),
        (
            "status".to_string(),
            JsonSchema::string(Some("One of: pending, in_progress, completed".to_string())),
        ),
    ]);

    let properties = BTreeMap::from([
        (
            "explanation".to_string(),
            JsonSchema::string(/*description*/ None),
        ),
        (
            "plan".to_string(),
            JsonSchema::array(
                JsonSchema::object(
                    plan_item_properties,
                    Some(vec!["step".to_string(), "status".to_string()]),
                    Some(false.into()),
                ),
                Some("The list of steps".to_string()),
            ),
        ),
    ]);

    ToolSpec::Function(ResponsesApiTool {
        name: "update_plan".to_string(),
        description: [
            r#"Updates the task plan.
Provide an optional explanation and a list of plan items, each with a step and status.
At most one step can be in_progress at a time.
"#,
            MAIN_AGENT_PLAN_DELEGATION_PROMPT,
            "\n",
        ]
        .concat(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["plan".to_string()]),
            Some(false.into()),
        ),
        output_schema: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_plan_description_includes_delegation_policy() {
        let ToolSpec::Function(tool) = create_update_plan_tool() else {
            panic!("update_plan should be a function tool");
        };

        assert!(tool.description.contains("what to delegate to subagents"));
        assert!(
            tool.description
                .contains("short summary or short result only when")
        );
        assert!(
            tool.description
                .contains("including update_plan calls outside Plan mode")
        );
        assert!(
            tool.description
                .contains("context drift or context compactions")
        );
        assert!(tool.description.contains("even one worker is useful"));
        assert!(tool.description.contains("5 minutes between checks"));
        assert!(tool.description.contains("portable PowerShell"));
        assert!(tool.description.contains("Start-Process powershell"));
    }
}
