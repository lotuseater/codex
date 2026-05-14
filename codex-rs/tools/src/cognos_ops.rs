use crate::ResponsesApiTool;
use crate::ToolSpec;
use codex_tool_schema::AdditionalProperties;
use codex_tool_schema::JsonSchema;
use serde_json::json;
use std::collections::BTreeMap;

pub const PROBLEM_MEMORY_LOOKUP_TOOL_NAME: &str = "problem_memory_lookup";
pub const CODE_RELATION_SCOUT_TOOL_NAME: &str = "code_relation_scout";
pub const AGENT_GRAPH_SCOUT_TOOL_NAME: &str = "agent_graph_scout";
pub const OPERATION_CACHE_STATS_TOOL_NAME: &str = "operation_cache_stats";
pub const EVIDENCE_FUSION_SUMMARY_TOOL_NAME: &str = "evidence_fusion_summary";
pub const MISSION_TRACE_EXPORT_TOOL_NAME: &str = "mission_trace_export";

pub fn create_cognos_ops_tools() -> Vec<ToolSpec> {
    vec![
        create_tool(
            PROBLEM_MEMORY_LOOKUP_TOOL_NAME,
            "Look up scoped project/problem memory hints for the current repo and prompt. Treat results as routing evidence, not authority.",
            object_schema(
                [
                    (
                        "project_root",
                        JsonSchema::string(Some(
                            "Repo root used to scope project/problem matches. Defaults to the current working directory."
                                .to_string(),
                        )),
                    ),
                    (
                        "prompt",
                        JsonSchema::string(Some(
                            "Task prompt used to rank memory hints.".to_string(),
                        )),
                    ),
                    (
                        "max_matches",
                        JsonSchema::integer(Some(
                            "Maximum matches to return. Defaults to 3.".to_string(),
                        )),
                    ),
                ],
                None,
            ),
        ),
        create_tool(
            CODE_RELATION_SCOUT_TOOL_NAME,
            "Classify candidate repo files by role and relation edges for graph-aware navigation.",
            object_schema(
                [
                    (
                        "project_root",
                        JsonSchema::string(Some(
                            "Repo root to inspect. Defaults to the current working directory."
                                .to_string(),
                        )),
                    ),
                    (
                        "prompt",
                        JsonSchema::string(Some(
                            "Task prompt used to rank candidate files.".to_string(),
                        )),
                    ),
                    (
                        "max_paths",
                        JsonSchema::integer(Some(
                            "Maximum paths to return. Defaults to 16 and is capped at 64."
                                .to_string(),
                        )),
                    ),
                ],
                None,
            ),
        ),
        create_tool(
            AGENT_GRAPH_SCOUT_TOOL_NAME,
            "Summarize live/persisted spawned-agent graph state and recommend reuse, follow-up, compact, restart, or fresh spawn.",
            object_schema(
                [
                    (
                        "status",
                        JsonSchema::string_enum(
                            vec![json!("open"), json!("closed"), json!("all")],
                            Some("Persisted edge status filter. Defaults to open.".to_string()),
                        ),
                    ),
                    (
                        "max_agents",
                        JsonSchema::integer(Some(
                            "Maximum agents to include. Defaults to 20 and is capped at 100."
                                .to_string(),
                        )),
                    ),
                ],
                None,
            ),
        ),
        create_tool(
            OPERATION_CACHE_STATS_TOOL_NAME,
            "Report operation-cache availability, cacheability rules, environment configuration, and common miss reasons.",
            object_schema(
                [(
                    "project_root",
                    JsonSchema::string(Some(
                        "Repo root/cwd used to derive the cache scope. Defaults to the current working directory."
                            .to_string(),
                    )),
                )],
                None,
            ),
        ),
        create_tool(
            EVIDENCE_FUSION_SUMMARY_TOOL_NAME,
            "Classify current task evidence as Accept, Modify, or Stop from tests, diff status, review findings, blockers, and caveats.",
            object_schema(
                [
                    (
                        "tests",
                        JsonSchema::array(
                            JsonSchema::string(/*description*/ None),
                            Some("Test/build/verification outcomes.".to_string()),
                        ),
                    ),
                    (
                        "diff_summary",
                        JsonSchema::string(Some("Short summary of changed behavior.".to_string())),
                    ),
                    (
                        "review_findings",
                        JsonSchema::array(
                            JsonSchema::string(/*description*/ None),
                            Some("Known review findings or defects.".to_string()),
                        ),
                    ),
                    (
                        "blockers",
                        JsonSchema::array(
                            JsonSchema::string(/*description*/ None),
                            Some("External or unresolved blockers.".to_string()),
                        ),
                    ),
                    (
                        "unresolved_caveats",
                        JsonSchema::array(
                            JsonSchema::string(/*description*/ None),
                            Some("Remaining caveats that may be fixable.".to_string()),
                        ),
                    ),
                ],
                None,
            ),
        ),
        create_tool(
            MISSION_TRACE_EXPORT_TOOL_NAME,
            "Normalize task evidence into a read-only mission trace record for memory, eval, and bounded-repair review.",
            object_schema(
                [
                    (
                        "task_prompt",
                        JsonSchema::string(Some(
                            "Task or objective being traced.".to_string(),
                        )),
                    ),
                    (
                        "tests",
                        JsonSchema::array(
                            JsonSchema::string(/*description*/ None),
                            Some("Test/build/verification outcomes.".to_string()),
                        ),
                    ),
                    (
                        "diff_summary",
                        JsonSchema::string(Some("Short summary of changed behavior.".to_string())),
                    ),
                    (
                        "review_findings",
                        JsonSchema::array(
                            JsonSchema::string(/*description*/ None),
                            Some("Known review findings or defects.".to_string()),
                        ),
                    ),
                    (
                        "blockers",
                        JsonSchema::array(
                            JsonSchema::string(/*description*/ None),
                            Some("External or unresolved blockers.".to_string()),
                        ),
                    ),
                    (
                        "unresolved_caveats",
                        JsonSchema::array(
                            JsonSchema::string(/*description*/ None),
                            Some("Remaining caveats that may be fixable.".to_string()),
                        ),
                    ),
                    (
                        "agent_notes",
                        JsonSchema::array(
                            JsonSchema::string(/*description*/ None),
                            Some("Agent reuse, review, or orchestration notes.".to_string()),
                        ),
                    ),
                    (
                        "tool_notes",
                        JsonSchema::array(
                            JsonSchema::string(/*description*/ None),
                            Some("Tool, operation, or prompt-injection notes.".to_string()),
                        ),
                    ),
                    (
                        "cache_notes",
                        JsonSchema::array(
                            JsonSchema::string(/*description*/ None),
                            Some("Cache hit, miss, or invalidation notes.".to_string()),
                        ),
                    ),
                ],
                None,
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
        output_schema: Some(json!({
            "type": "object",
            "additionalProperties": true
        })),
    })
}

fn object_schema(
    properties: impl IntoIterator<Item = (&'static str, JsonSchema)>,
    required: Option<Vec<String>>,
) -> JsonSchema {
    JsonSchema::object(
        BTreeMap::from_iter(
            properties
                .into_iter()
                .map(|(name, schema)| (name.to_string(), schema)),
        ),
        required,
        Some(AdditionalProperties::Boolean(false)),
    )
}
