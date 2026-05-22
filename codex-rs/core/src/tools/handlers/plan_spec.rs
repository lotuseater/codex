use codex_agent_policy::MAIN_AGENT_PLAN_DELEGATION_PROMPT;
use codex_tool_registry_api::ToolSpec;
use codex_tool_registry_api::create_update_plan_tool_with_delegation_policy;

pub(crate) fn create_update_plan_tool() -> ToolSpec {
    create_update_plan_tool_with_delegation_policy(MAIN_AGENT_PLAN_DELEGATION_PROMPT)
}
