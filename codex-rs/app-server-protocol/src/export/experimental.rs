use super::*;

pub(crate) fn filter_experimental_ts(out_dir: &Path) -> Result<()> {
    let registered_fields = experimental_fields();
    let experimental_method_types = experimental_method_types();
    // Most generated TS files are filtered by schema processing, but
    // `ClientRequest.ts` and any type with `#[experimental(...)]` fields need
    // direct post-processing because they encode method/field information in
    // file-local unions/interfaces.
    filter_client_request_ts(out_dir, EXPERIMENTAL_CLIENT_METHODS)?;
    filter_experimental_type_fields_ts(out_dir, &registered_fields)?;
    remove_generated_type_files(out_dir, &experimental_method_types, "ts")?;
    Ok(())
}

pub(crate) fn filter_experimental_ts_tree(tree: &mut BTreeMap<PathBuf, String>) -> Result<()> {
    let registered_fields = experimental_fields();
    let experimental_method_types = experimental_method_types();
    if let Some(content) = tree.get_mut(Path::new("ClientRequest.ts")) {
        let filtered =
            filter_client_request_ts_contents(std::mem::take(content), EXPERIMENTAL_CLIENT_METHODS);
        *content = filtered;
    }

    let mut fields_by_type_name: HashMap<String, HashSet<String>> = HashMap::new();
    for field in registered_fields {
        fields_by_type_name
            .entry(field.type_name.to_string())
            .or_default()
            .insert(field.field_name.to_string());
    }

    for (path, content) in tree.iter_mut() {
        let Some(type_name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let Some(experimental_field_names) = fields_by_type_name.get(type_name) else {
            continue;
        };
        let filtered = filter_experimental_type_fields_ts_contents(
            std::mem::take(content),
            experimental_field_names,
        );
        *content = filtered;
    }

    remove_generated_type_entries(tree, &experimental_method_types, "ts");
    Ok(())
}

/// Removes union arms from `ClientRequest.ts` for methods marked experimental.
fn filter_client_request_ts(out_dir: &Path, experimental_methods: &[&str]) -> Result<()> {
    let path = out_dir.join("ClientRequest.ts");
    if !path.exists() {
        return Ok(());
    }
    let mut content =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    content = filter_client_request_ts_contents(content, experimental_methods);

    fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn filter_client_request_ts_contents(mut content: String, experimental_methods: &[&str]) -> String {
    let Some((prefix, body, suffix)) = split_type_alias(&content) else {
        return content;
    };
    let experimental_methods: HashSet<&str> = experimental_methods
        .iter()
        .copied()
        .filter(|method| !method.is_empty())
        .collect();
    let arms = split_top_level(&body, '|');
    let filtered_arms: Vec<String> = arms
        .into_iter()
        .filter(|arm| {
            extract_method_from_arm(arm)
                .is_none_or(|method| !experimental_methods.contains(method.as_str()))
        })
        .collect();
    let new_body = filtered_arms.join(" | ");
    content = format!("{prefix}{new_body}{suffix}");
    let import_usage_scope = split_type_alias(&content)
        .map(|(_, filtered_body, _)| filtered_body)
        .unwrap_or_else(|| new_body.clone());
    prune_unused_type_imports(content, &import_usage_scope)
}

/// Removes experimental properties from generated TypeScript type files.
fn filter_experimental_type_fields_ts(
    out_dir: &Path,
    experimental_fields: &[&'static crate::experimental_api::ExperimentalField],
) -> Result<()> {
    let mut fields_by_type_name: HashMap<String, HashSet<String>> = HashMap::new();
    for field in experimental_fields {
        fields_by_type_name
            .entry(field.type_name.to_string())
            .or_default()
            .insert(field.field_name.to_string());
    }
    if fields_by_type_name.is_empty() {
        return Ok(());
    }

    for path in ts_files_in_recursive(out_dir)? {
        let Some(type_name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let Some(experimental_field_names) = fields_by_type_name.get(type_name) else {
            continue;
        };
        filter_experimental_fields_in_ts_file(&path, experimental_field_names)?;
    }

    Ok(())
}

fn filter_experimental_fields_in_ts_file(
    path: &Path,
    experimental_field_names: &HashSet<String>,
) -> Result<()> {
    let mut content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    content = filter_experimental_type_fields_ts_contents(content, experimental_field_names);
    fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn filter_experimental_type_fields_ts_contents(
    mut content: String,
    experimental_field_names: &HashSet<String>,
) -> String {
    let Some((open_brace, close_brace)) = type_body_brace_span(&content) else {
        return content;
    };
    let inner = &content[open_brace + 1..close_brace];
    let fields = split_top_level_multi(inner, &[',', ';']);
    let filtered_fields: Vec<String> = fields
        .into_iter()
        .filter(|field| {
            let field = strip_leading_block_comments(field);
            parse_property_name(field)
                .is_none_or(|name| !experimental_field_names.contains(name.as_str()))
        })
        .collect();
    let new_inner = filtered_fields.join(", ");
    let prefix = &content[..open_brace + 1];
    let suffix = &content[close_brace..];
    content = format!("{prefix}{new_inner}{suffix}");
    let import_usage_scope = split_type_alias(&content)
        .map(|(_, body, _)| body)
        .unwrap_or_else(|| new_inner.clone());
    prune_unused_type_imports(content, &import_usage_scope)
}

pub(crate) fn filter_experimental_schema(bundle: &mut Value) -> Result<()> {
    let registered_fields = experimental_fields();
    filter_experimental_fields_in_root(bundle, &registered_fields);
    filter_experimental_fields_in_definitions(bundle, &registered_fields);
    prune_experimental_methods(bundle, EXPERIMENTAL_CLIENT_METHODS);
    remove_experimental_method_type_definitions(bundle);
    Ok(())
}

fn filter_experimental_fields_in_root(
    schema: &mut Value,
    experimental_fields: &[&'static crate::experimental_api::ExperimentalField],
) {
    let Some(title) = schema.get("title").and_then(Value::as_str) else {
        return;
    };
    let title = title.to_string();

    for field in experimental_fields {
        if title != field.type_name {
            continue;
        }
        remove_property_from_schema(schema, field.field_name);
    }
}

fn filter_experimental_fields_in_definitions(
    bundle: &mut Value,
    experimental_fields: &[&'static crate::experimental_api::ExperimentalField],
) {
    let Some(definitions) = bundle.get_mut("definitions").and_then(Value::as_object_mut) else {
        return;
    };

    filter_experimental_fields_in_definitions_map(definitions, experimental_fields);
}

fn filter_experimental_fields_in_definitions_map(
    definitions: &mut Map<String, Value>,
    experimental_fields: &[&'static crate::experimental_api::ExperimentalField],
) {
    for (def_name, def_schema) in definitions.iter_mut() {
        if is_namespace_map(def_schema) {
            if let Some(namespace_defs) = def_schema.as_object_mut() {
                filter_experimental_fields_in_definitions_map(namespace_defs, experimental_fields);
            }
            continue;
        }

        for field in experimental_fields {
            if !definition_matches_type(def_name, field.type_name) {
                continue;
            }
            remove_property_from_schema(def_schema, field.field_name);
        }
    }
}

fn is_namespace_map(value: &Value) -> bool {
    let Value::Object(map) = value else {
        return false;
    };

    if map.keys().any(|key| key.starts_with('$')) {
        return false;
    }

    let looks_like_schema = map.contains_key("type")
        || map.contains_key("properties")
        || map.contains_key("anyOf")
        || map.contains_key("oneOf")
        || map.contains_key("allOf");

    !looks_like_schema && map.values().all(Value::is_object)
}

fn definition_matches_type(def_name: &str, type_name: &str) -> bool {
    def_name == type_name || def_name.ends_with(&format!("::{type_name}"))
}

fn remove_property_from_schema(schema: &mut Value, field_name: &str) {
    if let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) {
        properties.remove(field_name);
    }

    if let Some(required) = schema.get_mut("required").and_then(Value::as_array_mut) {
        required.retain(|entry| entry.as_str() != Some(field_name));
    }

    if let Some(inner_schema) = schema.get_mut("schema") {
        remove_property_from_schema(inner_schema, field_name);
    }
}

fn prune_experimental_methods(bundle: &mut Value, experimental_methods: &[&str]) {
    let experimental_methods: HashSet<&str> = experimental_methods
        .iter()
        .copied()
        .filter(|method| !method.is_empty())
        .collect();
    prune_experimental_methods_inner(bundle, &experimental_methods);
}

fn prune_experimental_methods_inner(value: &mut Value, experimental_methods: &HashSet<&str>) {
    match value {
        Value::Array(items) => {
            items.retain(|item| !is_experimental_method_variant(item, experimental_methods));
            for item in items {
                prune_experimental_methods_inner(item, experimental_methods);
            }
        }
        Value::Object(map) => {
            for entry in map.values_mut() {
                prune_experimental_methods_inner(entry, experimental_methods);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn is_experimental_method_variant(value: &Value, experimental_methods: &HashSet<&str>) -> bool {
    let Value::Object(map) = value else {
        return false;
    };
    let Some(properties) = map.get("properties").and_then(Value::as_object) else {
        return false;
    };
    let Some(method_schema) = properties.get("method").and_then(Value::as_object) else {
        return false;
    };

    if let Some(method) = method_schema.get("const").and_then(Value::as_str) {
        return experimental_methods.contains(method);
    }

    if let Some(values) = method_schema.get("enum").and_then(Value::as_array)
        && values.len() == 1
        && let Some(method) = values[0].as_str()
    {
        return experimental_methods.contains(method);
    }

    false
}

pub(crate) fn filter_experimental_json_files(out_dir: &Path) -> Result<()> {
    for path in json_files_in_recursive(out_dir)? {
        let mut value = read_json_value(&path)?;
        filter_experimental_schema(&mut value)?;
        write_pretty_json(path, &value)?;
    }
    let experimental_method_types = experimental_method_types();
    remove_generated_type_files(out_dir, &experimental_method_types, "json")?;
    Ok(())
}

fn experimental_method_types() -> HashSet<String> {
    let mut type_names = HashSet::new();
    collect_experimental_type_names(EXPERIMENTAL_CLIENT_METHOD_PARAM_TYPES, &mut type_names);
    collect_experimental_type_names(EXPERIMENTAL_CLIENT_METHOD_RESPONSE_TYPES, &mut type_names);
    collect_experimental_type_names(EXPERIMENTAL_CLIENT_METHOD_DEPENDENCY_TYPES, &mut type_names);
    type_names
}

fn collect_experimental_type_names(entries: &[&str], out: &mut HashSet<String>) {
    for entry in entries {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }
        let name = trimmed.rsplit("::").next().unwrap_or(trimmed);
        if !name.is_empty() {
            out.insert(name.to_string());
        }
    }
}

fn remove_generated_type_files(
    out_dir: &Path,
    type_names: &HashSet<String>,
    extension: &str,
) -> Result<()> {
    for type_name in type_names {
        for subdir in ["", "v1", "v2"] {
            let path = if subdir.is_empty() {
                out_dir.join(format!("{type_name}.{extension}"))
            } else {
                out_dir
                    .join(subdir)
                    .join(format!("{type_name}.{extension}"))
            };
            if path.exists() {
                fs::remove_file(&path)
                    .with_context(|| format!("Failed to remove {}", path.display()))?;
            }
        }
    }
    Ok(())
}

fn remove_generated_type_entries(
    tree: &mut BTreeMap<PathBuf, String>,
    type_names: &HashSet<String>,
    extension: &str,
) {
    for type_name in type_names {
        for subdir in ["", "v1", "v2"] {
            let path = if subdir.is_empty() {
                PathBuf::from(format!("{type_name}.{extension}"))
            } else {
                PathBuf::from(subdir).join(format!("{type_name}.{extension}"))
            };
            tree.remove(&path);
        }
    }
}

fn remove_experimental_method_type_definitions(bundle: &mut Value) {
    let type_names = experimental_method_types();
    let Some(definitions) = bundle.get_mut("definitions").and_then(Value::as_object_mut) else {
        return;
    };
    remove_experimental_method_type_definitions_map(definitions, &type_names);
}

fn remove_experimental_method_type_definitions_map(
    definitions: &mut Map<String, Value>,
    experimental_type_names: &HashSet<String>,
) {
    let keys_to_remove: Vec<String> = definitions
        .keys()
        .filter(|def_name| {
            experimental_type_names
                .iter()
                .any(|type_name| definition_matches_type(def_name, type_name))
        })
        .cloned()
        .collect();
    for key in keys_to_remove {
        definitions.remove(&key);
    }

    for value in definitions.values_mut() {
        if !is_namespace_map(value) {
            continue;
        }
        if let Some(namespace_defs) = value.as_object_mut() {
            remove_experimental_method_type_definitions_map(
                namespace_defs,
                experimental_type_names,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::v2;
    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use std::collections::BTreeSet;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[test]
    fn generate_ts_with_experimental_api_retains_experimental_entries() -> Result<()> {
        let client_request_ts = ClientRequest::export_to_string()?;
        assert_eq!(client_request_ts.contains("mock/experimentalMethod"), true);
        assert_eq!(
            client_request_ts.contains("MockExperimentalMethodParams"),
            true
        );
        assert_eq!(
            v2::MockExperimentalMethodParams::export_to_string()?
                .contains("MockExperimentalMethodParams"),
            true
        );
        assert_eq!(
            v2::MockExperimentalMethodResponse::export_to_string()?
                .contains("MockExperimentalMethodResponse"),
            true
        );

        let thread_start_ts = v2::ThreadStartParams::export_to_string()?;
        assert_eq!(thread_start_ts.contains("mockExperimentalField"), true);
        let command_execution_request_approval_ts =
            v2::CommandExecutionRequestApprovalParams::export_to_string()?;
        assert_eq!(
            command_execution_request_approval_ts.contains("additionalPermissions"),
            true
        );

        Ok(())
    }

    #[test]
    fn stable_schema_filter_removes_mock_thread_start_field() -> Result<()> {
        let output_dir = std::env::temp_dir().join(format!("codex_schema_{}", Uuid::now_v7()));
        fs::create_dir(&output_dir)?;
        let schema = write_json_schema_with_return::<v2::ThreadStartParams>(
            &output_dir,
            "ThreadStartParams",
        )?;
        let mut bundle = build_schema_bundle(vec![schema])?;
        filter_experimental_schema(&mut bundle)?;

        let definitions = bundle["definitions"]
            .as_object()
            .expect("schema bundle should include definitions");
        let (_, def_schema) = definitions
            .iter()
            .find(|(name, _)| definition_matches_type(name, "ThreadStartParams"))
            .expect("ThreadStartParams definition should exist");
        let properties = def_schema["properties"]
            .as_object()
            .expect("ThreadStartParams should have properties");
        assert_eq!(properties.contains_key("mockExperimentalField"), false);
        let _cleanup = fs::remove_dir_all(&output_dir);
        Ok(())
    }

    #[test]
    fn experimental_type_fields_ts_filter_handles_interface_shape() -> Result<()> {
        let output_dir = std::env::temp_dir().join(format!("codex_ts_filter_{}", Uuid::now_v7()));
        fs::create_dir_all(&output_dir)?;

        struct TempDirGuard(PathBuf);

        impl Drop for TempDirGuard {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        let _guard = TempDirGuard(output_dir.clone());
        let path = output_dir.join("CustomParams.ts");
        let content = r#"export interface CustomParams {
  stableField: string | null;
  unstableField: string | null;
  otherStableField: boolean;
}
"#;
        fs::write(&path, content)?;

        static CUSTOM_FIELD: crate::experimental_api::ExperimentalField =
            crate::experimental_api::ExperimentalField {
                type_name: "CustomParams",
                field_name: "unstableField",
                reason: "custom/unstableField",
            };
        filter_experimental_type_fields_ts(&output_dir, &[&CUSTOM_FIELD])?;

        let filtered = fs::read_to_string(&path)?;
        assert_eq!(filtered.contains("unstableField"), false);
        assert_eq!(filtered.contains("stableField"), true);
        assert_eq!(filtered.contains("otherStableField"), true);
        Ok(())
    }

    #[test]
    fn experimental_type_fields_ts_filter_keeps_imports_used_in_intersection_suffix() -> Result<()>
    {
        let output_dir = std::env::temp_dir().join(format!("codex_ts_filter_{}", Uuid::now_v7()));
        fs::create_dir_all(&output_dir)?;

        struct TempDirGuard(PathBuf);

        impl Drop for TempDirGuard {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        let _guard = TempDirGuard(output_dir.clone());
        let path = output_dir.join("Config.ts");
        let content = r#"import type { JsonValue } from "../serde_json/JsonValue";
import type { Keep } from "./Keep";

export type Config = { stableField: Keep, unstableField: string | null } & ({ [key in string]?: number | string | boolean | Array<JsonValue> | { [key in string]?: JsonValue } | null });
"#;
        fs::write(&path, content)?;

        static CUSTOM_FIELD: crate::experimental_api::ExperimentalField =
            crate::experimental_api::ExperimentalField {
                type_name: "Config",
                field_name: "unstableField",
                reason: "custom/unstableField",
            };
        filter_experimental_type_fields_ts(&output_dir, &[&CUSTOM_FIELD])?;

        let filtered = fs::read_to_string(&path)?;
        assert_eq!(filtered.contains("unstableField"), false);
        assert_eq!(
            filtered.contains(r#"import type { JsonValue } from "../serde_json/JsonValue";"#),
            true
        );
        assert_eq!(
            filtered.contains(r#"import type { Keep } from "./Keep";"#),
            true
        );
        Ok(())
    }

    #[test]
    fn experimental_type_fields_ts_filter_handles_generated_command_params_shape() -> Result<()> {
        let output_dir = std::env::temp_dir().join(format!("codex_ts_filter_{}", Uuid::now_v7()));
        fs::create_dir_all(&output_dir)?;

        struct TempDirGuard(PathBuf);

        impl Drop for TempDirGuard {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        let _guard = TempDirGuard(output_dir.clone());
        let path = output_dir.join("CommandExecParams.ts");
        let content = r#"import type { CommandExecTerminalSize } from "./CommandExecTerminalSize";
import type { SandboxPolicy } from "./SandboxPolicy";

export type CommandExecParams = {/**
 * Command argv vector. Empty arrays are rejected.
 */
command: Array<string>, /**
 * Optional environment overrides merged into the server-computed
 * environment.
 */
env?: { [key in string]?: string | null } | null, /**
 * Optional initial PTY size in character cells. Only valid when `tty` is
 * true.
 */
size?: CommandExecTerminalSize | null, /**
 * Optional sandbox policy for this command.
 *
 * Uses the same shape as thread/turn execution sandbox configuration and
 * defaults to the user's configured policy when omitted. Cannot be
 * combined with `permissionProfile`.
 */
sandboxPolicy?: SandboxPolicy | null,
/**
 * Optional active permissions profile id for this command.
 *
 * Defaults to the user's configured permissions when omitted. Cannot be
 * combined with `sandboxPolicy`.
 */
permissionProfile?: string | null};
"#;
        fs::write(&path, content)?;

        static CUSTOM_FIELD: crate::experimental_api::ExperimentalField =
            crate::experimental_api::ExperimentalField {
                type_name: "CommandExecParams",
                field_name: "permissionProfile",
                reason: "command/exec.permissionProfile",
            };
        filter_experimental_type_fields_ts(&output_dir, &[&CUSTOM_FIELD])?;

        let filtered = fs::read_to_string(&path)?;
        assert_eq!(filtered.contains("permissionProfile?: string"), false);
        assert_eq!(filtered.contains("sandboxPolicy?: SandboxPolicy"), true);
        assert_eq!(
            filtered.contains(r#"import type { SandboxPolicy } from "./SandboxPolicy";"#),
            true
        );
        Ok(())
    }

    #[test]
    fn stable_schema_filter_removes_mock_experimental_method() -> Result<()> {
        let output_dir = std::env::temp_dir().join(format!("codex_schema_{}", Uuid::now_v7()));
        fs::create_dir(&output_dir)?;
        let schema =
            write_json_schema_with_return::<crate::ClientRequest>(&output_dir, "ClientRequest")?;
        let mut bundle = build_schema_bundle(vec![schema])?;
        filter_experimental_schema(&mut bundle)?;

        let bundle_str = serde_json::to_string(&bundle)?;
        assert_eq!(bundle_str.contains("mock/experimentalMethod"), false);
        let _cleanup = fs::remove_dir_all(&output_dir);
        Ok(())
    }

    #[test]
    fn generate_json_filters_experimental_fields_and_methods() -> Result<()> {
        let output_dir = std::env::temp_dir().join(format!("codex_schema_{}", Uuid::now_v7()));
        fs::create_dir(&output_dir)?;
        generate_json_with_experimental(&output_dir, /*experimental_api*/ false)?;

        let thread_start_json =
            fs::read_to_string(output_dir.join("v2").join("ThreadStartParams.json"))?;
        assert_eq!(thread_start_json.contains("mockExperimentalField"), false);
        let command_execution_request_approval_json =
            fs::read_to_string(output_dir.join("CommandExecutionRequestApprovalParams.json"))?;
        assert_eq!(
            command_execution_request_approval_json.contains("additionalPermissions"),
            false
        );

        let client_request_json = fs::read_to_string(output_dir.join("ClientRequest.json"))?;
        assert_eq!(
            client_request_json.contains("mock/experimentalMethod"),
            false
        );
        assert_eq!(output_dir.join("EventMsg.json").exists(), false);

        let bundle_json =
            fs::read_to_string(output_dir.join("codex_app_server_protocol.schemas.json"))?;
        assert_eq!(bundle_json.contains("mockExperimentalField"), false);
        assert_eq!(bundle_json.contains("additionalPermissions"), false);
        assert_eq!(bundle_json.contains("MockExperimentalMethodParams"), false);
        assert_eq!(
            bundle_json.contains("MockExperimentalMethodResponse"),
            false
        );
        let flat_v2_bundle_json =
            fs::read_to_string(output_dir.join("codex_app_server_protocol.v2.schemas.json"))?;
        assert_eq!(flat_v2_bundle_json.contains("mockExperimentalField"), false);
        assert_eq!(flat_v2_bundle_json.contains("additionalPermissions"), false);
        assert_eq!(
            flat_v2_bundle_json.contains("MockExperimentalMethodParams"),
            false
        );
        assert_eq!(
            flat_v2_bundle_json.contains("MockExperimentalMethodResponse"),
            false
        );
        assert_eq!(flat_v2_bundle_json.contains("#/definitions/v2/"), false);
        assert_eq!(
            flat_v2_bundle_json.contains("\"title\": \"CodexAppServerProtocolV2\""),
            true
        );
        let flat_v2_bundle =
            read_json_value(&output_dir.join("codex_app_server_protocol.v2.schemas.json"))?;
        let definitions = flat_v2_bundle["definitions"]
            .as_object()
            .expect("flat v2 bundle should include definitions");
        let client_request_methods: BTreeSet<String> = definitions["ClientRequest"]["oneOf"]
            .as_array()
            .expect("flat v2 ClientRequest should remain a oneOf")
            .iter()
            .filter_map(|variant| {
                variant["properties"]["method"]["enum"]
                    .as_array()
                    .and_then(|values| values.first())
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .collect();
        let missing_client_request_methods: Vec<String> = [
            "account/logout",
            "account/rateLimits/read",
            "config/mcpServer/reload",
            "configRequirements/read",
            "fuzzyFileSearch",
            "initialize",
        ]
        .into_iter()
        .filter(|method| !client_request_methods.contains(*method))
        .map(str::to_string)
        .collect();
        assert_eq!(missing_client_request_methods, Vec::<String>::new());
        let server_notification_methods: BTreeSet<String> =
            definitions["ServerNotification"]["oneOf"]
                .as_array()
                .expect("flat v2 ServerNotification should remain a oneOf")
                .iter()
                .filter_map(|variant| {
                    variant["properties"]["method"]["enum"]
                        .as_array()
                        .and_then(|values| values.first())
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .collect();
        let missing_server_notification_methods: Vec<String> = [
            "fuzzyFileSearch/sessionCompleted",
            "fuzzyFileSearch/sessionUpdated",
            "serverRequest/resolved",
        ]
        .into_iter()
        .filter(|method| !server_notification_methods.contains(*method))
        .map(str::to_string)
        .collect();
        assert_eq!(missing_server_notification_methods, Vec::<String>::new());
        assert_eq!(definitions.contains_key("EventMsg"), false);
        assert_eq!(
            output_dir
                .join("v2")
                .join("MockExperimentalMethodParams.json")
                .exists(),
            false
        );
        assert_eq!(
            output_dir
                .join("v2")
                .join("MockExperimentalMethodResponse.json")
                .exists(),
            false
        );

        let _cleanup = fs::remove_dir_all(&output_dir);
        Ok(())
    }
}
