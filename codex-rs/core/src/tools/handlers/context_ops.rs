mod execution;
pub(crate) mod file_outline;
pub(crate) mod git_worktree_summary;
pub(crate) mod search_text;

use codex_tools::FILE_OUTLINE_TOOL_NAME;
use codex_tools::GIT_WORKTREE_SUMMARY_TOOL_NAME;
use codex_tools::SEARCH_TEXT_TOOL_NAME;
use codex_tools::ToolName;

use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;

pub struct ContextOpsHandler {
    tool_name: ToolName,
}

impl ContextOpsHandler {
    pub fn new(tool_name: ToolName) -> Self {
        Self { tool_name }
    }
}

impl ToolHandler for ContextOpsHandler {
    type Output = FunctionToolOutput;

    fn tool_name(&self) -> ToolName {
        self.tool_name.clone()
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let arguments = match &invocation.payload {
            ToolPayload::Function { arguments } => arguments.clone(),
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "context ops handler received unsupported payload".to_string(),
                ));
            }
        };

        match invocation.tool_name.name.as_str() {
            FILE_OUTLINE_TOOL_NAME => file_outline::handle(invocation, arguments.as_str()).await,
            GIT_WORKTREE_SUMMARY_TOOL_NAME => {
                git_worktree_summary::handle(invocation, arguments.as_str()).await
            }
            SEARCH_TEXT_TOOL_NAME => search_text::handle(invocation, arguments.as_str()).await,
            other => Err(FunctionCallError::RespondToModel(format!(
                "unknown context ops tool: {other}"
            ))),
        }
    }
}
