use super::*;
use pretty_assertions::assert_eq;

#[test]
fn context_ops_tool_specs_are_compact_and_named() {
    let tools = create_context_ops_tools();
    let names = tools.iter().map(ToolSpec::name).collect::<Vec<_>>();

    assert_eq!(names, vec!["file_outline", "search_text", "workflow_batch"]);

    let ToolSpec::Function(tool) = &tools[0] else {
        panic!("file outline should be a function tool");
    };
    assert_eq!(
        tool.parameters.required.as_deref(),
        Some(&["path".to_string()][..])
    );

    let ToolSpec::Function(tool) = &tools[2] else {
        panic!("workflow batch should be a function tool");
    };
    assert_eq!(tool.parameters.required.as_deref(), None);
    assert!(tool.description.contains("root-confined"));
    assert!(tool.description.contains("inline `spec`"));
    assert!(tool.description.contains("dependent deterministic"));
    assert!(
        tool.description
            .contains("Avoid this tool for simple read-only probes")
    );
}
