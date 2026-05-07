use super::*;
use pretty_assertions::assert_eq;

#[test]
fn context_ops_tool_specs_are_compact_and_named() {
    let tools = create_context_ops_tools();
    let names = tools.iter().map(ToolSpec::name).collect::<Vec<_>>();

    assert_eq!(
        names,
        vec!["file_outline", "git_worktree_summary", "search_text"]
    );

    let ToolSpec::Function(tool) = &tools[0] else {
        panic!("file outline should be a function tool");
    };
    assert_eq!(
        tool.parameters.required.as_deref(),
        Some(&["path".to_string()][..])
    );
}
