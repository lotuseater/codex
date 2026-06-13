#![allow(clippy::expect_used)]

use codex_core::test_support::hosted_web_namespace_alias_probe;

const ALIASED_WEB_NAMESPACE: &str = "codex_ext_web";
const SOURCE_WEB_NAMESPACE: &str = "web";
const WEB_TOOL_NAME: &str = "open";

#[test]
fn hosted_web_search_aliases_dynamic_web_namespace_in_request() {
    let probe = hosted_web_namespace_alias_probe(
        /*defer_loading*/ false, /*namespace_tools*/ true, /*tool_search*/ false,
    );

    assert_contains(&probe.model_tool_names, "web_search");
    assert_contains(&probe.model_tool_names, ALIASED_WEB_NAMESPACE);
    assert_lacks(&probe.model_tool_names, SOURCE_WEB_NAMESPACE);
    assert_namespace_child(
        &probe.namespace_children,
        ALIASED_WEB_NAMESPACE,
        WEB_TOOL_NAME,
    );
    assert_no_namespace_child(
        &probe.namespace_children,
        SOURCE_WEB_NAMESPACE,
        WEB_TOOL_NAME,
    );
    assert_contains_executor(
        &probe.executor_tool_names,
        Some(ALIASED_WEB_NAMESPACE),
        WEB_TOOL_NAME,
    );
    assert_lacks_executor(
        &probe.executor_tool_names,
        Some(SOURCE_WEB_NAMESPACE),
        WEB_TOOL_NAME,
    );
}

#[test]
fn hosted_web_search_aliases_deferred_dynamic_web_namespace_in_tool_search_output() {
    let probe = hosted_web_namespace_alias_probe(
        /*defer_loading*/ true, /*namespace_tools*/ true, /*tool_search*/ true,
    );

    assert_contains(&probe.model_tool_names, "web_search");
    assert_contains(&probe.model_tool_names, "tool_search");
    assert_lacks(&probe.model_tool_names, SOURCE_WEB_NAMESPACE);
    assert_lacks(&probe.model_tool_names, ALIASED_WEB_NAMESPACE);
    assert_contains(&probe.deferred_search_namespaces, ALIASED_WEB_NAMESPACE);
    assert_lacks(&probe.deferred_search_namespaces, SOURCE_WEB_NAMESPACE);
    assert_namespace_child(
        &probe.deferred_search_children,
        ALIASED_WEB_NAMESPACE,
        WEB_TOOL_NAME,
    );
    assert_no_namespace_child(
        &probe.deferred_search_children,
        SOURCE_WEB_NAMESPACE,
        WEB_TOOL_NAME,
    );
    assert_contains_executor(
        &probe.executor_tool_names,
        Some(ALIASED_WEB_NAMESPACE),
        WEB_TOOL_NAME,
    );
    assert_lacks_executor(
        &probe.executor_tool_names,
        Some(SOURCE_WEB_NAMESPACE),
        WEB_TOOL_NAME,
    );
}

#[test]
fn hosted_web_search_alias_keeps_dispatch_handler_when_namespace_specs_hidden() {
    let probe = hosted_web_namespace_alias_probe(
        /*defer_loading*/ false, /*namespace_tools*/ false, /*tool_search*/ false,
    );

    assert_contains(&probe.model_tool_names, "web_search");
    assert_lacks(&probe.model_tool_names, SOURCE_WEB_NAMESPACE);
    assert_lacks(&probe.model_tool_names, ALIASED_WEB_NAMESPACE);
    assert_contains_executor(
        &probe.executor_tool_names,
        Some(ALIASED_WEB_NAMESPACE),
        WEB_TOOL_NAME,
    );
    assert_lacks_executor(
        &probe.executor_tool_names,
        Some(SOURCE_WEB_NAMESPACE),
        WEB_TOOL_NAME,
    );
}

fn assert_contains(values: &[String], expected: &str) {
    assert!(
        values.iter().any(|value| value == expected),
        "expected {expected:?} in {values:?}"
    );
}

fn assert_lacks(values: &[String], unexpected: &str) {
    assert!(
        !values.iter().any(|value| value == unexpected),
        "did not expect {unexpected:?} in {values:?}"
    );
}

fn assert_namespace_child(
    children_by_namespace: &[(String, Vec<String>)],
    namespace: &str,
    child: &str,
) {
    assert!(
        children_by_namespace
            .iter()
            .any(|(candidate_namespace, children)| {
                candidate_namespace == namespace && children.iter().any(|name| name == child)
            }),
        "expected namespace {namespace:?} to contain {child:?}: {children_by_namespace:?}"
    );
}

fn assert_no_namespace_child(
    children_by_namespace: &[(String, Vec<String>)],
    namespace: &str,
    child: &str,
) {
    assert!(
        !children_by_namespace
            .iter()
            .any(|(candidate_namespace, children)| {
                candidate_namespace == namespace && children.iter().any(|name| name == child)
            }),
        "did not expect namespace {namespace:?} to contain {child:?}: {children_by_namespace:?}"
    );
}

fn assert_contains_executor(
    executor_names: &[(Option<String>, String)],
    namespace: Option<&str>,
    name: &str,
) {
    assert!(
        executor_names
            .iter()
            .any(|(candidate_namespace, candidate_name)| {
                candidate_namespace.as_deref() == namespace && candidate_name == name
            }),
        "expected executor {namespace:?}/{name:?} in {executor_names:?}"
    );
}

fn assert_lacks_executor(
    executor_names: &[(Option<String>, String)],
    namespace: Option<&str>,
    name: &str,
) {
    assert!(
        !executor_names
            .iter()
            .any(|(candidate_namespace, candidate_name)| {
                candidate_namespace.as_deref() == namespace && candidate_name == name
            }),
        "did not expect executor {namespace:?}/{name:?} in {executor_names:?}"
    );
}
