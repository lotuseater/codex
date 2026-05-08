use crate::ResponsesApiTool;
use crate::ToolSpec;
use codex_tool_schema::AdditionalProperties;
use codex_tool_schema::JsonSchema;
use std::collections::BTreeMap;

pub const FILE_OUTLINE_TOOL_NAME: &str = "file_outline";
pub const SEARCH_TEXT_TOOL_NAME: &str = "search_text";

pub fn create_context_ops_tools() -> Vec<ToolSpec> {
    vec![
        create_tool(
            FILE_OUTLINE_TOOL_NAME,
            "Read a compact structural outline of a source file. Use before reading a large file when names and signatures may be enough.",
            object_schema(
                [
                    (
                        "path",
                        JsonSchema::string(Some("File path to outline.".to_string())),
                    ),
                    (
                        "workdir",
                        JsonSchema::string(Some(
                            "Base directory for relative paths. Defaults to the current working directory."
                                .to_string(),
                        )),
                    ),
                    (
                        "max_items",
                        JsonSchema::integer(Some(
                            "Maximum definition items to return. Defaults to 200 and is capped at 1000."
                                .to_string(),
                        )),
                    ),
                ],
                Some(vec!["path".to_string()]),
            ),
        ),
        create_tool(
            SEARCH_TEXT_TOOL_NAME,
            "Search text with capped grouped results. Use instead of broad raw rg output when finding likely files or examples.",
            object_schema(
                [
                    (
                        "pattern",
                        JsonSchema::string(Some("Text or regex pattern to search for.".to_string())),
                    ),
                    (
                        "workdir",
                        JsonSchema::string(Some(
                            "Base directory to search. Defaults to the current working directory."
                                .to_string(),
                        )),
                    ),
                    (
                        "glob",
                        JsonSchema::string(Some(
                            "Optional rg glob filter, for example '*.rs' or 'codex-rs/core/**'."
                                .to_string(),
                        )),
                    ),
                    (
                        "globs",
                        JsonSchema::array(
                            JsonSchema::string(/*description*/ None),
                            Some(
                                "Optional additional rg glob filters, preserving repeated --glob filters."
                                    .to_string(),
                            ),
                        ),
                    ),
                    (
                        "paths",
                        JsonSchema::array(
                            JsonSchema::string(/*description*/ None),
                            Some(
                                "Optional path filters searched relative to workdir, passed after the pattern."
                                    .to_string(),
                            ),
                        ),
                    ),
                    (
                        "max_files",
                        JsonSchema::integer(Some(
                            "Maximum matching files to return. Defaults to 50 and is capped at 500."
                                .to_string(),
                        )),
                    ),
                    (
                        "max_matches_per_file",
                        JsonSchema::integer(Some(
                            "Maximum matches per file. Defaults to 5 and is capped at 50."
                                .to_string(),
                        )),
                    ),
                ],
                Some(vec!["pattern".to_string()]),
            ),
        ),
    ]
}

fn create_tool(name: &str, description: &str, parameters: JsonSchema) -> ToolSpec {
    ToolSpec::Function(ResponsesApiTool {
        name: name.to_string(),
        description: description.to_string(),
        strict: false,
        defer_loading: None,
        parameters,
        output_schema: None,
    })
}

fn object_schema<const N: usize>(
    fields: [(&'static str, JsonSchema); N],
    required: Option<Vec<String>>,
) -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from_iter(
            fields
                .into_iter()
                .map(|(name, schema)| (name.to_string(), schema)),
        ),
        required,
        Some(AdditionalProperties::Boolean(false)),
    )
}

#[cfg(test)]
#[path = "context_ops_tests.rs"]
mod tests;
