use crate::tools::registry::PostToolUsePayload;
use crate::tools::registry::PreToolUsePayload;

pub(crate) use codex_operation_cache::OperationCacheHit;

pub(crate) async fn lookup(
    payload: &PreToolUsePayload,
    cwd: &std::path::Path,
) -> Option<OperationCacheHit> {
    codex_operation_cache::lookup(payload.tool_name.name(), &payload.tool_input, cwd).await
}

pub(crate) async fn store(payload: &PostToolUsePayload, cwd: &std::path::Path) {
    codex_operation_cache::store(
        payload.tool_name.name(),
        &payload.tool_input,
        &payload.tool_response,
        cwd,
    )
    .await;
}
