use codex_code_mode::CodeModeToolKind;
use codex_code_mode::ToolDefinition as CodeModeToolDefinition;
use codex_tool_execution_api::ToolName;
use codex_tool_registry_api::FreeformTool;
use codex_tool_registry_api::FreeformToolFormat;
use codex_tool_registry_api::ResponsesApiNamespaceTool;
use codex_tool_registry_api::ToolSpec;
use std::collections::BTreeMap;

pub(crate) fn create_code_mode_tool(
    enabled_tools: &[CodeModeToolDefinition],
    deferred_tools: &[CodeModeToolDefinition],
    namespace_descriptions: &BTreeMap<String, codex_code_mode::ToolNamespaceDescription>,
    code_mode_only: bool,
) -> ToolSpec {
    const CODE_MODE_FREEFORM_GRAMMAR: &str = r#"
start: pragma_source | plain_source
pragma_source: PRAGMA_LINE NEWLINE SOURCE
plain_source: SOURCE

PRAGMA_LINE: /[ \t]*\/\/ @exec:[^\r\n]*/
NEWLINE: /\r?\n/
SOURCE: /[\s\S]+/
"#;

    ToolSpec::Freeform(FreeformTool {
        name: codex_code_mode::PUBLIC_TOOL_NAME.to_string(),
        description: codex_code_mode::build_exec_tool_description(
            enabled_tools,
            deferred_tools,
            namespace_descriptions,
            code_mode_only,
        ),
        format: FreeformToolFormat {
            r#type: "grammar".to_string(),
            syntax: "lark".to_string(),
            definition: CODE_MODE_FREEFORM_GRAMMAR.to_string(),
        },
    })
}

pub(crate) fn collect_code_mode_tool_definitions<'a>(
    specs: impl IntoIterator<Item = &'a ToolSpec>,
) -> Vec<CodeModeToolDefinition> {
    let mut tool_definitions = specs
        .into_iter()
        .flat_map(code_mode_tool_definitions_for_spec)
        .filter(|definition| codex_code_mode::is_code_mode_nested_tool(&definition.name))
        .map(codex_code_mode::augment_tool_definition)
        .collect::<Vec<_>>();
    tool_definitions.sort_by(|left, right| left.name.cmp(&right.name));
    tool_definitions.dedup_by(|left, right| left.name == right.name);
    tool_definitions
}

fn code_mode_tool_definitions_for_spec(spec: &ToolSpec) -> Vec<CodeModeToolDefinition> {
    match spec {
        ToolSpec::Function(tool) => {
            let name = tool.name.clone();
            vec![CodeModeToolDefinition {
                tool_name: ToolName::plain(name.clone()),
                name,
                description: tool.description.clone(),
                kind: CodeModeToolKind::Function,
                input_schema: serde_json::to_value(&tool.parameters).ok(),
                output_schema: tool.output_schema.clone(),
            }]
        }
        ToolSpec::Namespace(namespace) => namespace
            .tools
            .iter()
            .map(|tool| match tool {
                ResponsesApiNamespaceTool::Function(tool) => {
                    let tool_name = ToolName::namespaced(namespace.name.clone(), tool.name.clone());
                    CodeModeToolDefinition {
                        name: code_mode_name_for_tool_name(&tool_name),
                        tool_name,
                        description: tool.description.clone(),
                        kind: CodeModeToolKind::Function,
                        input_schema: serde_json::to_value(&tool.parameters).ok(),
                        output_schema: tool.output_schema.clone(),
                    }
                }
            })
            .collect(),
        ToolSpec::Freeform(tool) => {
            let name = tool.name.clone();
            vec![CodeModeToolDefinition {
                tool_name: ToolName::plain(name.clone()),
                name,
                description: tool.description.clone(),
                kind: CodeModeToolKind::Freeform,
                input_schema: None,
                output_schema: None,
            }]
        }
        ToolSpec::ImageGeneration { .. }
        | ToolSpec::LocalShell {}
        | ToolSpec::ToolSearch { .. }
        | ToolSpec::WebSearch { .. } => Vec::new(),
    }
}

fn code_mode_name_for_tool_name(tool_name: &ToolName) -> String {
    match tool_name.namespace.as_deref() {
        Some(namespace) if namespace.ends_with('_') || tool_name.name.starts_with('_') => {
            format!("{namespace}{}", tool_name.name)
        }
        Some(namespace) => format!("{namespace}_{}", tool_name.name),
        None => tool_name.name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_tool_execution_api::ToolName;
    use pretty_assertions::assert_eq;

    #[test]
    fn create_code_mode_tool_matches_expected_spec() {
        let enabled_tools = vec![codex_code_mode::ToolDefinition {
            name: "update_plan".to_string(),
            tool_name: ToolName::plain("update_plan"),
            description: "Update the plan".to_string(),
            kind: codex_code_mode::CodeModeToolKind::Function,
            input_schema: None,
            output_schema: None,
        }];

        assert_eq!(
            create_code_mode_tool(
                &enabled_tools,
                &[],
                &BTreeMap::new(),
                /*code_mode_only*/ true,
            ),
            ToolSpec::Freeform(FreeformTool {
                name: codex_code_mode::PUBLIC_TOOL_NAME.to_string(),
                description: codex_code_mode::build_exec_tool_description(
                    &enabled_tools,
                    &[],
                    &BTreeMap::new(),
                    /*code_mode_only*/ true,
                ),
                format: FreeformToolFormat {
                    r#type: "grammar".to_string(),
                    syntax: "lark".to_string(),
                    definition: r#"
start: pragma_source | plain_source
pragma_source: PRAGMA_LINE NEWLINE SOURCE
plain_source: SOURCE

PRAGMA_LINE: /[ \t]*\/\/ @exec:[^\r\n]*/
NEWLINE: /\r?\n/
SOURCE: /[\s\S]+/
"#
                    .to_string(),
                },
            })
        );
    }
}
