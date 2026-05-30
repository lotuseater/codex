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

#[cfg(test)]
#[path = "json_schema_tests.rs"]
mod tests;
