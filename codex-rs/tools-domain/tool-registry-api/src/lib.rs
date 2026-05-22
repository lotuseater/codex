//! Tool registry abstractions.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

mod agent_tool;
mod code_mode;
mod cognos_ops;
mod context_ops;
mod desktop_automation;
mod first_moves;
mod mcp_tool;
mod plan_tool;
mod repo_context_scout;
mod request_plugin_install;
mod tool_discovery;

pub use agent_tool::SpawnAgentToolOptions;
pub use agent_tool::WaitAgentTimeoutOptions;
pub use agent_tool::create_close_agent_tool_v1;
pub use agent_tool::create_close_agent_tool_v2;
pub use agent_tool::create_compact_agent_tool;
pub use agent_tool::create_followup_task_tool;
pub use agent_tool::create_list_agents_tool;
pub use agent_tool::create_restart_agent_tool;
pub use agent_tool::create_resume_agent_tool;
pub use agent_tool::create_resume_agent_tool_v2;
pub use agent_tool::create_send_input_tool_v1;
pub use agent_tool::create_send_message_tool;
pub use agent_tool::create_spawn_agent_tool_v1;
pub use agent_tool::create_spawn_agent_tool_v2;
pub use agent_tool::create_wait_agent_tool_v1;
pub use agent_tool::create_wait_agent_tool_v2;
pub use code_mode::augment_tool_spec_for_code_mode;
pub use code_mode::code_mode_name_for_tool_name;
pub use code_mode::collect_code_mode_exec_prompt_tool_definitions;
pub use code_mode::collect_code_mode_tool_definitions;
pub use codex_tool_schema::JsonSchema;
pub use codex_tool_schema::JsonSchemaPrimitiveType;
pub use codex_tool_schema::JsonSchemaType;
pub use codex_tool_schema::parse_tool_input_schema;
pub use cognos_ops::AGENT_GRAPH_SCOUT_TOOL_NAME;
pub use cognos_ops::CODE_RELATION_SCOUT_TOOL_NAME;
pub use cognos_ops::EVIDENCE_FUSION_SUMMARY_TOOL_NAME;
pub use cognos_ops::MISSION_TRACE_EXPORT_TOOL_NAME;
pub use cognos_ops::OPERATION_CACHE_STATS_TOOL_NAME;
pub use cognos_ops::PROBLEM_MEMORY_LOOKUP_TOOL_NAME;
pub use cognos_ops::create_cognos_ops_tools;
pub use context_ops::FILE_OUTLINE_TOOL_NAME;
pub use context_ops::SEARCH_TEXT_TOOL_NAME;
pub use context_ops::WORKFLOW_BATCH_TOOL_NAME;
pub use context_ops::create_context_ops_tools;
pub use context_ops::create_workflow_batch_tool;
pub use desktop_automation::create_desktop_automation_tools;
pub use first_moves::FIRST_MOVES_PREDICT_TOOL_NAME;
pub use first_moves::FIRST_MOVES_STATS_TOOL_NAME;
pub use first_moves::create_first_moves_tools;
pub use mcp_tool::mcp_call_tool_result_output_schema;
pub use plan_tool::create_update_plan_tool_with_delegation_policy;
pub use repo_context_scout::REPO_CONTEXT_SCOUT_TOOL_NAME;
pub use repo_context_scout::create_repo_context_scout_tool;
pub use request_plugin_install::REQUEST_PLUGIN_INSTALL_APPROVAL_KIND_VALUE;
pub use request_plugin_install::REQUEST_PLUGIN_INSTALL_PERSIST_ALWAYS_VALUE;
pub use request_plugin_install::REQUEST_PLUGIN_INSTALL_PERSIST_KEY;
pub use request_plugin_install::RequestPluginInstallArgs;
pub use request_plugin_install::RequestPluginInstallMeta;
pub use request_plugin_install::RequestPluginInstallResult;
pub use request_plugin_install::all_requested_connectors_picked_up;
pub use request_plugin_install::build_request_plugin_install_meta;
pub use request_plugin_install::verified_connector_install_completed;
pub use tool_discovery::LIST_AVAILABLE_PLUGINS_TO_INSTALL_TOOL_NAME;
pub use tool_discovery::ListAvailablePluginsToInstallResult;
pub use tool_discovery::REQUEST_PLUGIN_INSTALL_TOOL_NAME;
pub use tool_discovery::RequestPluginInstallEntry;
pub use tool_discovery::TOOL_SEARCH_DEFAULT_LIMIT;
pub use tool_discovery::TOOL_SEARCH_TOOL_NAME;
pub use tool_discovery::ToolSearchSourceInfo;
pub use tool_discovery::coalesce_loadable_tool_specs;
pub use tool_discovery::collect_request_plugin_install_entries;
pub use tool_discovery::create_request_plugin_install_tool;
pub use tool_discovery::create_tool_search_tool;
pub use tool_discovery::filter_request_plugin_install_discoverable_tools_for_client;
pub use tool_discovery::loadable_namespace_tools;
pub use tool_discovery::loadable_tool_spec_name;

use codex_app_catalog_types::AppInfo;
use codex_protocol::config_types::WebSearchContextSize;
use codex_protocol::config_types::WebSearchFilters as ConfigWebSearchFilters;
use codex_protocol::config_types::WebSearchUserLocation as ConfigWebSearchUserLocation;
use codex_protocol::config_types::WebSearchUserLocationType;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

/// Model-visible name for the goal creation tool.
pub const CREATE_GOAL_TOOL_NAME: &str = "create_goal";
/// Model-visible name for the goal lookup tool.
pub const GET_GOAL_TOOL_NAME: &str = "get_goal";
/// Model-visible name for the goal update tool.
pub const UPDATE_GOAL_TOOL_NAME: &str = "update_goal";

/// Kind of installable tool surfaced through discovery.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiscoverableToolType {
    Connector,
    Plugin,
}

impl DiscoverableToolType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Connector => "connector",
            Self::Plugin => "plugin",
        }
    }
}

/// Action requested for a discoverable tool.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiscoverableToolAction {
    Install,
    Enable,
}

/// Installable connector or plugin surfaced through tool discovery.
#[derive(Clone, Debug, PartialEq)]
pub enum DiscoverableTool {
    Connector(Box<AppInfo>),
    Plugin(Box<DiscoverablePluginInfo>),
}

impl DiscoverableTool {
    pub fn tool_type(&self) -> DiscoverableToolType {
        match self {
            Self::Connector(_) => DiscoverableToolType::Connector,
            Self::Plugin(_) => DiscoverableToolType::Plugin,
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Self::Connector(connector) => connector.id.as_str(),
            Self::Plugin(plugin) => plugin.id.as_str(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Connector(connector) => connector.name.as_str(),
            Self::Plugin(plugin) => plugin.name.as_str(),
        }
    }

    pub fn install_url(&self) -> Option<&str> {
        match self {
            Self::Connector(connector) => connector.install_url.as_deref(),
            Self::Plugin(_) => None,
        }
    }
}

impl From<AppInfo> for DiscoverableTool {
    fn from(value: AppInfo) -> Self {
        Self::Connector(Box::new(value))
    }
}

impl From<DiscoverablePluginInfo> for DiscoverableTool {
    fn from(value: DiscoverablePluginInfo) -> Self {
        Self::Plugin(Box::new(value))
    }
}

/// Plugin metadata surfaced through tool discovery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoverablePluginInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub has_skills: bool,
    pub mcp_server_names: Vec<String>,
    pub app_connector_ids: Vec<String>,
}

/// Protocol-neutral description of an available tool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolDescriptor {
    /// Stable tool name used by callers.
    pub name: String,
    /// Human-readable description of the tool contract.
    pub description: String,
}

/// Exposes the tool set available to a caller.
///
/// Implementations should describe registered tools without coupling the
/// caller to the concrete handler, execution engine, or protocol serializer.
pub trait ToolRegistry {
    /// Returns the tool descriptors available for registration.
    fn tool_descriptors(&self) -> Vec<ToolDescriptor>;
}

/// Configuration boundary used to decide how MCP tools are exposed.
///
/// Implementations should provide only the policy values needed for MCP tool
/// exposure, keeping callers decoupled from concrete tool registry
/// configuration types.
pub trait McpToolExposureConfig {
    /// Returns whether `tool_search` is available for deferred MCP tools.
    fn search_tool_enabled(&self) -> bool;
}

/// Controls where a registered tool is exposed to the model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolExposure {
    /// Include this tool in the initial model-visible tool list.
    ///
    /// When code mode is enabled, this tool is also available as a nested
    /// code-mode tool.
    Direct,

    /// Register this tool for later discovery, but omit it from the initial
    /// model-visible tool list.
    Deferred,

    /// Include this tool in the initial model-visible tool list only.
    ///
    /// In code-mode-only sessions, this keeps the tool callable as a normal
    /// model tool while excluding it from the nested code-mode tool surface.
    DirectModelOnly,
}

impl ToolExposure {
    pub fn is_direct(self) -> bool {
        matches!(self, Self::Direct | Self::DirectModelOnly)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FreeformTool {
    pub name: String,
    pub description: String,
    pub format: FreeformToolFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FreeformToolFormat {
    pub r#type: String,
    pub syntax: String,
    pub definition: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ResponsesApiTool {
    pub name: String,
    pub description: String,
    /// TODO: Validation. When strict is set to true, the JSON schema,
    /// `required` and `additional_properties` must be present. All fields in
    /// `properties` must be present in `required`.
    pub strict: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    pub parameters: JsonSchema,
    #[serde(skip)]
    pub output_schema: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "type")]
#[allow(clippy::large_enum_variant)]
pub enum LoadableToolSpec {
    #[allow(dead_code)]
    #[serde(rename = "function")]
    Function(ResponsesApiTool),
    #[serde(rename = "namespace")]
    Namespace(ResponsesApiNamespace),
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ResponsesApiNamespace {
    pub name: String,
    pub description: String,
    pub tools: Vec<ResponsesApiNamespaceTool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfiguredToolSpec {
    pub spec: ToolSpec,
    pub supports_parallel_tool_calls: bool,
}

impl ConfiguredToolSpec {
    pub fn new(spec: ToolSpec, supports_parallel_tool_calls: bool) -> Self {
        Self {
            spec,
            supports_parallel_tool_calls,
        }
    }

    pub fn name(&self) -> &str {
        self.spec.name()
    }
}

pub fn default_namespace_description(namespace_name: &str) -> String {
    format!("Tools in the {namespace_name} namespace.")
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "type")]
pub enum ResponsesApiNamespaceTool {
    #[serde(rename = "function")]
    Function(ResponsesApiTool),
}

/// Serializable tool spec accepted by the Responses API.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "type")]
pub enum ToolSpec {
    #[serde(rename = "function")]
    Function(ResponsesApiTool),
    #[serde(rename = "namespace")]
    Namespace(ResponsesApiNamespace),
    #[serde(rename = "tool_search")]
    ToolSearch {
        execution: String,
        description: String,
        parameters: JsonSchema,
    },
    #[serde(rename = "local_shell")]
    LocalShell {},
    #[serde(rename = "image_generation")]
    ImageGeneration { output_format: String },
    /// The Responses API supports "web_search", but the documented type
    /// https://platform.openai.com/docs/guides/tools-web-search?api-mode=responses#:~:text=%7B%20type%3A%20%22web_search%22%20%7D%2C
    /// The `external_web_access` field determines whether the web search is over
    /// cached or live content.
    /// https://platform.openai.com/docs/guides/tools-web-search#live-internet-access
    #[serde(rename = "web_search")]
    WebSearch {
        #[serde(skip_serializing_if = "Option::is_none")]
        external_web_access: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        filters: Option<ResponsesApiWebSearchFilters>,
        #[serde(skip_serializing_if = "Option::is_none")]
        user_location: Option<ResponsesApiWebSearchUserLocation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        search_context_size: Option<WebSearchContextSize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        search_content_types: Option<Vec<String>>,
    },
    #[serde(rename = "custom")]
    Freeform(FreeformTool),
}

impl ToolSpec {
    pub fn name(&self) -> &str {
        match self {
            ToolSpec::Function(tool) => tool.name.as_str(),
            ToolSpec::Namespace(namespace) => namespace.name.as_str(),
            ToolSpec::ToolSearch { .. } => "tool_search",
            ToolSpec::LocalShell {} => "local_shell",
            ToolSpec::ImageGeneration { .. } => "image_generation",
            ToolSpec::WebSearch { .. } => "web_search",
            ToolSpec::Freeform(tool) => tool.name.as_str(),
        }
    }
}

/// Returns JSON values that are compatible with Function Calling in the
/// Responses API:
/// https://platform.openai.com/docs/guides/function-calling?api-mode=responses
pub fn create_tools_json_for_responses_api(
    tools: &[ToolSpec],
) -> Result<Vec<Value>, serde_json::Error> {
    let mut tools_json = Vec::new();

    for tool in tools {
        let json = serde_json::to_value(tool)?;
        tools_json.push(json);
    }

    Ok(tools_json)
}

impl From<LoadableToolSpec> for ToolSpec {
    fn from(value: LoadableToolSpec) -> Self {
        match value {
            LoadableToolSpec::Function(tool) => ToolSpec::Function(tool),
            LoadableToolSpec::Namespace(namespace) => ToolSpec::Namespace(namespace),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ResponsesApiWebSearchFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
}

impl From<ConfigWebSearchFilters> for ResponsesApiWebSearchFilters {
    fn from(filters: ConfigWebSearchFilters) -> Self {
        Self {
            allowed_domains: filters.allowed_domains,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ResponsesApiWebSearchUserLocation {
    #[serde(rename = "type")]
    pub r#type: WebSearchUserLocationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

impl From<ConfigWebSearchUserLocation> for ResponsesApiWebSearchUserLocation {
    fn from(user_location: ConfigWebSearchUserLocation) -> Self {
        Self {
            r#type: user_location.r#type,
            country: user_location.country,
            region: user_location.region,
            city: user_location.city,
            timezone: user_location.timezone,
        }
    }
}
