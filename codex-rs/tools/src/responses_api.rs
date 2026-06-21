use crate::ToolDefinition;
use crate::ToolName;
use crate::parse_dynamic_tool;
use crate::parse_mcp_tool;
use codex_protocol::dynamic_tools::DynamicToolFunctionSpec;
use codex_protocol::dynamic_tools::DynamicToolNamespaceTool;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_tool_registry_api::LoadableToolSpec;
use codex_tool_registry_api::ResponsesApiNamespace;
use codex_tool_registry_api::ResponsesApiNamespaceTool;
use codex_tool_registry_api::ResponsesApiTool;

pub fn default_namespace_description(namespace_name: &str) -> String {
    format!("Tools in the {namespace_name} namespace.")
}

pub fn dynamic_tool_to_responses_api_tool(
    tool: &DynamicToolFunctionSpec,
) -> Result<ResponsesApiTool, serde_json::Error> {
    Ok(tool_definition_to_responses_api_tool(parse_dynamic_tool(
        tool,
    )?))
}

pub fn dynamic_tool_to_loadable_tool_spec(
    tool: &DynamicToolSpec,
) -> Result<LoadableToolSpec, serde_json::Error> {
    Ok(match tool {
        DynamicToolSpec::Function(function) => {
            LoadableToolSpec::Function(dynamic_tool_to_responses_api_tool(function)?)
        }
        DynamicToolSpec::Namespace(namespace) => {
            let description = if namespace.description.trim().is_empty() {
                default_namespace_description(&namespace.name)
            } else {
                namespace.description.clone()
            };
            let mut tools = Vec::with_capacity(namespace.tools.len());
            for tool in &namespace.tools {
                let DynamicToolNamespaceTool::Function(function) = tool;
                tools.push(ResponsesApiNamespaceTool::Function(
                    dynamic_tool_to_responses_api_tool(function)?,
                ));
            }
            LoadableToolSpec::Namespace(ResponsesApiNamespace {
                name: namespace.name.clone(),
                description,
                tools,
            })
        }
    })
}

pub fn coalesce_loadable_tool_specs(
    specs: impl IntoIterator<Item = LoadableToolSpec>,
) -> Vec<LoadableToolSpec> {
    let mut coalesced_specs = Vec::new();
    for spec in specs {
        match spec {
            LoadableToolSpec::Function(tool) => {
                coalesced_specs.push(LoadableToolSpec::Function(tool));
            }
            LoadableToolSpec::Namespace(mut namespace) => {
                if let Some(existing_namespace) =
                    coalesced_specs.iter_mut().find_map(|spec| match spec {
                        LoadableToolSpec::Namespace(existing_namespace)
                            if existing_namespace.name == namespace.name =>
                        {
                            Some(existing_namespace)
                        }
                        LoadableToolSpec::Function(_) | LoadableToolSpec::Namespace(_) => None,
                    })
                {
                    existing_namespace.tools.append(&mut namespace.tools);
                } else {
                    coalesced_specs.push(LoadableToolSpec::Namespace(namespace));
                }
            }
        }
    }
    coalesced_specs
}

pub fn mcp_tool_to_responses_api_tool(
    tool_name: &ToolName,
    tool: &rmcp::model::Tool,
) -> Result<ResponsesApiTool, serde_json::Error> {
    Ok(tool_definition_to_responses_api_tool(
        parse_mcp_tool(tool)?.renamed(tool_name.name.clone()),
    ))
}

pub fn mcp_tool_to_deferred_responses_api_tool(
    tool_name: &ToolName,
    tool: &rmcp::model::Tool,
) -> Result<ResponsesApiTool, serde_json::Error> {
    Ok(tool_definition_to_responses_api_tool(
        parse_mcp_tool(tool)?
            .renamed(tool_name.name.clone())
            .into_deferred(),
    ))
}

pub fn tool_definition_to_responses_api_tool(tool_definition: ToolDefinition) -> ResponsesApiTool {
    ResponsesApiTool {
        name: tool_definition.name,
        description: tool_definition.description,
        strict: false,
        defer_loading: tool_definition.defer_loading.then_some(true),
        parameters: tool_definition.input_schema,
        output_schema: tool_definition.output_schema,
    }
}

#[cfg(test)]
#[path = "responses_api_tests.rs"]
mod tests;
