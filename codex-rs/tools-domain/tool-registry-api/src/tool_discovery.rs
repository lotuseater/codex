use std::collections::BTreeMap;

use serde::Serialize;

use crate::DiscoverableTool;
use crate::DiscoverableToolType;
use crate::JsonSchema;
use crate::JsonSchemaPrimitiveType;
use crate::JsonSchemaType;
use crate::LoadableToolSpec;
use crate::ResponsesApiNamespaceTool;
use crate::ResponsesApiTool;
use crate::ToolSpec;

const TUI_CLIENT_NAME: &str = "codex-tui";

pub const TOOL_SEARCH_TOOL_NAME: &str = "tool_search";
pub const TOOL_SEARCH_DEFAULT_LIMIT: usize = 8;
pub const REQUEST_PLUGIN_INSTALL_TOOL_NAME: &str = "request_plugin_install";
pub const LIST_AVAILABLE_PLUGINS_TO_INSTALL_TOOL_NAME: &str = "list_available_plugins_to_install";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolSearchSourceInfo {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RequestPluginInstallEntry {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tool_type: DiscoverableToolType,
    pub has_skills: bool,
    pub mcp_server_names: Vec<String>,
    pub app_connector_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ListAvailablePluginsToInstallResult {
    pub tools: Vec<RequestPluginInstallEntry>,
}

pub fn filter_request_plugin_install_discoverable_tools_for_client(
    discoverable_tools: Vec<DiscoverableTool>,
    app_server_client_name: Option<&str>,
) -> Vec<DiscoverableTool> {
    if app_server_client_name != Some(TUI_CLIENT_NAME) {
        return discoverable_tools;
    }

    discoverable_tools
        .into_iter()
        .filter(|tool| !matches!(tool, DiscoverableTool::Plugin(_)))
        .collect()
}

pub fn collect_request_plugin_install_entries(
    discoverable_tools: &[DiscoverableTool],
) -> Vec<RequestPluginInstallEntry> {
    discoverable_tools
        .iter()
        .map(|tool| match tool {
            DiscoverableTool::Connector(connector) => RequestPluginInstallEntry {
                id: connector.id.clone(),
                name: connector.name.clone(),
                description: connector.description.clone(),
                tool_type: DiscoverableToolType::Connector,
                has_skills: false,
                mcp_server_names: Vec::new(),
                app_connector_ids: Vec::new(),
            },
            DiscoverableTool::Plugin(plugin) => RequestPluginInstallEntry {
                id: plugin.id.clone(),
                name: plugin.name.clone(),
                description: plugin.description.clone(),
                tool_type: DiscoverableToolType::Plugin,
                has_skills: plugin.has_skills,
                mcp_server_names: plugin.mcp_server_names.clone(),
                app_connector_ids: plugin.app_connector_ids.clone(),
            },
        })
        .collect()
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
                if let Some(existing_namespace) = coalesced_specs.iter_mut().find_map(|spec| {
                    if let LoadableToolSpec::Namespace(existing_namespace) = spec
                        && existing_namespace.name == namespace.name
                    {
                        Some(existing_namespace)
                    } else {
                        None
                    }
                }) {
                    existing_namespace.tools.append(&mut namespace.tools);
                } else {
                    coalesced_specs.push(LoadableToolSpec::Namespace(namespace));
                }
            }
        }
    }

    coalesced_specs
}

pub fn create_tool_search_tool(
    searchable_sources: &[ToolSearchSourceInfo],
    default_limit: usize,
) -> ToolSpec {
    let mut properties = BTreeMap::from([
        (
            "query".to_string(),
            JsonSchema::string(Some("Search query for deferred tools.".to_string())),
        ),
        (
            "limit".to_string(),
            JsonSchema {
                schema_type: Some(JsonSchemaType::Multiple(vec![
                    JsonSchemaPrimitiveType::Number,
                    JsonSchemaPrimitiveType::Null,
                ])),
                description: Some(format!(
                    "Maximum number of tools to return (defaults to {default_limit})."
                )),
                ..Default::default()
            },
        ),
    ]);

    let mut required = vec!["query".to_string(), "limit".to_string()];

    let mut source_descriptions = BTreeMap::new();
    for source in searchable_sources {
        source_descriptions
            .entry(source.name.clone())
            .or_insert_with(|| source.description.clone());
    }

    if !source_descriptions.is_empty() {
        let source_text = source_descriptions
            .into_iter()
            .map(|(name, description)| match description {
                Some(description) if !description.trim().is_empty() => {
                    format!("- {name}: {description}")
                }
                _ => format!("- {name}"),
            })
            .collect::<Vec<_>>()
            .join("\n");
        properties.insert(
            "source".to_string(),
            JsonSchema {
                schema_type: Some(JsonSchemaType::Multiple(vec![
                    JsonSchemaPrimitiveType::String,
                    JsonSchemaPrimitiveType::Null,
                ])),
                description: Some(format!(
                    "Optional source filter. Available sources:\n{source_text}"
                )),
                ..Default::default()
            },
        );
        required.push("source".to_string());
    }

    ToolSpec::Function(ResponsesApiTool {
        name: TOOL_SEARCH_TOOL_NAME.to_string(),
        description: "Search deferred tools by name or description.".to_string(),
        strict: true,
        defer_loading: None,
        parameters: JsonSchema::object(properties, Some(required), Some(false.into())),
        output_schema: None,
    })
}

pub fn create_request_plugin_install_tool(
    discoverable_tools: &[RequestPluginInstallEntry],
    tool_search_available: bool,
) -> ToolSpec {
    let discoverable_tools = format_discoverable_tools(discoverable_tools);
    let discovery_guidance = if tool_search_available {
        "If one plugin or connector clearly fits, call `request_plugin_install`. If you need to inspect additional available tools first, use `tool_search`."
    } else {
        "If one plugin or connector clearly fits, call `request_plugin_install`."
    };

    ToolSpec::Function(ResponsesApiTool {
        name: REQUEST_PLUGIN_INSTALL_TOOL_NAME.to_string(),
        description: format!(
            "# Request plugin/connector install\n\
\n\
Use this tool only to ask the user to install one known plugin or connector from the list below. \
The list contains known candidates that are not currently installed.\n\
\n\
{discovery_guidance}\n\
\n\
Known plugins/connectors available to install:\n\
{discoverable_tools}"
        ),
        strict: true,
        defer_loading: None,
        parameters: JsonSchema::object(
            BTreeMap::from([
                (
                    "tool_type".to_string(),
                    JsonSchema::string(Some(
                        "Type of discoverable tool to suggest. Use \"connector\" or \"plugin\"."
                            .to_string(),
                    )),
                ),
                (
                    "action_type".to_string(),
                    JsonSchema::string(Some("Suggested action for the tool. Use \"install\".".to_string())),
                ),
                (
                    "tool_id".to_string(),
                    JsonSchema::string(Some(
                        "Connector or plugin id to suggest.".to_string(),
                    )),
                ),
                (
                    "suggest_reason".to_string(),
                    JsonSchema::string(Some(
                        "Concise one-line user-facing reason why this plugin or connector can help with the current request."
                            .to_string(),
                    )),
                ),
            ]),
            Some(vec![
                "tool_type".to_string(),
                "action_type".to_string(),
                "tool_id".to_string(),
                "suggest_reason".to_string(),
            ]),
            Some(false.into()),
        ),
        output_schema: None,
    })
}

fn format_discoverable_tools(discoverable_tools: &[RequestPluginInstallEntry]) -> String {
    let mut discoverable_tools = discoverable_tools.to_vec();
    discoverable_tools.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.id.cmp(&right.id))
    });

    discoverable_tools
        .into_iter()
        .map(|tool| {
            let description = tool_description_or_fallback(&tool);
            format!(
                "- {} (id: `{}`, type: {}, action: install): {}",
                tool.name,
                tool.id,
                tool.tool_type.as_str(),
                description
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn tool_description_or_fallback(tool: &RequestPluginInstallEntry) -> String {
    if let Some(description) = tool
        .description
        .as_deref()
        .map(str::trim)
        .filter(|description| !description.is_empty())
    {
        return description.to_string();
    }

    match tool.tool_type {
        DiscoverableToolType::Connector => "No description provided.".to_string(),
        DiscoverableToolType::Plugin => plugin_summary(tool),
    }
}

fn plugin_summary(tool: &RequestPluginInstallEntry) -> String {
    let mut details = Vec::new();
    if tool.has_skills {
        details.push("skills".to_string());
    }
    if !tool.mcp_server_names.is_empty() {
        details.push(format!("MCP servers: {}", tool.mcp_server_names.join(", ")));
    }
    if !tool.app_connector_ids.is_empty() {
        details.push(format!(
            "app connectors: {}",
            tool.app_connector_ids.join(", ")
        ));
    }

    if details.is_empty() {
        "No description provided.".to_string()
    } else {
        details.join("; ")
    }
}

pub fn loadable_tool_spec_name(spec: &LoadableToolSpec) -> &str {
    match spec {
        LoadableToolSpec::Function(tool) => tool.name.as_str(),
        LoadableToolSpec::Namespace(namespace) => namespace.name.as_str(),
    }
}

pub fn loadable_namespace_tools(spec: &LoadableToolSpec) -> Option<&[ResponsesApiNamespaceTool]> {
    match spec {
        LoadableToolSpec::Namespace(namespace) => Some(namespace.tools.as_slice()),
        LoadableToolSpec::Function(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tool_search_tool_spec_requires_nullable_optional_fields() {
        let tool = create_tool_search_tool(
            &[ToolSearchSourceInfo {
                name: "Google Drive".to_string(),
                description: None,
            }],
            TOOL_SEARCH_DEFAULT_LIMIT,
        );
        let value = serde_json::to_value(tool).expect("serialize tool_search tool");
        let parameters = value
            .get("parameters")
            .expect("tool_search should have parameters");

        assert_eq!(
            parameters
                .pointer("/properties/limit/type")
                .expect("limit should have a type"),
            &json!(["number", "null"])
        );
        assert_eq!(
            parameters
                .pointer("/properties/source/type")
                .expect("source should have a type"),
            &json!(["string", "null"])
        );
        assert_eq!(
            parameters
                .get("required")
                .expect("tool_search should declare required properties"),
            &json!(["query", "limit", "source"])
        );
    }
}
