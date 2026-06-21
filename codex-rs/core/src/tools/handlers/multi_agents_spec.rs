use codex_protocol::openai_models::ModelPreset;
use codex_tool_registry_api::ToolSpec;
// fork-local: imports required by the fork-local `create_interrupt_agent_tool_v2` below,
// which upstream defines inline in this module but the fork serves from a facade. These
// types are re-exported by `codex_tools` from `codex_tool_registry_api`, so they unify
// with the facade's `ToolSpec`.
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeMap;

pub const MULTI_AGENT_V1_NAMESPACE: &str = "multi_agent_v1";

pub use codex_tool_registry_api::create_close_agent_tool_v1;
pub use codex_tool_registry_api::create_close_agent_tool_v2;
pub use codex_tool_registry_api::create_compact_agent_tool;
pub use codex_tool_registry_api::create_followup_task_tool;
pub use codex_tool_registry_api::create_list_agents_tool;
pub use codex_tool_registry_api::create_restart_agent_tool;
pub use codex_tool_registry_api::create_resume_agent_tool;
pub use codex_tool_registry_api::create_resume_agent_tool_v2;
pub use codex_tool_registry_api::create_send_input_tool_v1;
pub use codex_tool_registry_api::create_send_message_tool;

#[derive(Debug, Clone, Default)]
pub struct SpawnAgentToolOptions {
    pub available_models: Vec<ModelPreset>,
    pub agent_type_description: String,
    pub hide_agent_type_model_reasoning: bool,
    pub include_usage_hint: bool,
    pub usage_hint_text: Option<String>,
    pub max_concurrent_threads_per_session: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitAgentTimeoutOptions {
    pub default_timeout_ms: i64,
    pub min_timeout_ms: i64,
    pub max_timeout_ms: i64,
}

impl Default for WaitAgentTimeoutOptions {
    fn default() -> Self {
        Self {
            default_timeout_ms: super::multi_agents_common::DEFAULT_WAIT_TIMEOUT_MS,
            min_timeout_ms: super::multi_agents_common::MIN_WAIT_TIMEOUT_MS,
            max_timeout_ms: super::multi_agents_common::MAX_WAIT_TIMEOUT_MS,
        }
    }
}

pub fn create_spawn_agent_tool_v1(options: SpawnAgentToolOptions) -> ToolSpec {
    codex_tool_registry_api::create_spawn_agent_tool_v1(options.as_tool_options())
}

pub fn create_spawn_agent_tool_v2(options: SpawnAgentToolOptions) -> ToolSpec {
    // fork-local: the v2 spawn-agent spec body lives in `codex_tool_registry_api`
    // (agent_tool.rs); the helpers upstream inlines here were moved there.
    codex_tool_registry_api::create_spawn_agent_tool_v2(options.as_tool_options())
}

pub fn create_wait_agent_tool_v1(options: WaitAgentTimeoutOptions) -> ToolSpec {
    codex_tool_registry_api::create_wait_agent_tool_v1(options.into_tool_options())
}

pub fn create_wait_agent_tool_v2(options: WaitAgentTimeoutOptions) -> ToolSpec {
    codex_tool_registry_api::create_wait_agent_tool_v2(options.into_tool_options())
}

// fork-local: upstream added `interrupt_agent` (and its new `interrupt_agent` v2 handler
// imports `multi_agents_spec::create_interrupt_agent_tool_v2`). The fork serves every other
// multi-agent tool from `codex_tool_registry_api`, but that builder was not migrated there,
// so keep a working copy here adapted to the fork's facade.
pub fn create_interrupt_agent_tool_v2() -> ToolSpec {
    let properties = BTreeMap::from([(
        "target".to_string(),
        JsonSchema::string(Some(
            "Agent id or canonical task name to interrupt (from spawn_agent).".to_string(),
        )),
    )]);

    ToolSpec::Function(ResponsesApiTool {
        name: "interrupt_agent".to_string(),
        description: "Interrupt an agent's current turn, if any, and return its previous status. The agent remains available for messages and follow-up tasks.".to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(properties, Some(vec!["target".to_string()]), Some(false.into())),
        output_schema: Some(agent_previous_status_output_schema(
            "The agent status observed before the interrupt request was handled.",
        )),
    })
}

// fork-local: helper schemas backing `create_interrupt_agent_tool_v2`.
fn agent_status_output_schema() -> Value {
    json!({
        "oneOf": [
            {
                "type": "string",
                "enum": ["pending_init", "running", "interrupted", "shutdown", "not_found"]
            },
            {
                "type": "object",
                "properties": {
                    "completed": {
                        "type": ["string", "null"]
                    }
                },
                "required": ["completed"],
                "additionalProperties": false
            },
            {
                "type": "object",
                "properties": {
                    "errored": {
                        "type": "string"
                    }
                },
                "required": ["errored"],
                "additionalProperties": false
            }
        ]
    })
}

// fork-local: helper schema backing `create_interrupt_agent_tool_v2`.
fn agent_previous_status_output_schema(previous_status_description: &str) -> Value {
    json!({
        "type": "object",
        "properties": {
            "previous_status": {
                "description": previous_status_description,
                "allOf": [agent_status_output_schema()]
            }
        },
        "required": ["previous_status"],
        "additionalProperties": false
    })
}

impl SpawnAgentToolOptions {
    fn as_tool_options(&self) -> codex_tool_registry_api::SpawnAgentToolOptions<'_> {
        codex_tool_registry_api::SpawnAgentToolOptions {
            available_models: &self.available_models,
            agent_type_description: self.agent_type_description.clone(),
            hide_agent_type_model_reasoning: self.hide_agent_type_model_reasoning,
            include_usage_hint: self.include_usage_hint,
            usage_hint_text: self.usage_hint_text.clone(),
            max_concurrent_threads_per_session: self.max_concurrent_threads_per_session,
        }
    }
}

impl WaitAgentTimeoutOptions {
    fn into_tool_options(self) -> codex_tool_registry_api::WaitAgentTimeoutOptions {
        codex_tool_registry_api::WaitAgentTimeoutOptions {
            default_timeout_ms: self.default_timeout_ms,
            min_timeout_ms: self.min_timeout_ms,
            max_timeout_ms: self.max_timeout_ms,
        }
    }
}

#[cfg(test)]
#[path = "multi_agents_spec_tests.rs"]
mod tests;
