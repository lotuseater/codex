use super::*;
use pretty_assertions::assert_eq;

#[test]
fn context_ops_tool_specs_are_compact_and_named() {
    let tools = create_context_ops_tools();
    let names = tools.iter().map(ToolSpec::name).collect::<Vec<_>>();

    assert_eq!(names, vec!["file_outline", "search_text"]);

    let ToolSpec::Function(tool) = &tools[0] else {
        panic!("file outline should be a function tool");
    };
    assert_eq!(
        tool.parameters.required.as_deref(),
        Some(&["path".to_string()][..])
    );
}

#[test]
fn workflow_batch_tool_spec_is_compact_and_named() {
    let ToolSpec::Function(tool) = create_workflow_batch_tool() else {
        panic!("workflow batch should be a function tool");
    };

    assert_eq!(tool.name, "workflow_batch");
    assert_eq!(tool.parameters.required.as_deref(), None);
    assert!(tool.description.contains("root-confined"));
    assert!(tool.description.contains("inline `spec`"));
    assert!(tool.description.contains("spec_path"));
    assert!(tool.description.contains("report_path"));
    assert!(tool.description.contains("response_length"));
    assert!(tool.description.contains("dependent deterministic"));
    assert!(
        tool.description
            .contains("safe PowerShell-like substitutions")
    );
    assert!(tool.description.contains("stat_path"));
    assert!(tool.description.contains("list_files"));
    assert!(tool.description.contains("map/filter/reduce/scan"));
    assert!(
        tool.description
            .contains("bounded recursive conditional scans")
    );
    assert!(
        tool.description
            .contains("Use Python for richer algorithms/data structures/libraries")
    );
    assert!(
        tool.description
            .contains("shell/rg for single read-only probes")
    );
    let Some(properties) = tool.parameters.properties.as_ref() else {
        panic!("workflow batch parameters should have properties");
    };
    let Some(spec_property) = properties.get("spec") else {
        panic!("workflow batch spec property should be present");
    };
    let spec_description = spec_property
        .description
        .as_deref()
        .expect("workflow batch spec should be described");
    assert!(spec_description.contains("ensure_dir"));
    assert!(spec_description.contains("stat_path"));
    assert!(spec_description.contains("list_files"));
    assert!(spec_description.contains("Step payloads are objects"));
    assert!(spec_description.contains("not bare path strings"));
    assert!(spec_description.contains("object records via literal"));
}
