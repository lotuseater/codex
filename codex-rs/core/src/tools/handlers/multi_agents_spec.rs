use codex_protocol::openai_models::ModelPreset;
use codex_tool_registry_api::ToolSpec;

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
    codex_tool_registry_api::create_spawn_agent_tool_v2(options.as_tool_options())
}

pub fn create_wait_agent_tool_v1(options: WaitAgentTimeoutOptions) -> ToolSpec {
    codex_tool_registry_api::create_wait_agent_tool_v1(options.into_tool_options())
}

pub fn create_wait_agent_tool_v2(options: WaitAgentTimeoutOptions) -> ToolSpec {
    codex_tool_registry_api::create_wait_agent_tool_v2(options.into_tool_options())
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
