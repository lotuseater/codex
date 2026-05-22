use codex_tool_registry_api::JsonSchema;
use codex_tool_registry_api::JsonSchemaPrimitiveType;
use codex_tool_registry_api::JsonSchemaType;
use codex_tool_registry_api::TOOL_SEARCH_TOOL_NAME;
use codex_tool_registry_api::ToolSearchSourceInfo;
use codex_tool_registry_api::ToolSpec;
use std::collections::BTreeMap;

pub(crate) fn create_tool_search_tool(
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
            .and_modify(|existing: &mut Option<String>| {
                if existing.is_none() {
                    *existing = source.description.clone();
                }
            })
            .or_insert(source.description.clone());
    }

    let has_sources = !source_descriptions.is_empty();
    let source_descriptions = if !has_sources {
        "None currently enabled.".to_string()
    } else {
        source_descriptions
            .into_iter()
            .map(|(name, description)| match description {
                Some(description) => format!("- {name}: {description}"),
                None => format!("- {name}"),
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    if has_sources {
        properties.insert(
            "source".to_string(),
            JsonSchema {
                schema_type: Some(JsonSchemaType::Multiple(vec![
                    JsonSchemaPrimitiveType::String,
                    JsonSchemaPrimitiveType::Null,
                ])),
                description: Some(format!(
                    "Optional source filter. Available sources:\n{source_descriptions}"
                )),
                ..Default::default()
            },
        );
        required.push("source".to_string());
    }

    let description = format!(
        "# Tool discovery\n\nSearches over deferred tool metadata with BM25 and exposes matching tools for the next model call.\n\nYou have access to tools from the following sources:\n{source_descriptions}\nSome of the tools may not have been provided to you upfront, and you should use this tool (`{TOOL_SEARCH_TOOL_NAME}`) to search for the required tools. For MCP tool discovery, always use `{TOOL_SEARCH_TOOL_NAME}` instead of `list_mcp_resources` or `list_mcp_resource_templates`."
    );

    ToolSpec::ToolSearch {
        execution: "client".to_string(),
        description,
        parameters: JsonSchema::object(properties, Some(required), Some(false.into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_tool_registry_api::JsonSchema;
    use pretty_assertions::assert_eq;
    use std::collections::BTreeMap;

    #[test]
    fn create_tool_search_tool_deduplicates_and_renders_enabled_sources() {
        assert_eq!(
            create_tool_search_tool(
                &[
                    ToolSearchSourceInfo {
                        name: "Google Drive".to_string(),
                        description: Some(
                            "Use Google Drive as the single entrypoint for Drive, Docs, Sheets, and Slides work."
                                .to_string(),
                        ),
                    },
                    ToolSearchSourceInfo {
                        name: "Google Drive".to_string(),
                        description: None,
                    },
                    ToolSearchSourceInfo {
                        name: "docs".to_string(),
                        description: None,
                    },
                ],
                /*default_limit*/ 8,
            ),
            ToolSpec::ToolSearch {
                execution: "client".to_string(),
                description: "# Tool discovery\n\nSearches over deferred tool metadata with BM25 and exposes matching tools for the next model call.\n\nYou have access to tools from the following sources:\n- Google Drive: Use Google Drive as the single entrypoint for Drive, Docs, Sheets, and Slides work.\n- docs\nSome of the tools may not have been provided to you upfront, and you should use this tool (`tool_search`) to search for the required tools. For MCP tool discovery, always use `tool_search` instead of `list_mcp_resources` or `list_mcp_resource_templates`.".to_string(),
                parameters: JsonSchema::object(BTreeMap::from([
                        (
                            "limit".to_string(),
                            JsonSchema {
                                schema_type: Some(JsonSchemaType::Multiple(vec![
                                    JsonSchemaPrimitiveType::Number,
                                    JsonSchemaPrimitiveType::Null,
                                ])),
                                description: Some(
                                    "Maximum number of tools to return (defaults to 8)."
                                        .to_string(),
                                ),
                                ..Default::default()
                            },
                        ),
                    (
                        "query".to_string(),
                        JsonSchema::string(Some("Search query for deferred tools.".to_string()),),
                    ),
                    (
                        "source".to_string(),
                        JsonSchema {
                            schema_type: Some(JsonSchemaType::Multiple(vec![
                                JsonSchemaPrimitiveType::String,
                                JsonSchemaPrimitiveType::Null,
                            ])),
                            description: Some(
                                "Optional source filter. Available sources:\n- Google Drive: Use Google Drive as the single entrypoint for Drive, Docs, Sheets, and Slides work.\n- docs"
                                    .to_string(),
                            ),
                            ..Default::default()
                        },
                    ),
                ]), Some(vec!["query".to_string(), "limit".to_string(), "source".to_string()]), Some(false.into())),
            }
        );
    }
}
