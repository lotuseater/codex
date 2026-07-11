use crate::ClientNotification;
use crate::ClientRequest;
use crate::ServerNotification;
use crate::ServerRequest;
use crate::experimental_api::experimental_fields;
use crate::export_client_notification_schemas;
use crate::export_client_param_schemas;
use crate::export_client_response_schemas;
use crate::export_client_responses;
use crate::export_server_notification_schemas;
use crate::export_server_param_schemas;
use crate::export_server_response_schemas;
use crate::export_server_responses;
use crate::protocol::common::EXPERIMENTAL_CLIENT_METHOD_PARAM_TYPES;
use crate::protocol::common::EXPERIMENTAL_CLIENT_METHOD_RESPONSE_TYPES;
use crate::protocol::common::EXPERIMENTAL_CLIENT_METHODS;
use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use codex_protocol::protocol::RolloutLine;
use schemars::JsonSchema;
use schemars::schema_for;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use ts_rs::TS;

mod annotate;
mod experimental;
mod schema_bundle;
mod ts_files;
mod ts_parse;

use annotate::*;
use experimental::*;
use schema_bundle::*;
use ts_files::*;
use ts_parse::*;

// Preserve the existing `codex_app_server_protocol::export::*` /
// `crate::export::*` paths for items that moved into submodules but are
// referenced from sibling modules of `export` (e.g. `schema_fixtures`,
// `protocol::common`).
pub(crate) use experimental::filter_experimental_ts_tree;
pub(crate) use schema_bundle::write_json_schema;
pub(crate) use ts_files::generate_index_ts_tree;
pub(crate) use ts_files::trim_trailing_line_whitespace;

pub(crate) const GENERATED_TS_HEADER: &str = "// GENERATED CODE! DO NOT MODIFY BY HAND!\n\n";
const IGNORED_DEFINITIONS: &[&str] = &["Option<()>"];
const JSON_V1_ALLOWLIST: &[&str] = &["InitializeParams", "InitializeResponse"];
const EXPERIMENTAL_CLIENT_METHOD_DEPENDENCY_TYPES: &[&str] = &[
    "EnvironmentShellInfo",
    "PathUri",
    "RemoteControlClient",
    "RemoteControlClientsListOrder",
    "ThreadBackgroundTerminal",
];
const SPECIAL_DEFINITIONS: &[&str] = &[
    "ClientNotification",
    "ClientRequest",
    "ServerNotification",
    "ServerRequest",
];
const FLAT_V2_SHARED_DEFINITIONS: &[&str] = &["ClientRequest", "ServerNotification"];
const V1_CLIENT_REQUEST_METHODS: &[&str] =
    &["getConversationSummary", "gitDiffToRemote", "getAuthStatus"];
const EXCLUDED_SERVER_NOTIFICATION_METHODS_FOR_JSON: &[&str] = &["rawResponseItem/completed"];

#[derive(Clone)]
pub struct GeneratedSchema {
    namespace: Option<String>,
    logical_name: String,
    value: Value,
    in_v1_dir: bool,
}

impl GeneratedSchema {
    fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    fn logical_name(&self) -> &str {
        &self.logical_name
    }

    fn value(&self) -> &Value {
        &self.value
    }
}

type JsonSchemaEmitter = fn(&Path) -> Result<GeneratedSchema>;
pub fn generate_types(out_dir: &Path, prettier: Option<&Path>) -> Result<()> {
    generate_ts(out_dir, prettier)?;
    generate_json(out_dir)?;
    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub struct GenerateTsOptions {
    pub generate_indices: bool,
    pub ensure_headers: bool,
    pub run_prettier: bool,
    pub experimental_api: bool,
}

impl Default for GenerateTsOptions {
    fn default() -> Self {
        Self {
            generate_indices: true,
            ensure_headers: true,
            run_prettier: true,
            experimental_api: false,
        }
    }
}

pub fn generate_ts(out_dir: &Path, prettier: Option<&Path>) -> Result<()> {
    generate_ts_with_options(out_dir, prettier, GenerateTsOptions::default())
}

pub fn generate_ts_with_options(
    out_dir: &Path,
    prettier: Option<&Path>,
    options: GenerateTsOptions,
) -> Result<()> {
    let v2_out_dir = out_dir.join("v2");
    ensure_dir(out_dir)?;
    ensure_dir(&v2_out_dir)?;

    ClientRequest::export_all_to(out_dir)?;
    export_client_responses(out_dir)?;
    ClientNotification::export_all_to(out_dir)?;

    ServerRequest::export_all_to(out_dir)?;
    export_server_responses(out_dir)?;
    ServerNotification::export_all_to(out_dir)?;

    if !options.experimental_api {
        filter_experimental_ts(out_dir)?;
    }

    if options.generate_indices {
        generate_index_ts(out_dir)?;
        generate_index_ts(&v2_out_dir)?;
    }

    // Ensure our header is present on all TS files (root + subdirs like v2/).
    let ts_files = ts_files_in_recursive(out_dir)?;

    if options.ensure_headers {
        let worker_count = thread::available_parallelism()
            .map_or(1, usize::from)
            .min(ts_files.len().max(1));
        let chunk_size = ts_files.len().div_ceil(worker_count);
        thread::scope(|scope| -> Result<()> {
            let mut workers = Vec::new();
            for chunk in ts_files.chunks(chunk_size.max(1)) {
                workers.push(scope.spawn(move || -> Result<()> {
                    for file in chunk {
                        prepend_header_if_missing(file)?;
                    }
                    Ok(())
                }));
            }

            for worker in workers {
                worker
                    .join()
                    .map_err(|_| anyhow!("TypeScript header worker panicked"))??;
            }

            Ok(())
        })?;
    }

    // Optionally run Prettier on all generated TS files.
    if options.run_prettier
        && let Some(prettier_bin) = prettier
        && !ts_files.is_empty()
    {
        let status = Command::new(prettier_bin)
            .arg("--write")
            .arg("--log-level")
            .arg("warn")
            .args(ts_files.iter().map(|p| p.as_os_str()))
            .status()
            .with_context(|| format!("Failed to invoke Prettier at {}", prettier_bin.display()))?;
        if !status.success() {
            return Err(anyhow!("Prettier failed with status {status}"));
        }
    }

    trim_trailing_whitespace_in_ts_files(&ts_files)?;

    Ok(())
}

pub fn generate_json(out_dir: &Path) -> Result<()> {
    generate_json_with_experimental(out_dir, /*experimental_api*/ false)
}

pub fn generate_internal_json_schema(out_dir: &Path) -> Result<()> {
    ensure_dir(out_dir)?;
    write_json_schema::<RolloutLine>(out_dir, "RolloutLine")?;
    Ok(())
}

pub fn generate_json_with_experimental(out_dir: &Path, experimental_api: bool) -> Result<()> {
    ensure_dir(out_dir)?;
    let envelope_emitters: Vec<JsonSchemaEmitter> = vec![
        |d| write_json_schema_with_return::<crate::RequestId>(d, "RequestId"),
        |d| write_json_schema_with_return::<crate::JSONRPCMessage>(d, "JSONRPCMessage"),
        |d| write_json_schema_with_return::<crate::JSONRPCRequest>(d, "JSONRPCRequest"),
        |d| write_json_schema_with_return::<crate::JSONRPCNotification>(d, "JSONRPCNotification"),
        |d| write_json_schema_with_return::<crate::JSONRPCResponse>(d, "JSONRPCResponse"),
        |d| write_json_schema_with_return::<crate::JSONRPCError>(d, "JSONRPCError"),
        |d| write_json_schema_with_return::<crate::JSONRPCErrorError>(d, "JSONRPCErrorError"),
        |d| write_json_schema_with_return::<crate::ClientRequest>(d, "ClientRequest"),
        |d| write_json_schema_with_return::<crate::ServerRequest>(d, "ServerRequest"),
        |d| write_json_schema_with_return::<crate::ClientNotification>(d, "ClientNotification"),
        |d| write_json_schema_with_return::<crate::ServerNotification>(d, "ServerNotification"),
    ];

    let mut schemas: Vec<GeneratedSchema> = Vec::new();
    for emit in &envelope_emitters {
        schemas.push(emit(out_dir)?);
    }

    schemas.extend(export_client_param_schemas(out_dir)?);
    schemas.extend(export_client_response_schemas(out_dir)?);
    schemas.extend(export_server_param_schemas(out_dir)?);
    schemas.extend(export_server_response_schemas(out_dir)?);
    schemas.extend(export_client_notification_schemas(out_dir)?);
    schemas.extend(export_server_notification_schemas(out_dir)?);
    schemas
        .retain(|schema| !schema.in_v1_dir || JSON_V1_ALLOWLIST.contains(&schema.logical_name()));

    let mut bundle = build_schema_bundle(schemas)?;
    if !experimental_api {
        filter_experimental_schema(&mut bundle)?;
    }
    write_pretty_json(
        out_dir.join("codex_app_server_protocol.schemas.json"),
        &bundle,
    )?;
    let flat_v2_bundle = build_flat_v2_schema(&bundle)?;
    write_pretty_json(
        out_dir.join("codex_app_server_protocol.v2.schemas.json"),
        &flat_v2_bundle,
    )?;

    if !experimental_api {
        filter_experimental_json_files(out_dir)?;
    }

    Ok(())
}

fn ensure_dir(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)
        .with_context(|| format!("Failed to create output directory {}", dir.display()))
}
