//! MCP elicitation request and schema abstractions.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as JsonValue;

/// Request payload for an MCP server elicitation observed by Codex.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpServerElicitationRequestParams {
    pub thread_id: String,
    /// Active Codex turn when this elicitation was observed, if one was known.
    pub turn_id: Option<String>,
    pub server_name: String,
    #[serde(flatten)]
    pub request: McpServerElicitationRequest,
}

/// MCP server elicitation request modes supported by Codex clients.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum McpServerElicitationRequest {
    #[serde(rename_all = "camelCase")]
    Form {
        #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
        meta: Option<JsonValue>,
        message: String,
        requested_schema: McpElicitationSchema,
    },
    #[serde(rename_all = "camelCase")]
    Url {
        #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
        meta: Option<JsonValue>,
        message: String,
        url: String,
        elicitation_id: String,
    },
}

/// JSON-schema subset accepted for MCP form elicitation.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpElicitationSchema {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema_uri: Option<String>,
    #[serde(rename = "type")]
    pub type_: McpElicitationObjectType,
    pub properties: BTreeMap<String, McpElicitationPrimitiveSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum McpElicitationObjectType {
    Object,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum McpElicitationPrimitiveSchema {
    Enum(McpElicitationEnumSchema),
    String(McpElicitationStringSchema),
    Number(McpElicitationNumberSchema),
    Boolean(McpElicitationBooleanSchema),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpElicitationStringSchema {
    #[serde(rename = "type")]
    pub type_: McpElicitationStringType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<McpElicitationStringFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum McpElicitationStringType {
    String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum McpElicitationStringFormat {
    Email,
    Uri,
    Date,
    DateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpElicitationNumberSchema {
    #[serde(rename = "type")]
    pub type_: McpElicitationNumberType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum McpElicitationNumberType {
    Number,
    Integer,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpElicitationBooleanSchema {
    #[serde(rename = "type")]
    pub type_: McpElicitationBooleanType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum McpElicitationBooleanType {
    Boolean,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum McpElicitationEnumSchema {
    SingleSelect(McpElicitationSingleSelectEnumSchema),
    MultiSelect(McpElicitationMultiSelectEnumSchema),
    Legacy(McpElicitationLegacyTitledEnumSchema),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpElicitationLegacyTitledEnumSchema {
    #[serde(rename = "type")]
    pub type_: McpElicitationStringType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "enum")]
    pub enum_: Vec<String>,
    #[serde(rename = "enumNames", skip_serializing_if = "Option::is_none")]
    pub enum_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum McpElicitationSingleSelectEnumSchema {
    Untitled(McpElicitationUntitledSingleSelectEnumSchema),
    Titled(McpElicitationTitledSingleSelectEnumSchema),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpElicitationUntitledSingleSelectEnumSchema {
    #[serde(rename = "type")]
    pub type_: McpElicitationStringType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "enum")]
    pub enum_: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpElicitationTitledSingleSelectEnumSchema {
    #[serde(rename = "type")]
    pub type_: McpElicitationStringType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "oneOf")]
    pub one_of: Vec<McpElicitationConstOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum McpElicitationMultiSelectEnumSchema {
    Untitled(McpElicitationUntitledMultiSelectEnumSchema),
    Titled(McpElicitationTitledMultiSelectEnumSchema),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpElicitationUntitledMultiSelectEnumSchema {
    #[serde(rename = "type")]
    pub type_: McpElicitationArrayType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u64>,
    pub items: McpElicitationUntitledEnumItems,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpElicitationTitledMultiSelectEnumSchema {
    #[serde(rename = "type")]
    pub type_: McpElicitationArrayType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u64>,
    pub items: McpElicitationTitledEnumItems,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum McpElicitationArrayType {
    Array,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpElicitationUntitledEnumItems {
    #[serde(rename = "type")]
    pub type_: McpElicitationStringType,
    #[serde(rename = "enum")]
    pub enum_: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpElicitationTitledEnumItems {
    #[serde(rename = "anyOf", alias = "oneOf")]
    pub any_of: Vec<McpElicitationConstOption>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpElicitationConstOption {
    #[serde(rename = "const")]
    pub const_: String,
    pub title: String,
}
