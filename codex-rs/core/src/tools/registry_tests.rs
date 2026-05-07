use super::*;
use crate::session::tests::make_session_and_context;
use crate::tools::context::ToolCallSource;
use crate::turn_diff_tracker::TurnDiffTracker;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

struct TestHandler {
    tool_name: codex_tools::ToolName,
}

impl ToolHandler for TestHandler {
    type Output = crate::tools::context::FunctionToolOutput;

    fn tool_name(&self) -> codex_tools::ToolName {
        self.tool_name.clone()
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, _invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        Ok(crate::tools::context::FunctionToolOutput::from_text(
            "ok".to_string(),
            Some(true),
        ))
    }
}

#[test]
fn handler_looks_up_namespaced_aliases_explicitly() {
    let namespace = "mcp__codex_apps__gmail";
    let tool_name = "gmail_get_recent_emails";
    let plain_name = codex_tools::ToolName::plain(tool_name);
    let namespaced_name = codex_tools::ToolName::namespaced(namespace, tool_name);
    let plain_handler = Arc::new(TestHandler {
        tool_name: plain_name.clone(),
    }) as Arc<dyn AnyToolHandler>;
    let namespaced_handler = Arc::new(TestHandler {
        tool_name: namespaced_name.clone(),
    }) as Arc<dyn AnyToolHandler>;
    let registry = ToolRegistry::new(HashMap::from([
        (plain_name.clone(), Arc::clone(&plain_handler)),
        (namespaced_name.clone(), Arc::clone(&namespaced_handler)),
    ]));

    let plain = registry.handler(&plain_name);
    let namespaced = registry.handler(&namespaced_name);
    let missing_namespaced = registry.handler(&codex_tools::ToolName::namespaced(
        "mcp__codex_apps__calendar",
        tool_name,
    ));

    assert_eq!(plain.is_some(), true);
    assert_eq!(namespaced.is_some(), true);
    assert_eq!(missing_namespaced.is_none(), true);
    assert!(
        plain
            .as_ref()
            .is_some_and(|handler| Arc::ptr_eq(handler, &plain_handler))
    );
    assert!(
        namespaced
            .as_ref()
            .is_some_and(|handler| Arc::ptr_eq(handler, &namespaced_handler))
    );
}

#[tokio::test]
async fn operation_cache_cwd_uses_function_workdir() {
    let (session, turn) = make_session_and_context().await;
    let session = Arc::new(session);
    let turn = Arc::new(turn);
    let invocation = ToolInvocation {
        session,
        turn: turn.clone(),
        cancellation_token: CancellationToken::new(),
        tracker: Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new())),
        call_id: "call-1".to_string(),
        tool_name: ToolName::plain("exec_command"),
        source: ToolCallSource::Direct,
        payload: ToolPayload::Function {
            arguments: serde_json::json!({
                "cmd": "Get-ChildItem",
                "workdir": "nested",
            })
            .to_string(),
        },
    };

    assert_eq!(
        operation_cache_cwd(&invocation),
        turn.resolve_path(Some("nested".to_string()))
    );
}

#[tokio::test]
async fn operation_cache_cwd_uses_local_shell_workdir() {
    let (session, turn) = make_session_and_context().await;
    let session = Arc::new(session);
    let turn = Arc::new(turn);
    let invocation = ToolInvocation {
        session,
        turn: turn.clone(),
        cancellation_token: CancellationToken::new(),
        tracker: Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new())),
        call_id: "call-1".to_string(),
        tool_name: ToolName::plain("shell"),
        source: ToolCallSource::Direct,
        payload: ToolPayload::LocalShell {
            params: codex_protocol::models::ShellToolCallParams {
                command: vec!["Get-ChildItem".to_string()],
                workdir: Some("nested".to_string()),
                timeout_ms: None,
                sandbox_permissions: None,
                additional_permissions: None,
                prefix_rule: None,
                justification: None,
            },
        },
    };

    assert_eq!(
        operation_cache_cwd(&invocation),
        turn.resolve_path(Some("nested".to_string()))
    );
}
