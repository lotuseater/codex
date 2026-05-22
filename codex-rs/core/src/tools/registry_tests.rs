use super::*;
use pretty_assertions::assert_eq;
use std::sync::Arc;

struct TestHandler {
    tool_name: ToolName,
}

impl ToolExecutor<ToolInvocation> for TestHandler {
    type Output = crate::tools::context::FunctionToolOutput;

    fn tool_name(&self) -> ToolName {
        self.tool_name.clone()
    }

    async fn handle(&self, _invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        Ok(crate::tools::context::FunctionToolOutput::from_text(
            "ok".to_string(),
            Some(true),
        ))
    }
}

impl ToolHandler for TestHandler {}

#[test]
fn handler_looks_up_namespaced_aliases_explicitly() {
    let namespace = "mcp__codex_apps__gmail";
    let tool_name = "gmail_get_recent_emails";
    let plain_name = ToolName::plain(tool_name);
    let namespaced_name = ToolName::namespaced(namespace, tool_name);
    let plain_handler = Arc::new(TestHandler {
        tool_name: plain_name.clone(),
    }) as Arc<dyn RegisteredTool>;
    let namespaced_handler = Arc::new(TestHandler {
        tool_name: namespaced_name.clone(),
    }) as Arc<dyn RegisteredTool>;
    let registry = ToolRegistry::new(HashMap::from([
        (plain_name.clone(), Arc::clone(&plain_handler)),
        (namespaced_name.clone(), Arc::clone(&namespaced_handler)),
    ]));

    let plain = registry.tool(&plain_name);
    let namespaced = registry.tool(&namespaced_name);
    let missing_namespaced = registry.tool(&ToolName::namespaced(
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
