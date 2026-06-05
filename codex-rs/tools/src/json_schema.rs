//! Re-export of `codex_tool_schema` so `codex_tools::JsonSchema`
//! and `codex_tool_schema::JsonSchema` resolve to the same type.
//!
//! The fork promoted JSON Schema definitions into a standalone
//! `codex-tool-schema` crate. Files that still reach for these
//! types through the `codex_tools` path resolve to the schema crate.

pub use codex_tool_schema::AdditionalProperties;
pub use codex_tool_schema::JsonSchema;
pub use codex_tool_schema::JsonSchemaPrimitiveType;
pub use codex_tool_schema::JsonSchemaType;
pub use codex_tool_schema::parse_tool_input_schema;
pub use codex_tool_schema::parse_tool_input_schema_without_compaction;

// fork-local: tests for these schema types live in the `codex-tool-schema`
// crate (`tool-schema/src/json_schema_tests.rs`) after the schema split;
// the upstream-side `mod tests;` referenced a `tools/src/json_schema_tests.rs`
// file that the fork removed, so it is intentionally not re-declared here.
