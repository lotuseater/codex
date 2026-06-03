use std::future::Future;
use std::pin::Pin;

use codex_tool_execution_api::FunctionCallError;
use codex_tool_execution_api::ToolExecutor;
use codex_tools::ToolCall;
use codex_tool_execution_api::ToolName;
use codex_tool_execution_api::ToolOutput;
use codex_tool_registry_api::ToolExposure;
use codex_tool_registry_api::ToolSpec;

/// Model-facing output returned by extension-owned tools.
pub type ExtensionToolOutput = Box<dyn ToolOutput>;

/// Future returned by an extension-owned executable tool.
pub type ExtensionToolFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ExtensionToolOutput, FunctionCallError>> + Send + 'a>>;

/// Object-safe adapter for extension-owned executable tools.
///
/// Extensions implement the shared `ToolExecutor<ToolCall>` contract directly;
/// this adapter keeps extension registries dynamically dispatchable without
/// making the core `ToolExecutor` trait give up its RPITIT future shape.
pub trait ExtensionToolExecutor: Send + Sync {
    fn tool_name(&self) -> ToolName;

    fn spec(&self) -> Option<ToolSpec>;

    fn exposure(&self) -> ToolExposure;

    fn supports_parallel_tool_calls(&self) -> bool;

    fn handle(&self, invocation: ToolCall) -> ExtensionToolFuture<'_>;
}

impl<T> ExtensionToolExecutor for T
where
    T: ToolExecutor<ToolCall, Output = ExtensionToolOutput>,
{
    fn tool_name(&self) -> ToolName {
        ToolExecutor::tool_name(self)
    }

    fn spec(&self) -> Option<ToolSpec> {
        ToolExecutor::spec(self)
    }

    fn exposure(&self) -> ToolExposure {
        ToolExecutor::exposure(self)
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        ToolExecutor::supports_parallel_tool_calls(self)
    }

    fn handle(&self, invocation: ToolCall) -> ExtensionToolFuture<'_> {
        Box::pin(ToolExecutor::handle(self, invocation))
    }
}
