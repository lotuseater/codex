mod execution;
pub(crate) mod file_outline;
pub(crate) mod search_text;
pub(crate) mod workflow_batch;

use codex_tool_execution_api::ToolName;
use codex_tool_registry_api::FILE_OUTLINE_TOOL_NAME;
use codex_tool_registry_api::SEARCH_TEXT_TOOL_NAME;
use codex_tool_registry_api::ToolSpec;
use codex_tool_registry_api::WORKFLOW_BATCH_TOOL_NAME;

use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::registry::ToolExecutor;
use crate::tools::registry::ToolHandler;

pub struct ContextOpsHandler {
    spec: ToolSpec,
}

impl ContextOpsHandler {
    pub fn new(spec: ToolSpec) -> Self {
        Self { spec }
    }

    fn arguments_from_payload<'a>(&self, payload: &'a ToolPayload) -> Option<&'a str> {
        let ToolPayload::Function { arguments } = payload else {
            return None;
        };
        Some(arguments)
    }
}

impl ToolExecutor<ToolInvocation> for ContextOpsHandler {
    type Output = FunctionToolOutput;

    fn tool_name(&self) -> ToolName {
        ToolName::plain(self.spec.name())
    }

    fn spec(&self) -> Option<ToolSpec> {
        Some(self.spec.clone())
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let arguments = self
            .arguments_from_payload(&invocation.payload)
            .map(str::to_string)
            .ok_or_else(|| {
                FunctionCallError::RespondToModel(
                    "context ops handler received unsupported payload".to_string(),
                )
            })?;

        match invocation.tool_name.name.as_str() {
            FILE_OUTLINE_TOOL_NAME => file_outline::handle(invocation, arguments.as_str()).await,
            SEARCH_TEXT_TOOL_NAME => search_text::handle(invocation, arguments.as_str()).await,
            WORKFLOW_BATCH_TOOL_NAME => {
                workflow_batch::handle(invocation, arguments.as_str()).await
            }
            other => Err(FunctionCallError::RespondToModel(format!(
                "unknown context ops tool: {other}"
            ))),
        }
    }
}

impl ToolHandler for ContextOpsHandler {
    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        self.arguments_from_payload(payload).is_some()
    }
}
