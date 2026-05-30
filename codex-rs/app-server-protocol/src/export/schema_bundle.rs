use super::*;

pub(crate) fn build_schema_bundle(schemas: Vec<GeneratedSchema>) -> Result<Value> {
    let namespaced_types = collect_namespaced_types(&schemas);
    let mut definitions = Map::new();

    for schema in schemas {
        let GeneratedSchema {
            namespace,
            logical_name,
            mut value,
            in_v1_dir,
        } = schema;

        if IGNORED_DEFINITIONS.contains(&logical_name.as_str()) {
            continue;
        }

        if let Some(ref ns) = namespace {
            rewrite_refs_to_namespace(&mut value, ns);
        } else {
            rewrite_refs_to_known_namespaces(&mut value, &namespaced_types);
        }

        let mut forced_namespace_refs: Vec<(String, String)> = Vec::new();
        if let Value::Object(ref mut obj) = value
            && let Some(defs) = obj.remove("definitions")
            && let Value::Object(defs_obj) = defs
        {
            for (def_name, mut def_schema) in defs_obj {
                if IGNORED_DEFINITIONS.contains(&def_name.as_str()) {
                    continue;
                }
                if SPECIAL_DEFINITIONS.contains(&def_name.as_str()) {
                    continue;
                }
                annotate_schema(&mut def_schema, Some(def_name.as_str()));
                let target_namespace = match namespace {
                    Some(ref ns) => Some(ns.clone()),
                    None => namespace_for_definition(&def_name, &namespaced_types)
                        .cloned()
                        .filter(|_| !in_v1_dir),
                };
                if let Some(ref ns) = target_namespace {
                    if namespace.as_deref() == Some(ns.as_str()) {
                        rewrite_refs_to_namespace(&mut def_schema, ns);
                        insert_into_namespace(&mut definitions, ns, def_name.clone(), def_schema)?;
                    } else if !forced_namespace_refs
                        .iter()
                        .any(|(name, existing_ns)| name == &def_name && existing_ns == ns)
                    {
                        forced_namespace_refs.push((def_name.clone(), ns.clone()));
                    }
                } else {
                    definitions.insert(def_name, def_schema);
                }
            }
        }

        for (name, ns) in forced_namespace_refs {
            rewrite_named_ref_to_namespace(&mut value, &ns, &name);
        }

        if let Some(ref ns) = namespace {
            insert_into_namespace(&mut definitions, ns, logical_name.clone(), value)?;
        } else {
            definitions.insert(logical_name, value);
        }
    }

    let mut root = Map::new();
    root.insert(
        "$schema".to_string(),
        Value::String("http://json-schema.org/draft-07/schema#".into()),
    );
    root.insert(
        "title".to_string(),
        Value::String("CodexAppServerProtocol".into()),
    );
    root.insert("type".to_string(), Value::String("object".into()));
    root.insert("definitions".to_string(), Value::Object(definitions));

    Ok(Value::Object(root))
}

/// Build a datamodel-code-generator-friendly v2 bundle from the mixed export.
///
/// The full bundle keeps v2 schemas nested under `definitions.v2`, plus a few
/// shared root definitions like `ClientRequest` and `ServerNotification`.
/// Python codegen only walks one definitions map level, so
/// a direct feed would treat `v2` itself as a schema and miss unreferenced v2
/// leaves. This helper flattens all v2 definitions to the root definitions map,
/// then pulls in the shared root schemas and any non-v2 transitive deps they
/// still reference. Keep the shared root unions intact here: some valid
/// request/notification/event variants are inline or only reference shared root
/// helpers, so filtering them by the presence of a `#/definitions/v2/` ref
/// would silently drop real API surface from the flat bundle.
pub(crate) fn build_flat_v2_schema(bundle: &Value) -> Result<Value> {
    let Value::Object(root) = bundle else {
        return Err(anyhow!("expected bundle root to be an object"));
    };
    let definitions = root
        .get("definitions")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("expected bundle definitions map"))?;
    let v2_definitions = definitions
        .get("v2")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("expected v2 namespace in bundle definitions"))?;

    let mut flat_root = root.clone();
    let title = root
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("CodexAppServerProtocol");
    let mut flat_definitions = v2_definitions.clone();
    let mut shared_definitions = Map::new();
    let mut non_v2_refs = HashSet::new();

    for shared in FLAT_V2_SHARED_DEFINITIONS {
        let Some(shared_schema) = definitions.get(*shared) else {
            continue;
        };
        let shared_schema = shared_schema.clone();
        non_v2_refs.extend(collect_non_v2_refs(&shared_schema));
        shared_definitions.insert((*shared).to_string(), shared_schema);
    }

    for name in collect_definition_dependencies(definitions, non_v2_refs) {
        if name == "v2" || flat_definitions.contains_key(&name) {
            continue;
        }
        if let Some(schema) = definitions.get(&name) {
            flat_definitions.insert(name, schema.clone());
        }
    }

    flat_definitions.extend(shared_definitions);
    flat_root.insert("title".to_string(), Value::String(format!("{title}V2")));
    flat_root.insert("definitions".to_string(), Value::Object(flat_definitions));
    let mut flat_bundle = Value::Object(flat_root);
    rewrite_ref_prefix(&mut flat_bundle, "#/definitions/v2/", "#/definitions/");
    ensure_no_ref_prefix(&flat_bundle, "#/definitions/v2/", "flat v2")?;
    ensure_referenced_definitions_present(&flat_bundle, "flat v2")?;
    Ok(flat_bundle)
}

fn collect_non_v2_refs(value: &Value) -> HashSet<String> {
    let mut refs = HashSet::new();
    collect_non_v2_refs_inner(value, &mut refs);
    refs
}

fn collect_non_v2_refs_inner(value: &Value, refs: &mut HashSet<String>) {
    match value {
        Value::Object(obj) => {
            if let Some(Value::String(reference)) = obj.get("$ref")
                && let Some(name) = reference.strip_prefix("#/definitions/")
                && !reference.starts_with("#/definitions/v2/")
            {
                refs.insert(name.to_string());
            }
            for child in obj.values() {
                collect_non_v2_refs_inner(child, refs);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_non_v2_refs_inner(child, refs);
            }
        }
        _ => {}
    }
}

fn collect_definition_dependencies(
    definitions: &Map<String, Value>,
    names: HashSet<String>,
) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut to_process: Vec<String> = names.into_iter().collect();
    while let Some(name) = to_process.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        let Some(schema) = definitions.get(&name) else {
            continue;
        };
        for dep in collect_non_v2_refs(schema) {
            if !seen.contains(&dep) {
                to_process.push(dep);
            }
        }
    }
    seen
}

fn rewrite_ref_prefix(value: &mut Value, prefix: &str, replacement: &str) {
    match value {
        Value::Object(obj) => {
            if let Some(Value::String(reference)) = obj.get_mut("$ref") {
                *reference = reference.replace(prefix, replacement);
            }
            for child in obj.values_mut() {
                rewrite_ref_prefix(child, prefix, replacement);
            }
        }
        Value::Array(items) => {
            for child in items {
                rewrite_ref_prefix(child, prefix, replacement);
            }
        }
        _ => {}
    }
}

fn ensure_no_ref_prefix(value: &Value, prefix: &str, label: &str) -> Result<()> {
    if let Some(reference) = first_ref_with_prefix(value, prefix) {
        return Err(anyhow!(
            "{label} schema still references namespaced definitions; found {reference}"
        ));
    }
    Ok(())
}

pub(crate) fn first_ref_with_prefix(value: &Value, prefix: &str) -> Option<String> {
    match value {
        Value::Object(obj) => {
            if let Some(Value::String(reference)) = obj.get("$ref")
                && reference.starts_with(prefix)
            {
                return Some(reference.clone());
            }
            obj.values()
                .find_map(|child| first_ref_with_prefix(child, prefix))
        }
        Value::Array(items) => items
            .iter()
            .find_map(|child| first_ref_with_prefix(child, prefix)),
        _ => None,
    }
}

fn ensure_referenced_definitions_present(schema: &Value, label: &str) -> Result<()> {
    let definitions = schema
        .get("definitions")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("expected definitions map in {label} schema"))?;
    let mut missing = HashSet::new();
    collect_missing_definitions(schema, definitions, &mut missing);
    if missing.is_empty() {
        return Ok(());
    }
    let mut missing_names: Vec<String> = missing.into_iter().collect();
    missing_names.sort();
    Err(anyhow!(
        "{label} schema missing definitions: {}",
        missing_names.join(", ")
    ))
}

fn collect_missing_definitions(
    value: &Value,
    definitions: &Map<String, Value>,
    missing: &mut HashSet<String>,
) {
    match value {
        Value::Object(obj) => {
            if let Some(Value::String(reference)) = obj.get("$ref")
                && let Some(name) = reference.strip_prefix("#/definitions/")
            {
                let name = name.split('/').next().unwrap_or(name);
                if !definitions.contains_key(name) {
                    missing.insert(name.to_string());
                }
            }
            for child in obj.values() {
                collect_missing_definitions(child, definitions, missing);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_missing_definitions(child, definitions, missing);
            }
        }
        _ => {}
    }
}

fn insert_into_namespace(
    definitions: &mut Map<String, Value>,
    namespace: &str,
    name: String,
    schema: Value,
) -> Result<()> {
    let entry = definitions
        .entry(namespace.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    match entry {
        Value::Object(map) => {
            insert_definition(map, name, schema, &format!("namespace `{namespace}`"))
        }
        _ => Err(anyhow!("expected namespace {namespace} to be an object")),
    }
}

fn insert_definition(
    definitions: &mut Map<String, Value>,
    name: String,
    schema: Value,
    location: &str,
) -> Result<()> {
    if let Some(existing) = definitions.get(&name) {
        if existing == &schema {
            return Ok(());
        }

        let existing_title = existing
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("<untitled>");
        let new_title = schema
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("<untitled>");
        return Err(anyhow!(
            "schema definition collision in {location}: {name} (existing title: {existing_title}, new title: {new_title}); use #[schemars(rename = \"...\")] to rename one of the conflicting schema definitions"
        ));
    }

    definitions.insert(name, schema);
    Ok(())
}

pub(crate) fn write_json_schema_with_return<T>(out_dir: &Path, name: &str) -> Result<GeneratedSchema>
where
    T: JsonSchema,
{
    let file_stem = name.trim();
    let (raw_namespace, logical_name) = split_namespace(file_stem);
    let include_in_json_codegen =
        raw_namespace != Some("v1") || JSON_V1_ALLOWLIST.contains(&logical_name);
    let schema = schema_for!(T);
    let mut schema_value = serde_json::to_value(schema)?;
    if include_in_json_codegen {
        if file_stem == "ClientRequest" {
            strip_v1_client_request_variants_from_json_schema(&mut schema_value);
        } else if file_stem == "ServerNotification" {
            strip_v1_server_notification_variants_from_json_schema(&mut schema_value);
        }
        enforce_numbered_definition_collision_overrides(file_stem, &mut schema_value);
        annotate_schema(&mut schema_value, Some(file_stem));
    }
    // If the name looks like a namespaced path (e.g., "v2::Type"), mirror
    // the TypeScript layout and write to out_dir/v2/Type.json. Otherwise
    // write alongside the legacy files.
    let out_path = if let Some(ns) = raw_namespace {
        let dir = out_dir.join(ns);
        ensure_dir(&dir)?;
        dir.join(format!("{logical_name}.json"))
    } else {
        out_dir.join(format!("{file_stem}.json"))
    };

    if include_in_json_codegen && !IGNORED_DEFINITIONS.contains(&logical_name) {
        write_pretty_json(out_path, &schema_value)
            .with_context(|| format!("Failed to write JSON schema for {file_stem}"))?;
    }

    let namespace = match raw_namespace {
        Some("v1") | None => None,
        Some(ns) => Some(ns.to_string()),
    };
    Ok(GeneratedSchema {
        in_v1_dir: raw_namespace == Some("v1"),
        namespace,
        logical_name: logical_name.to_string(),
        value: schema_value,
    })
}

fn enforce_numbered_definition_collision_overrides(schema_name: &str, schema: &mut Value) {
    for defs_key in ["definitions", "$defs"] {
        let Some(defs) = schema.get(defs_key).and_then(Value::as_object) else {
            continue;
        };
        detect_numbered_definition_collisions(schema_name, defs_key, defs);
    }
}

fn strip_v1_client_request_variants_from_json_schema(schema: &mut Value) {
    let v1_methods: HashSet<&str> = V1_CLIENT_REQUEST_METHODS.iter().copied().collect();
    strip_method_variants_from_json_schema(schema, &v1_methods);
}

fn strip_v1_server_notification_variants_from_json_schema(schema: &mut Value) {
    let methods: HashSet<&str> = EXCLUDED_SERVER_NOTIFICATION_METHODS_FOR_JSON
        .iter()
        .copied()
        .collect();
    strip_method_variants_from_json_schema(schema, &methods);
}

fn strip_method_variants_from_json_schema(schema: &mut Value, methods_to_remove: &HashSet<&str>) {
    {
        let Some(root) = schema.as_object_mut() else {
            return;
        };
        let Some(Value::Array(variants)) = root.get_mut("oneOf") else {
            return;
        };
        variants.retain(|variant| !is_method_variant_in_set(variant, methods_to_remove));
    }

    let reachable = reachable_local_definitions(schema, "definitions");
    let Some(root) = schema.as_object_mut() else {
        return;
    };
    if let Some(definitions) = root.get_mut("definitions").and_then(Value::as_object_mut) {
        definitions.retain(|name, _| reachable.contains(name));
    }
}

fn is_method_variant_in_set(value: &Value, methods: &HashSet<&str>) -> bool {
    let Value::Object(map) = value else {
        return false;
    };
    let Some(properties) = map.get("properties").and_then(Value::as_object) else {
        return false;
    };
    let Some(method_schema) = properties.get("method") else {
        return false;
    };
    let Some(method) = string_literal(method_schema) else {
        return false;
    };
    methods.contains(method)
}

fn reachable_local_definitions(schema: &Value, defs_key: &str) -> HashSet<String> {
    let Some(definitions) = schema.get(defs_key).and_then(Value::as_object) else {
        return HashSet::new();
    };
    let mut queue: Vec<String> = Vec::new();
    let mut reachable: HashSet<String> = HashSet::new();

    collect_local_definition_refs_excluding_maps(schema, defs_key, &mut queue, &mut reachable);

    while let Some(name) = queue.pop() {
        if let Some(def_schema) = definitions.get(&name) {
            collect_local_definition_refs(def_schema, defs_key, &mut queue, &mut reachable);
        }
    }
    reachable
}

fn collect_local_definition_refs_excluding_maps(
    value: &Value,
    defs_key: &str,
    queue: &mut Vec<String>,
    reachable: &mut HashSet<String>,
) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key == defs_key || key == "$defs" || key == "definitions" {
                    continue;
                }
                collect_local_definition_refs_excluding_maps(child, defs_key, queue, reachable);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_local_definition_refs_excluding_maps(child, defs_key, queue, reachable);
            }
        }
        _ => {}
    }
    collect_local_definition_ref_here(value, defs_key, queue, reachable);
}

fn collect_local_definition_refs(
    value: &Value,
    defs_key: &str,
    queue: &mut Vec<String>,
    reachable: &mut HashSet<String>,
) {
    collect_local_definition_ref_here(value, defs_key, queue, reachable);
    match value {
        Value::Object(map) => {
            for child in map.values() {
                collect_local_definition_refs(child, defs_key, queue, reachable);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_local_definition_refs(child, defs_key, queue, reachable);
            }
        }
        _ => {}
    }
}

fn collect_local_definition_ref_here(
    value: &Value,
    defs_key: &str,
    queue: &mut Vec<String>,
    reachable: &mut HashSet<String>,
) {
    let Some(reference) = value
        .as_object()
        .and_then(|obj| obj.get("$ref"))
        .and_then(Value::as_str)
    else {
        return;
    };
    let Some(name) = reference.strip_prefix(&format!("#/{defs_key}/")) else {
        return;
    };
    let name = name.split('/').next().unwrap_or(name);
    if reachable.insert(name.to_string()) {
        queue.push(name.to_string());
    }
}

fn detect_numbered_definition_collisions(
    schema_name: &str,
    defs_key: &str,
    defs: &Map<String, Value>,
) {
    for generated_name in defs.keys() {
        let base_name = generated_name.trim_end_matches(|c: char| c.is_ascii_digit());
        if base_name == generated_name || !defs.contains_key(base_name) {
            continue;
        }

        panic!(
            "Numbered definition naming collision detected: schema={schema_name}|container={defs_key}|generated={generated_name}|base={base_name}"
        );
    }
}

pub(crate) fn write_json_schema<T>(out_dir: &Path, name: &str) -> Result<GeneratedSchema>
where
    T: JsonSchema,
{
    write_json_schema_with_return::<T>(out_dir, name)
}

pub(crate) fn write_pretty_json(path: PathBuf, value: &impl Serialize) -> Result<()> {
    let json = serde_json::to_vec_pretty(value)
        .with_context(|| format!("Failed to serialize JSON schema to {}", path.display()))?;
    fs::write(&path, json).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use std::collections::BTreeSet;

    #[test]
    fn build_schema_bundle_rewrites_root_helper_refs_to_namespaced_defs() -> Result<()> {
        let bundle = build_schema_bundle(vec![
            GeneratedSchema {
                namespace: None,
                logical_name: "LegacyEnvelope".to_string(),
                in_v1_dir: false,
                value: serde_json::json!({
                    "title": "LegacyEnvelope",
                    "type": "object",
                    "properties": {
                        "current_thread": { "$ref": "#/definitions/ThreadId" },
                        "turn_item": { "$ref": "#/definitions/TurnItem" }
                    },
                    "definitions": {
                        "TurnItem": {
                            "type": "object",
                            "properties": {
                                "thread_id": { "$ref": "#/definitions/ThreadId" },
                                "phase": { "$ref": "#/definitions/MessagePhase" },
                                "content": {
                                    "type": "array",
                                    "items": { "$ref": "#/definitions/UserInput" }
                                }
                            }
                        }
                    }
                }),
            },
            GeneratedSchema {
                namespace: Some("v2".to_string()),
                logical_name: "ThreadId".to_string(),
                in_v1_dir: false,
                value: serde_json::json!({
                    "title": "ThreadId",
                    "type": "string"
                }),
            },
            GeneratedSchema {
                namespace: Some("v2".to_string()),
                logical_name: "MessagePhase".to_string(),
                in_v1_dir: false,
                value: serde_json::json!({
                    "title": "MessagePhase",
                    "type": "string"
                }),
            },
            GeneratedSchema {
                namespace: Some("v2".to_string()),
                logical_name: "UserInput".to_string(),
                in_v1_dir: false,
                value: serde_json::json!({
                    "title": "UserInput",
                    "type": "string"
                }),
            },
        ])?;

        assert_eq!(
            bundle["definitions"]["LegacyEnvelope"]["properties"]["current_thread"]["$ref"],
            serde_json::json!("#/definitions/v2/ThreadId")
        );
        assert_eq!(
            bundle["definitions"]["LegacyEnvelope"]["properties"]["turn_item"]["$ref"],
            serde_json::json!("#/definitions/TurnItem")
        );
        assert_eq!(
            bundle["definitions"]["TurnItem"]["properties"]["thread_id"]["$ref"],
            serde_json::json!("#/definitions/v2/ThreadId")
        );
        assert_eq!(
            bundle["definitions"]["TurnItem"]["properties"]["phase"]["$ref"],
            serde_json::json!("#/definitions/v2/MessagePhase")
        );
        assert_eq!(
            bundle["definitions"]["TurnItem"]["properties"]["content"]["items"]["$ref"],
            serde_json::json!("#/definitions/v2/UserInput")
        );

        Ok(())
    }

    #[test]
    fn build_flat_v2_schema_keeps_shared_root_schemas_and_dependencies() -> Result<()> {
        let bundle = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "CodexAppServerProtocol",
            "type": "object",
            "definitions": {
                "ClientRequest": {
                    "oneOf": [
                        {
                            "title": "StartRequest",
                            "type": "object",
                            "properties": {
                                "params": { "$ref": "#/definitions/v2/ThreadStartParams" },
                                "shared": { "$ref": "#/definitions/SharedHelper" }
                            }
                        },
                        {
                            "title": "InitializeRequest",
                            "type": "object",
                            "properties": {
                                "params": { "$ref": "#/definitions/InitializeParams" }
                            }
                        },
                        {
                            "title": "LogoutRequest",
                            "type": "object",
                            "properties": {
                                "params": { "type": "null" }
                            }
                        }
                    ]
                },
                "EventMsg": {
                    "oneOf": [
                        { "$ref": "#/definitions/v2/ThreadStartedEventMsg" },
                        {
                            "title": "WarningEventMsg",
                            "type": "object",
                            "properties": {
                                "message": { "type": "string" },
                                "type": {
                                    "enum": ["warning"],
                                    "type": "string"
                                }
                            },
                            "required": ["message", "type"]
                        }
                    ]
                },
                "ServerNotification": {
                    "oneOf": [
                        { "$ref": "#/definitions/v2/ThreadStartedNotification" },
                        {
                            "title": "ServerRequestResolvedNotification",
                            "type": "object",
                            "properties": {
                                "params": { "$ref": "#/definitions/ServerRequestResolvedNotificationPayload" }
                            }
                        }
                    ]
                },
                "SharedHelper": {
                    "type": "object",
                    "properties": {
                        "leaf": { "$ref": "#/definitions/SharedLeaf" }
                    }
                },
                "SharedLeaf": {
                    "title": "SharedLeaf",
                    "type": "string"
                },
                "InitializeParams": {
                    "title": "InitializeParams",
                    "type": "string"
                },
                "ServerRequestResolvedNotificationPayload": {
                    "title": "ServerRequestResolvedNotificationPayload",
                    "type": "string"
                },
                "v2": {
                    "ThreadStartParams": {
                        "title": "ThreadStartParams",
                        "type": "object",
                        "properties": {
                            "cwd": { "type": "string" }
                        }
                    },
                    "ThreadStartResponse": {
                        "title": "ThreadStartResponse",
                        "type": "object",
                        "properties": {
                            "ok": { "type": "boolean" }
                        }
                    },
                    "ThreadStartedEventMsg": {
                        "title": "ThreadStartedEventMsg",
                        "type": "object",
                        "properties": {
                            "thread_id": { "type": "string" }
                        }
                    },
                    "ThreadStartedNotification": {
                        "title": "ThreadStartedNotification",
                        "type": "object",
                        "properties": {
                            "thread_id": { "type": "string" }
                        }
                    }
                }
            }
        });

        let flat_bundle = build_flat_v2_schema(&bundle)?;
        let definitions = flat_bundle["definitions"]
            .as_object()
            .expect("flat v2 schema should include definitions");

        assert_eq!(
            flat_bundle["title"],
            serde_json::json!("CodexAppServerProtocolV2")
        );
        assert_eq!(definitions.contains_key("v2"), false);
        assert_eq!(definitions.contains_key("ThreadStartParams"), true);
        assert_eq!(definitions.contains_key("ThreadStartResponse"), true);
        assert_eq!(definitions.contains_key("ThreadStartedNotification"), true);
        assert_eq!(definitions.contains_key("SharedHelper"), true);
        assert_eq!(definitions.contains_key("SharedLeaf"), true);
        assert_eq!(definitions.contains_key("InitializeParams"), true);
        assert_eq!(
            definitions.contains_key("ServerRequestResolvedNotificationPayload"),
            true
        );
        let client_request_titles: BTreeSet<String> = definitions["ClientRequest"]["oneOf"]
            .as_array()
            .expect("ClientRequest should remain a oneOf")
            .iter()
            .map(|variant| {
                variant["title"]
                    .as_str()
                    .expect("ClientRequest variant should have a title")
                    .to_string()
            })
            .collect();
        assert_eq!(
            client_request_titles,
            BTreeSet::from([
                "InitializeRequest".to_string(),
                "LogoutRequest".to_string(),
                "StartRequest".to_string(),
            ])
        );
        let notification_titles: BTreeSet<String> = definitions["ServerNotification"]["oneOf"]
            .as_array()
            .expect("ServerNotification should remain a oneOf")
            .iter()
            .map(|variant| {
                variant
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string()
            })
            .collect();
        assert_eq!(
            notification_titles,
            BTreeSet::from([
                "".to_string(),
                "ServerRequestResolvedNotification".to_string(),
            ])
        );
        assert_eq!(
            first_ref_with_prefix(&flat_bundle, "#/definitions/v2/").is_none(),
            true
        );

        Ok(())
    }
}
