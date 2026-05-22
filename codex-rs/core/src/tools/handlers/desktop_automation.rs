use codex_desktop_automation::execute_tool;
use codex_desktop_automation::text_output_value;
use codex_protocol::models::DEFAULT_IMAGE_DETAIL;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::openai_models::InputModality;
use codex_tool_execution_api::ToolName;
use codex_tool_registry_api::ToolSpec;
use serde_json::Value;

use codex_tool_execution_api::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::hook_names::HookToolName;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::PostToolUsePayload;
use crate::tools::registry::PreToolUsePayload;
use crate::tools::registry::ToolExecutor;

pub struct DesktopAutomationHandler {
    spec: ToolSpec,
}

impl DesktopAutomationHandler {
    pub fn new(spec: ToolSpec) -> Self {
        Self { spec }
    }
}

impl ToolExecutor<ToolInvocation> for DesktopAutomationHandler {
    type Output = FunctionToolOutput;

    fn tool_name(&self) -> ToolName {
        ToolName::plain(self.spec.name())
    }

    fn spec(&self) -> Option<ToolSpec> {
        Some(self.spec.clone())
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let arguments = match &invocation.payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "desktop automation handler received unsupported payload".to_string(),
                ));
            }
        };
        let input: Value = parse_arguments(arguments)?;
        let cwd = invocation
            .turn
            .environments
            .primary()
            .ok_or_else(|| {
                FunctionCallError::RespondToModel(
                    "desktop automation requires a selected turn environment".to_string(),
                )
            })?
            .cwd
            .as_path();
        let result = execute_tool(invocation.tool_name.name.as_str(), input, cwd)
            .await
            .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;

        let text_output = text_output_value(&result.output);
        let text = serde_json::to_string_pretty(&text_output)
            .unwrap_or_else(|err| format!("failed to serialize desktop automation output: {err}"));
        let mut content = vec![FunctionCallOutputContentItem::InputText { text }];
        if let Some(image_url) = result.image_url.clone()
            && invocation
                .turn
                .model_info
                .input_modalities
                .contains(&InputModality::Image)
        {
            content.push(FunctionCallOutputContentItem::InputImage {
                image_url,
                detail: Some(DEFAULT_IMAGE_DETAIL),
            });
        }

        let mut output = FunctionToolOutput::from_content(content, Some(result.ok));
        output.post_tool_use_response = Some(text_output);
        Ok(output)
    }
}

impl CoreToolRuntime for DesktopAutomationHandler {
    fn pre_tool_use_payload(&self, invocation: &ToolInvocation) -> Option<PreToolUsePayload> {
        Some(PreToolUsePayload {
            tool_name: HookToolName::new(invocation.tool_name.to_string()),
            tool_input: function_arguments(&invocation.payload),
        })
    }

    fn post_tool_use_payload(
        &self,
        invocation: &ToolInvocation,
        result: &dyn ToolOutput,
    ) -> Option<PostToolUsePayload> {
        Some(PostToolUsePayload {
            tool_name: HookToolName::new(invocation.tool_name.to_string()),
            tool_use_id: invocation.call_id.clone(),
            tool_input: function_arguments(&invocation.payload),
            tool_response: result
                .post_tool_use_response(&invocation.call_id, &invocation.payload)
                .unwrap_or(Value::Null),
        })
    }
}

fn function_arguments(payload: &ToolPayload) -> Value {
    match payload {
        ToolPayload::Function { arguments } => parse_arguments(arguments).unwrap_or(Value::Null),
        _ => Value::Null,
    }
}
