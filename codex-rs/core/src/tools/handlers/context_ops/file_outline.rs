use serde::Deserialize;

use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::handlers::parse_arguments;

use super::execution;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileOutlineArgs {
    path: String,
    #[serde(default)]
    workdir: Option<String>,
    #[serde(default)]
    max_items: Option<usize>,
}

pub(super) async fn handle(
    invocation: ToolInvocation,
    arguments: &str,
) -> Result<FunctionToolOutput, FunctionCallError> {
    let args: FileOutlineArgs = parse_arguments(arguments)?;
    let turn_environment = execution::primary_environment(&invocation)?;
    let base_path = execution::resolve_workdir(turn_environment, args.workdir.as_deref());
    let path = base_path.join(args.path);
    let max_items = args
        .max_items
        .unwrap_or(codex_context_ops_impl::DEFAULT_MAX_OUTLINE_ITEMS);
    let bytes = execution::read_file(&invocation, turn_environment, &path).await?;
    let output = codex_context_ops_impl::file_outline_from_bytes(path.as_path(), &bytes, max_items);
    Ok(FunctionToolOutput::from_text(output, Some(true)))
}
