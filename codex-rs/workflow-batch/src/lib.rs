use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Context as _;
use anyhow::bail;
use regex::Regex;
use serde::Serialize;
use serde_json::Map;
use serde_json::Number;
use serde_json::Value;
use sha1::Digest;
use sha1::Sha1;

#[derive(Debug, Clone, Serialize)]
pub struct StepRecord {
    pub id: String,
    pub status: String,
    pub elapsed_ms: u128,
    pub rc: Option<i32>,
    pub stdout_digest: Option<String>,
    pub stdout_preview: Option<String>,
    pub stderr_preview: Option<String>,
    pub note: Option<String>,
    pub iteration: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowSummary {
    pub name: Option<String>,
    pub status: String,
    pub spec: String,
    pub log: String,
    pub vars: BTreeMap<String, Value>,
    pub steps_total: usize,
    pub steps_failed: usize,
    pub steps_skipped: usize,
    pub steps: Vec<StepRecord>,
}

#[derive(Debug, Clone)]
pub struct WorkflowOptions {
    root: Option<PathBuf>,
    allow_commands: bool,
    allow_file_mutation_steps: bool,
    allow_external_paths: bool,
}

impl WorkflowOptions {
    pub fn unrestricted() -> Self {
        Self {
            root: None,
            allow_commands: true,
            allow_file_mutation_steps: true,
            allow_external_paths: true,
        }
    }

    pub fn unrestricted_with_root(root: impl Into<PathBuf>) -> Self {
        Self {
            root: Some(root.into()),
            allow_commands: true,
            allow_file_mutation_steps: true,
            allow_external_paths: true,
        }
    }

    pub fn root_confined(root: impl Into<PathBuf>) -> Self {
        Self {
            root: Some(root.into()),
            allow_commands: false,
            allow_file_mutation_steps: true,
            allow_external_paths: false,
        }
    }

    pub fn context_tool(root: impl Into<PathBuf>) -> Self {
        Self {
            root: Some(root.into()),
            allow_commands: false,
            allow_file_mutation_steps: false,
            allow_external_paths: false,
        }
    }

    fn resolved_root(&self, _spec_path: &Path) -> anyhow::Result<PathBuf> {
        if let Some(root) = self.root.as_deref() {
            if root.is_absolute() {
                Ok(root.to_path_buf())
            } else {
                root.canonicalize()
                    .with_context(|| format!("failed to resolve root for {}", root.display()))
            }
        } else {
            std::env::current_dir().context("failed to resolve current directory")
        }
    }

    fn ensure_path_allowed(&self, root: &Path, path: &Path, label: &str) -> anyhow::Result<()> {
        if self.allow_external_paths {
            return Ok(());
        }
        if path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            bail!(
                "{label} {} must not contain parent-directory components",
                path.display()
            );
        }
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        };
        if !absolute.starts_with(root) {
            bail!(
                "{label} {} is outside workflow root {}",
                path.display(),
                root.display()
            );
        }
        Ok(())
    }
}

impl Default for WorkflowOptions {
    fn default() -> Self {
        Self::unrestricted()
    }
}

struct WorkflowContext {
    spec: String,
    log_path: PathBuf,
    root: PathBuf,
    options: WorkflowOptions,
    vars: BTreeMap<String, Value>,
    steps: BTreeMap<String, Value>,
    records: Vec<StepRecord>,
    failed: bool,
}

struct TempVars {
    previous: Vec<(String, Option<Value>)>,
}

pub fn run_workflow(
    spec_path: &Path,
    report_path: &Path,
    log_path: &Path,
) -> anyhow::Result<WorkflowSummary> {
    let root = std::env::current_dir().context("failed to resolve current directory")?;
    run_workflow_with_options(
        spec_path,
        report_path,
        log_path,
        WorkflowOptions::unrestricted_with_root(root),
    )
}

pub fn run_workflow_with_options(
    spec_path: &Path,
    report_path: &Path,
    log_path: &Path,
    options: WorkflowOptions,
) -> anyhow::Result<WorkflowSummary> {
    let root = options.resolved_root(spec_path)?;
    options.ensure_path_allowed(&root, spec_path, "spec path")?;
    options.ensure_path_allowed(&root, report_path, "report path")?;
    options.ensure_path_allowed(&root, log_path, "log path")?;

    let spec_text = fs::read_to_string(spec_path)
        .with_context(|| format!("failed to read {}", spec_path.display()))?;
    let spec: Value = serde_json::from_str(&spec_text)
        .with_context(|| format!("failed to parse {}", spec_path.display()))?;

    run_workflow_value(
        spec,
        spec_path.to_string_lossy().to_string(),
        report_path,
        log_path,
        root,
        options,
    )
}

pub fn run_workflow_value_with_options(
    spec: Value,
    report_path: &Path,
    log_path: &Path,
    options: WorkflowOptions,
) -> anyhow::Result<WorkflowSummary> {
    let root = options.resolved_root(Path::new(""))?;
    options.ensure_path_allowed(&root, report_path, "report path")?;
    options.ensure_path_allowed(&root, log_path, "log path")?;
    run_workflow_value(
        spec,
        "<inline>".to_string(),
        report_path,
        log_path,
        root,
        options,
    )
}

fn run_workflow_value(
    spec: Value,
    spec_label: String,
    report_path: &Path,
    log_path: &Path,
    root: PathBuf,
    options: WorkflowOptions,
) -> anyhow::Result<WorkflowSummary> {
    ensure_parent(report_path)?;
    ensure_parent(log_path)?;
    if log_path.exists() {
        fs::remove_file(log_path)
            .with_context(|| format!("failed to remove old log {}", log_path.display()))?;
    }

    let mut ctx = WorkflowContext {
        spec: spec_label,
        log_path: log_path.to_path_buf(),
        root,
        options,
        vars: object_to_btree(spec.get("vars").and_then(Value::as_object)),
        steps: BTreeMap::new(),
        records: Vec::new(),
        failed: false,
    };

    let name = spec
        .get("name")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    ctx.log(json_object([
        ("event", Value::String("workflow_start".to_string())),
        ("name", option_string_value(name.as_deref())),
        ("spec", Value::String(ctx.spec.clone())),
    ]))?;

    let steps = spec
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Err(error) = execute_steps(&mut ctx, &steps, "", None) {
        ctx.failed = true;
        ctx.log(json_object([
            ("event", Value::String("workflow_error".to_string())),
            ("error", Value::String(error.to_string())),
        ]))?;
    }

    let summary = WorkflowSummary {
        name,
        status: if ctx.failed { "failed" } else { "ok" }.to_string(),
        spec: ctx.spec,
        log: log_path.to_string_lossy().to_string(),
        vars: ctx.vars,
        steps_total: ctx.records.len(),
        steps_failed: ctx
            .records
            .iter()
            .filter(|record| record.status == "failed")
            .count(),
        steps_skipped: ctx
            .records
            .iter()
            .filter(|record| record.status == "skipped")
            .count(),
        steps: ctx.records,
    };

    fs::write(
        report_path,
        format!("{}\n", serde_json::to_string_pretty(&summary)?),
    )
    .with_context(|| format!("failed to write {}", report_path.display()))?;
    Ok(summary)
}

fn execute_steps(
    ctx: &mut WorkflowContext,
    steps: &[Value],
    prefix: &str,
    iteration: Option<usize>,
) -> anyhow::Result<()> {
    for (index, step_value) in steps.iter().enumerate() {
        let step = step_value
            .as_object()
            .with_context(|| format!("step {} is not an object", index + 1))?;
        let raw_id = step
            .get("id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("step_{}", index + 1));
        let step_id = format!("{prefix}{raw_id}");

        if step.contains_key("then") || step.contains_key("else") {
            let branch_is_then = should_run(ctx, step)?;
            let branch_steps = if branch_is_then {
                step.get("then")
            } else {
                step.get("else")
            }
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
            execute_steps(ctx, &branch_steps, &format!("{step_id}."), iteration)?;
            let branch = if branch_is_then { "then" } else { "else" };
            ctx.steps.insert(
                step_id.clone(),
                json_object([
                    ("status", Value::String("ok".to_string())),
                    ("branch", Value::String(branch.to_string())),
                ]),
            );
            ctx.records.push(StepRecord {
                id: step_id,
                status: "ok".to_string(),
                elapsed_ms: 0,
                rc: None,
                stdout_digest: None,
                stdout_preview: None,
                stderr_preview: None,
                note: Some(branch.to_string()),
                iteration,
            });
            continue;
        }

        if !should_run(ctx, step)? {
            ctx.steps.insert(
                step_id.clone(),
                json_object([("status", Value::String("skipped".to_string()))]),
            );
            ctx.records.push(StepRecord {
                id: step_id.clone(),
                status: "skipped".to_string(),
                elapsed_ms: 0,
                rc: None,
                stdout_digest: None,
                stdout_preview: None,
                stderr_preview: None,
                note: None,
                iteration,
            });
            ctx.log(json_object([
                ("event", Value::String("skipped".to_string())),
                ("id", Value::String(step_id)),
                ("iteration", option_usize_value(iteration)),
            ]))?;
            continue;
        }

        if let Some(values) = step
            .get("set")
            .or_else(|| step.get("set_vars"))
            .and_then(Value::as_object)
        {
            set_vars(ctx, &step_id, values, iteration)?;
        } else if step.contains_key("run") {
            if !ctx.options.allow_commands {
                reject_disallowed_step(ctx, &step_id, iteration, "run")?;
            }
            run_command(ctx, &step_id, step, iteration)?;
        } else if step.contains_key("copy_file") {
            if !ctx.options.allow_file_mutation_steps {
                reject_disallowed_step(ctx, &step_id, iteration, "copy_file")?;
            }
            copy_file(ctx, &step_id, step, iteration)?;
        } else if step.contains_key("write_file") {
            if !ctx.options.allow_file_mutation_steps {
                reject_disallowed_step(ctx, &step_id, iteration, "write_file")?;
            }
            write_file(ctx, &step_id, step, iteration, WriteMode::Overwrite)?;
        } else if step.contains_key("append_file") {
            if !ctx.options.allow_file_mutation_steps {
                reject_disallowed_step(ctx, &step_id, iteration, "append_file")?;
            }
            write_file(ctx, &step_id, step, iteration, WriteMode::Append)?;
        } else if step.contains_key("ensure_dir") {
            if !ctx.options.allow_file_mutation_steps {
                reject_disallowed_step(ctx, &step_id, iteration, "ensure_dir")?;
            }
            ensure_dir(ctx, &step_id, step, iteration)?;
        } else if step.contains_key("edit_file") {
            if !ctx.options.allow_file_mutation_steps {
                reject_disallowed_step(ctx, &step_id, iteration, "edit_file")?;
            }
            edit_file(ctx, &step_id, step, iteration)?;
        } else if step.contains_key("read_file") {
            read_file(ctx, &step_id, step, iteration)?;
        } else if step.contains_key("read_json") {
            read_json(ctx, &step_id, step, iteration)?;
        } else if step.contains_key("stat_path") {
            stat_path(ctx, &step_id, step, iteration)?;
        } else if step.contains_key("list_files") {
            list_files(ctx, &step_id, step, iteration)?;
        } else if step.contains_key("write_json") {
            if !ctx.options.allow_file_mutation_steps {
                reject_disallowed_step(ctx, &step_id, iteration, "write_json")?;
            }
            write_json(ctx, &step_id, step, iteration)?;
        } else if step.contains_key("assert") {
            assert_step(ctx, &step_id, step, iteration)?;
        } else if step.contains_key("for_each") {
            for_each(ctx, &step_id, step, iteration)?;
        } else if step.contains_key("while") {
            run_while(ctx, &step_id, step)?;
        } else {
            bail!("`{step_id}` has no supported action");
        }
    }
    Ok(())
}

fn reject_disallowed_step(
    ctx: &mut WorkflowContext,
    step_id: &str,
    iteration: Option<usize>,
    action: &str,
) -> anyhow::Result<()> {
    let note = format!("{action} steps are disabled for this workflow execution mode");
    ctx.failed = true;
    ctx.steps.insert(
        step_id.to_string(),
        json_object([
            ("status", Value::String("failed".to_string())),
            ("error", Value::String(note.clone())),
        ]),
    );
    ctx.records.push(StepRecord {
        id: step_id.to_string(),
        status: "failed".to_string(),
        elapsed_ms: 0,
        rc: None,
        stdout_digest: None,
        stdout_preview: None,
        stderr_preview: None,
        note: Some(note.clone()),
        iteration,
    });
    ctx.log(json_object([
        ("event", Value::String("failed".to_string())),
        ("id", Value::String(step_id.to_string())),
        ("action", Value::String(action.to_string())),
        ("error", Value::String(note.clone())),
        ("iteration", option_usize_value(iteration)),
    ]))?;
    bail!("{note}");
}

fn run_while(
    ctx: &mut WorkflowContext,
    step_id: &str,
    step: &Map<String, Value>,
) -> anyhow::Result<()> {
    let max_iterations = step
        .get("max_iterations")
        .and_then(Value::as_u64)
        .unwrap_or(10) as usize;
    let body = step
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let condition = step
        .get("while")
        .with_context(|| format!("`{step_id}` missing while condition"))?;
    let mut count = 0;
    while truthy(&eval_expr(ctx, condition)?) {
        if count >= max_iterations {
            ctx.failed = true;
            bail!("`{step_id}` exceeded max_iterations={max_iterations}");
        }
        count += 1;
        ctx.log(json_object([
            ("event", Value::String("loop_iteration".to_string())),
            ("id", Value::String(step_id.to_string())),
            ("iteration", Value::Number(Number::from(count))),
        ]))?;
        execute_steps(ctx, &body, &format!("{step_id}[{count}]."), Some(count))?;
    }
    ctx.steps.insert(
        step_id.to_string(),
        json_object([
            ("status", Value::String("ok".to_string())),
            ("iterations", Value::Number(Number::from(count))),
        ]),
    );
    ctx.records.push(StepRecord {
        id: step_id.to_string(),
        status: "ok".to_string(),
        elapsed_ms: 0,
        rc: None,
        stdout_digest: None,
        stdout_preview: None,
        stderr_preview: None,
        note: Some(format!("iterations={count}")),
        iteration: None,
    });
    Ok(())
}

fn for_each(
    ctx: &mut WorkflowContext,
    step_id: &str,
    step: &Map<String, Value>,
    iteration: Option<usize>,
) -> anyhow::Result<()> {
    let items_value = step
        .get("for_each")
        .with_context(|| format!("`{step_id}` missing for_each expression"))?;
    let items = eval_expr(ctx, items_value)?
        .as_array()
        .cloned()
        .with_context(|| format!("`{step_id}` for_each did not evaluate to an array"))?;
    let item_var = step
        .get("as")
        .and_then(Value::as_str)
        .unwrap_or("item")
        .to_string();
    let index_var = step
        .get("index_as")
        .and_then(Value::as_str)
        .unwrap_or("index")
        .to_string();
    let body = step
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    ctx.log(json_object([
        ("event", Value::String("for_each_start".to_string())),
        ("id", Value::String(step_id.to_string())),
        ("items", Value::Array(items.clone())),
        ("iteration", option_usize_value(iteration)),
    ]))?;

    for (index, item) in items.iter().cloned().enumerate() {
        let snapshot = apply_temp_vars(
            ctx,
            [
                (item_var.clone(), item),
                (index_var.clone(), Value::Number(Number::from(index))),
            ],
        );
        let result = execute_steps(ctx, &body, &format!("{step_id}[{index}]."), Some(index));
        restore_temp_vars(ctx, snapshot);
        result?;
    }

    ctx.steps.insert(
        step_id.to_string(),
        json_object([
            ("status", Value::String("ok".to_string())),
            ("items", Value::Number(Number::from(items.len()))),
        ]),
    );
    ctx.records.push(StepRecord {
        id: step_id.to_string(),
        status: "ok".to_string(),
        elapsed_ms: 0,
        rc: None,
        stdout_digest: None,
        stdout_preview: None,
        stderr_preview: None,
        note: Some(format!("items={}", items.len())),
        iteration,
    });
    ctx.log(json_object([
        ("event", Value::String("for_each_done".to_string())),
        ("id", Value::String(step_id.to_string())),
        ("items", Value::Number(Number::from(items.len()))),
        ("iteration", option_usize_value(iteration)),
    ]))?;
    Ok(())
}

fn set_vars(
    ctx: &mut WorkflowContext,
    step_id: &str,
    values: &Map<String, Value>,
    iteration: Option<usize>,
) -> anyhow::Result<()> {
    let mut changed = Map::new();
    let mut pending = values
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<Vec<_>>();
    while !pending.is_empty() {
        let mut next = Vec::new();
        let mut made_progress = false;
        let mut last_error: Option<anyhow::Error> = None;
        for (key, value) in pending {
            match eval_expr(ctx, &value) {
                Ok(evaluated) => {
                    ctx.vars.insert(key.clone(), evaluated.clone());
                    changed.insert(key, evaluated);
                    made_progress = true;
                }
                Err(error) => {
                    last_error = Some(error);
                    next.push((key, value));
                }
            }
        }
        if !made_progress && let Some((key, _)) = next.first() {
            return Err(last_error
                .unwrap_or_else(|| anyhow::anyhow!("unknown evaluation failure"))
                .context(format!("failed to evaluate `{key}` in `{step_id}`")));
        }
        pending = next;
    }
    ctx.steps.insert(
        step_id.to_string(),
        json_object([
            ("status", Value::String("ok".to_string())),
            ("changed", Value::Object(changed.clone())),
        ]),
    );
    ctx.log(json_object([
        ("event", Value::String("set".to_string())),
        ("id", Value::String(step_id.to_string())),
        ("changed", Value::Object(changed.clone())),
        ("iteration", option_usize_value(iteration)),
    ]))?;
    let mut keys = changed.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    ctx.records.push(StepRecord {
        id: step_id.to_string(),
        status: "ok".to_string(),
        elapsed_ms: 0,
        rc: None,
        stdout_digest: None,
        stdout_preview: None,
        stderr_preview: None,
        note: Some(format!("set {keys:?}")),
        iteration,
    });
    Ok(())
}

fn run_command(
    ctx: &mut WorkflowContext,
    step_id: &str,
    step: &Map<String, Value>,
    iteration: Option<usize>,
) -> anyhow::Result<()> {
    let argv = step
        .get("run")
        .and_then(Value::as_array)
        .with_context(|| format!("`{step_id}` run must be an array"))?
        .iter()
        .map(|item| render_value(ctx, item).map(|value| value_to_string(&value)))
        .collect::<anyhow::Result<Vec<_>>>()?;
    if argv.is_empty() {
        bail!("`{step_id}` has empty argv");
    }
    let cwd = step
        .get("cwd")
        .map(|value| resolve_input_path(ctx, value))
        .transpose()?
        .unwrap_or_else(|| ctx.root.clone());
    let timeout = Duration::from_secs(
        step.get("timeout_sec")
            .and_then(Value::as_u64)
            .unwrap_or(30),
    );
    let fail_on_nonzero = step
        .get("fail_on_nonzero")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    ctx.log(json_object([
        ("event", Value::String("run_start".to_string())),
        ("id", Value::String(step_id.to_string())),
        (
            "argv",
            Value::Array(argv.iter().cloned().map(Value::String).collect()),
        ),
        ("cwd", Value::String(cwd.to_string_lossy().to_string())),
        ("iteration", option_usize_value(iteration)),
    ]))?;

    let started = Instant::now();
    let output = run_with_timeout(&argv, &cwd, timeout)
        .with_context(|| format!("failed to run `{step_id}`"))?;
    let elapsed_ms = started.elapsed().as_millis();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let (stdout_digest, stdout_preview) = compact_text(&stdout);
    let (_stderr_digest, stderr_preview) = compact_text(&stderr);
    let status = if output.rc == 0 || !fail_on_nonzero {
        "ok"
    } else {
        "failed"
    };
    let note = if output.timed_out {
        Some("timed out".to_string())
    } else if output.rc != 0 && !fail_on_nonzero {
        Some("nonzero tolerated".to_string())
    } else {
        None
    };

    ctx.steps.insert(
        step_id.to_string(),
        json_object([
            ("status", Value::String(status.to_string())),
            ("rc", Value::Number(Number::from(output.rc))),
            ("stdout", Value::String(stdout.trim().to_string())),
            ("stderr", Value::String(stderr.trim().to_string())),
            (
                "elapsed_ms",
                Value::Number(Number::from(u64::try_from(elapsed_ms).unwrap_or(u64::MAX))),
            ),
        ]),
    );
    ctx.log(json_object([
        ("event", Value::String("run_done".to_string())),
        ("id", Value::String(step_id.to_string())),
        ("rc", Value::Number(Number::from(output.rc))),
        (
            "elapsed_ms",
            Value::Number(Number::from(u64::try_from(elapsed_ms).unwrap_or(u64::MAX))),
        ),
        ("stdout", Value::String(stdout)),
        ("stderr", Value::String(stderr)),
        ("iteration", option_usize_value(iteration)),
    ]))?;

    ctx.records.push(StepRecord {
        id: step_id.to_string(),
        status: status.to_string(),
        elapsed_ms,
        rc: Some(output.rc),
        stdout_digest: stdout_digest.filter(|digest| !digest.is_empty()),
        stdout_preview,
        stderr_preview,
        note,
        iteration,
    });

    if output.rc != 0 && fail_on_nonzero {
        ctx.failed = true;
        bail!("`{step_id}` exited with {}", output.rc);
    }
    Ok(())
}

enum WriteMode {
    Overwrite,
    Append,
}

fn copy_file(
    ctx: &mut WorkflowContext,
    step_id: &str,
    step: &Map<String, Value>,
    iteration: Option<usize>,
) -> anyhow::Result<()> {
    let spec = step
        .get("copy_file")
        .and_then(Value::as_object)
        .with_context(|| format!("`{step_id}` copy_file must be an object"))?;
    let from = resolve_input_path(
        ctx,
        spec.get("from")
            .with_context(|| format!("`{step_id}` copy_file missing from"))?,
    )?;
    let to = resolve_output_path(
        ctx,
        spec.get("to")
            .with_context(|| format!("`{step_id}` copy_file missing to"))?,
    )?;
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let bytes = fs::copy(&from, &to)
        .with_context(|| format!("failed to copy {} to {}", from.display(), to.display()))?;
    ctx.steps.insert(
        step_id.to_string(),
        json_object([
            ("status", Value::String("ok".to_string())),
            ("from", Value::String(from.to_string_lossy().to_string())),
            ("path", Value::String(to.to_string_lossy().to_string())),
            ("bytes", Value::Number(Number::from(bytes))),
        ]),
    );
    ctx.log(json_object([
        ("event", Value::String("copy_file".to_string())),
        ("id", Value::String(step_id.to_string())),
        ("from", Value::String(from.to_string_lossy().to_string())),
        ("path", Value::String(to.to_string_lossy().to_string())),
        ("bytes", Value::Number(Number::from(bytes))),
        ("iteration", option_usize_value(iteration)),
    ]))?;
    ctx.records.push(StepRecord {
        id: step_id.to_string(),
        status: "ok".to_string(),
        elapsed_ms: 0,
        rc: None,
        stdout_digest: None,
        stdout_preview: None,
        stderr_preview: None,
        note: Some(format!("bytes={bytes}")),
        iteration,
    });
    Ok(())
}

fn write_file(
    ctx: &mut WorkflowContext,
    step_id: &str,
    step: &Map<String, Value>,
    iteration: Option<usize>,
    mode: WriteMode,
) -> anyhow::Result<()> {
    let key = match mode {
        WriteMode::Overwrite => "write_file",
        WriteMode::Append => "append_file",
    };
    let spec = step
        .get(key)
        .and_then(Value::as_object)
        .with_context(|| format!("`{step_id}` {key} must be an object"))?;
    let path = resolve_output_path(
        ctx,
        spec.get("path")
            .with_context(|| format!("`{step_id}` {key} missing path"))?,
    )?;
    let content = spec
        .get("content")
        .map(|value| eval_expr(ctx, value).map(|value| value_to_string(&value)))
        .transpose()?
        .unwrap_or_default();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    match mode {
        WriteMode::Overwrite => fs::write(&path, &content)?,
        WriteMode::Append => {
            let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
            file.write_all(content.as_bytes())?;
        }
    }
    let bytes = content.len();
    ctx.steps.insert(
        step_id.to_string(),
        json_object([
            ("status", Value::String("ok".to_string())),
            ("path", Value::String(path.to_string_lossy().to_string())),
            ("bytes", Value::Number(Number::from(bytes))),
        ]),
    );
    ctx.log(json_object([
        ("event", Value::String(key.to_string())),
        ("id", Value::String(step_id.to_string())),
        ("path", Value::String(path.to_string_lossy().to_string())),
        ("bytes", Value::Number(Number::from(bytes))),
        ("iteration", option_usize_value(iteration)),
    ]))?;
    ctx.records.push(StepRecord {
        id: step_id.to_string(),
        status: "ok".to_string(),
        elapsed_ms: 0,
        rc: None,
        stdout_digest: None,
        stdout_preview: None,
        stderr_preview: None,
        note: Some(format!("bytes={bytes}")),
        iteration,
    });
    Ok(())
}

fn edit_file(
    ctx: &mut WorkflowContext,
    step_id: &str,
    step: &Map<String, Value>,
    iteration: Option<usize>,
) -> anyhow::Result<()> {
    let spec = step
        .get("edit_file")
        .and_then(Value::as_object)
        .with_context(|| format!("`{step_id}` edit_file must be an object"))?;
    let path = resolve_output_path(
        ctx,
        spec.get("path")
            .with_context(|| format!("`{step_id}` edit_file missing path"))?,
    )?;
    let operations = spec
        .get("operations")
        .and_then(Value::as_array)
        .with_context(|| format!("`{step_id}` edit_file missing operations"))?;
    let mut text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let before_len = text.len();
    let mut changes = 0usize;

    for (index, operation) in operations.iter().enumerate() {
        let operation = operation
            .as_object()
            .with_context(|| format!("edit operation {} is not an object", index + 1))?;
        changes += apply_edit_operation(ctx, &mut text, operation)
            .with_context(|| format!("failed edit operation {} in `{step_id}`", index + 1))?;
    }

    fs::write(&path, &text).with_context(|| format!("failed to write {}", path.display()))?;
    ctx.steps.insert(
        step_id.to_string(),
        json_object([
            ("status", Value::String("ok".to_string())),
            ("path", Value::String(path.to_string_lossy().to_string())),
            ("changes", Value::Number(Number::from(changes))),
            ("bytes_before", Value::Number(Number::from(before_len))),
            ("bytes_after", Value::Number(Number::from(text.len()))),
        ]),
    );
    ctx.log(json_object([
        ("event", Value::String("edit_file".to_string())),
        ("id", Value::String(step_id.to_string())),
        ("path", Value::String(path.to_string_lossy().to_string())),
        ("changes", Value::Number(Number::from(changes))),
        ("iteration", option_usize_value(iteration)),
    ]))?;
    ctx.records.push(StepRecord {
        id: step_id.to_string(),
        status: "ok".to_string(),
        elapsed_ms: 0,
        rc: None,
        stdout_digest: None,
        stdout_preview: None,
        stderr_preview: None,
        note: Some(format!("changes={changes}")),
        iteration,
    });
    Ok(())
}

fn apply_edit_operation(
    ctx: &mut WorkflowContext,
    text: &mut String,
    operation: &Map<String, Value>,
) -> anyhow::Result<usize> {
    let op = operation
        .get("op")
        .and_then(Value::as_str)
        .context("edit operation missing op")?;
    let required = operation
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    match op {
        "insert_at_line" => {
            let line = operation
                .get("line")
                .map(|value| eval_expr(ctx, value).and_then(|value| value_to_i64(&value)))
                .transpose()?
                .unwrap_or(1);
            let content = edit_content(ctx, operation)?;
            let offset = line_start_offset(text, line);
            text.insert_str(offset, &content);
            Ok(1)
        }
        "insert_at_position" => {
            let line = required_i64(ctx, operation, "line")?;
            let column = required_i64(ctx, operation, "column")?;
            let content = edit_content(ctx, operation)?;
            let offset = line_column_offset(text, line, column)?;
            text.insert_str(offset, &content);
            Ok(1)
        }
        "replace_span" => {
            let start_line = required_i64(ctx, operation, "start_line")?;
            let start_column = required_i64(ctx, operation, "start_column")?;
            let end_line = required_i64(ctx, operation, "end_line")?;
            let end_column = required_i64(ctx, operation, "end_column")?;
            let content = edit_content(ctx, operation)?;
            let start = line_column_offset(text, start_line, start_column)?;
            let end = line_column_offset(text, end_line, end_column)?;
            if start > end {
                bail!("replace_span start must be before end");
            }
            text.replace_range(start..end, &content);
            Ok(1)
        }
        "insert_before" | "insert_after" => {
            let pattern = edit_pattern(ctx, operation)?;
            let regex = operation
                .get("regex")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let count = edit_count(ctx, operation)?;
            let content = edit_content(ctx, operation)?;
            let mut ranges = match_ranges(text, &pattern, regex)?;
            if count > 0 {
                ranges.truncate(count);
            }
            if ranges.is_empty() && required {
                bail!("pattern `{pattern}` did not match");
            }
            let changed = ranges.len();
            for (start, end) in ranges.into_iter().rev() {
                let offset = if op == "insert_before" { start } else { end };
                text.insert_str(offset, &content);
            }
            Ok(changed)
        }
        "replace" => {
            let pattern = edit_pattern(ctx, operation)?;
            let regex = operation
                .get("regex")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let count = edit_count(ctx, operation)?;
            let content = edit_content(ctx, operation)?;
            let matches = match_ranges(text, &pattern, regex)?.len();
            if matches == 0 && required {
                bail!("pattern `{pattern}` did not match");
            }
            let effective_count = if count == 0 {
                matches
            } else {
                matches.min(count)
            };
            let replaced = if regex {
                let re = Regex::new(&pattern)?;
                if count == 0 {
                    re.replace_all(text, content.as_str()).to_string()
                } else {
                    re.replacen(text, count, content.as_str()).to_string()
                }
            } else if count == 0 {
                text.replace(&pattern, &content)
            } else {
                text.replacen(&pattern, &content, count)
            };
            *text = replaced;
            Ok(effective_count)
        }
        _ => bail!("unsupported edit op `{op}`"),
    }
}

fn edit_pattern(
    ctx: &mut WorkflowContext,
    operation: &Map<String, Value>,
) -> anyhow::Result<String> {
    operation
        .get("pattern")
        .map(|value| eval_expr(ctx, value).map(|value| value_to_string(&value)))
        .transpose()?
        .context("edit operation missing pattern")
}

fn edit_content(
    ctx: &mut WorkflowContext,
    operation: &Map<String, Value>,
) -> anyhow::Result<String> {
    operation
        .get("content")
        .map(|value| eval_expr(ctx, value).map(|value| value_to_string(&value)))
        .transpose()
        .map(std::option::Option::unwrap_or_default)
}

fn edit_count(ctx: &mut WorkflowContext, operation: &Map<String, Value>) -> anyhow::Result<usize> {
    let count = operation
        .get("count")
        .map(|value| eval_expr(ctx, value).and_then(|value| value_to_i64(&value)))
        .transpose()?
        .unwrap_or(1);
    Ok(usize::try_from(count.max(0)).unwrap_or(usize::MAX))
}

fn line_start_offset(text: &str, line: i64) -> usize {
    if line <= 1 {
        return 0;
    }
    let mut current_line = 1;
    for (index, ch) in text.char_indices() {
        if ch == '\n' {
            current_line += 1;
            if current_line == line {
                return index + 1;
            }
        }
    }
    text.len()
}

fn line_column_offset(text: &str, line: i64, column: i64) -> anyhow::Result<usize> {
    if line < 1 || column < 1 {
        bail!("line and column are 1-based");
    }
    let line_start = line_start_offset(text, line);
    let mut current_column = 1;
    for (relative, ch) in text[line_start..].char_indices() {
        if current_column == column {
            return Ok(line_start + relative);
        }
        if ch == '\n' {
            break;
        }
        current_column += 1;
    }
    if current_column == column {
        return Ok(text.len());
    }
    bail!("column {column} is past line {line}");
}

fn required_i64(
    ctx: &mut WorkflowContext,
    object: &Map<String, Value>,
    key: &str,
) -> anyhow::Result<i64> {
    object
        .get(key)
        .with_context(|| format!("missing {key}"))
        .and_then(|value| eval_expr(ctx, value))
        .and_then(|value| value_to_i64(&value))
}

fn match_ranges(text: &str, pattern: &str, regex: bool) -> anyhow::Result<Vec<(usize, usize)>> {
    if regex {
        return Ok(Regex::new(pattern)?
            .find_iter(text)
            .map(|matched| (matched.start(), matched.end()))
            .collect());
    }
    Ok(text
        .match_indices(pattern)
        .map(|(start, matched)| (start, start + matched.len()))
        .collect())
}

fn ensure_dir(
    ctx: &mut WorkflowContext,
    step_id: &str,
    step: &Map<String, Value>,
    iteration: Option<usize>,
) -> anyhow::Result<()> {
    let spec = step
        .get("ensure_dir")
        .and_then(Value::as_object)
        .with_context(|| format!("`{step_id}` ensure_dir must be an object"))?;
    let path = resolve_output_path(
        ctx,
        spec.get("path")
            .with_context(|| format!("`{step_id}` ensure_dir missing path"))?,
    )?;
    fs::create_dir_all(&path)
        .with_context(|| format!("failed to create directory {}", path.display()))?;
    ctx.steps.insert(
        step_id.to_string(),
        json_object([
            ("status", Value::String("ok".to_string())),
            ("path", Value::String(path.to_string_lossy().to_string())),
        ]),
    );
    ctx.log(json_object([
        ("event", Value::String("ensure_dir".to_string())),
        ("id", Value::String(step_id.to_string())),
        ("path", Value::String(path.to_string_lossy().to_string())),
        ("iteration", option_usize_value(iteration)),
    ]))?;
    ctx.records.push(StepRecord {
        id: step_id.to_string(),
        status: "ok".to_string(),
        elapsed_ms: 0,
        rc: None,
        stdout_digest: None,
        stdout_preview: None,
        stderr_preview: None,
        note: Some("created directory".to_string()),
        iteration,
    });
    Ok(())
}

fn read_file(
    ctx: &mut WorkflowContext,
    step_id: &str,
    step: &Map<String, Value>,
    iteration: Option<usize>,
) -> anyhow::Result<()> {
    let spec = step
        .get("read_file")
        .and_then(Value::as_object)
        .with_context(|| format!("`{step_id}` read_file must be an object"))?;
    let path = resolve_input_path(
        ctx,
        spec.get("path")
            .with_context(|| format!("`{step_id}` read_file missing path"))?,
    )?;
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    if let Some(var) = spec.get("var").and_then(Value::as_str) {
        ctx.vars
            .insert(var.to_string(), Value::String(content.clone()));
    }
    let (digest, preview) = compact_text(&content);
    ctx.steps.insert(
        step_id.to_string(),
        json_object([
            ("status", Value::String("ok".to_string())),
            ("path", Value::String(path.to_string_lossy().to_string())),
            ("stdout", Value::String(content)),
        ]),
    );
    ctx.log(json_object([
        ("event", Value::String("read_file".to_string())),
        ("id", Value::String(step_id.to_string())),
        ("path", Value::String(path.to_string_lossy().to_string())),
        ("iteration", option_usize_value(iteration)),
    ]))?;
    ctx.records.push(StepRecord {
        id: step_id.to_string(),
        status: "ok".to_string(),
        elapsed_ms: 0,
        rc: None,
        stdout_digest: digest.filter(|digest| !digest.is_empty()),
        stdout_preview: preview,
        stderr_preview: None,
        note: None,
        iteration,
    });
    Ok(())
}

fn read_json(
    ctx: &mut WorkflowContext,
    step_id: &str,
    step: &Map<String, Value>,
    iteration: Option<usize>,
) -> anyhow::Result<()> {
    let spec = step
        .get("read_json")
        .and_then(Value::as_object)
        .with_context(|| format!("`{step_id}` read_json must be an object"))?;
    let path = resolve_input_path(
        ctx,
        spec.get("path")
            .with_context(|| format!("`{step_id}` read_json missing path"))?,
    )?;
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let parsed: Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse JSON from {}", path.display()))?;
    if let Some(var) = spec.get("var").and_then(Value::as_str) {
        ctx.vars.insert(var.to_string(), parsed.clone());
    }
    ctx.steps.insert(
        step_id.to_string(),
        json_object([
            ("status", Value::String("ok".to_string())),
            ("path", Value::String(path.to_string_lossy().to_string())),
            ("value", parsed),
        ]),
    );
    ctx.log(json_object([
        ("event", Value::String("read_json".to_string())),
        ("id", Value::String(step_id.to_string())),
        ("path", Value::String(path.to_string_lossy().to_string())),
        ("iteration", option_usize_value(iteration)),
    ]))?;
    ctx.records.push(StepRecord {
        id: step_id.to_string(),
        status: "ok".to_string(),
        elapsed_ms: 0,
        rc: None,
        stdout_digest: None,
        stdout_preview: None,
        stderr_preview: None,
        note: None,
        iteration,
    });
    Ok(())
}

fn stat_path(
    ctx: &mut WorkflowContext,
    step_id: &str,
    step: &Map<String, Value>,
    iteration: Option<usize>,
) -> anyhow::Result<()> {
    let spec = step
        .get("stat_path")
        .and_then(Value::as_object)
        .with_context(|| format!("`{step_id}` stat_path must be an object"))?;
    let path = resolve_input_path(
        ctx,
        spec.get("path")
            .with_context(|| format!("`{step_id}` stat_path missing path"))?,
    )?;
    let include_sha1 = spec.get("sha1").and_then(Value::as_bool).unwrap_or(false);
    let value = path_status_value(ctx, &path, include_sha1)?;
    if let Some(var) = spec.get("var").and_then(Value::as_str) {
        ctx.vars.insert(var.to_string(), value.clone());
    }
    ctx.steps.insert(
        step_id.to_string(),
        json_object([
            ("status", Value::String("ok".to_string())),
            ("path", Value::String(path.to_string_lossy().to_string())),
            ("value", value.clone()),
        ]),
    );
    ctx.log(json_object([
        ("event", Value::String("stat_path".to_string())),
        ("id", Value::String(step_id.to_string())),
        ("path", Value::String(path.to_string_lossy().to_string())),
        ("iteration", option_usize_value(iteration)),
    ]))?;
    ctx.records.push(StepRecord {
        id: step_id.to_string(),
        status: "ok".to_string(),
        elapsed_ms: 0,
        rc: None,
        stdout_digest: None,
        stdout_preview: Some(value_to_string(&value)),
        stderr_preview: None,
        note: None,
        iteration,
    });
    Ok(())
}

fn list_files(
    ctx: &mut WorkflowContext,
    step_id: &str,
    step: &Map<String, Value>,
    iteration: Option<usize>,
) -> anyhow::Result<()> {
    let spec = step
        .get("list_files")
        .and_then(Value::as_object)
        .with_context(|| format!("`{step_id}` list_files must be an object"))?;
    let path = if let Some(path) = spec.get("path") {
        resolve_input_path(ctx, path)?
    } else {
        ctx.root.clone()
    };
    let recursive = spec
        .get("recursive")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let include_dirs = spec
        .get("include_dirs")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let details = spec
        .get("details")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let pattern = spec
        .get("pattern")
        .and_then(Value::as_str)
        .map(Regex::new)
        .transpose()?;
    let max_entries = spec
        .get("max_entries")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(1000)
        .min(10_000);

    let mut entries = Vec::new();
    collect_list_entries(
        ctx,
        &path,
        recursive,
        include_dirs,
        details,
        pattern.as_ref(),
        max_entries,
        &mut entries,
    )?;
    let value = Value::Array(entries);
    if let Some(var) = spec.get("var").and_then(Value::as_str) {
        ctx.vars.insert(var.to_string(), value.clone());
    }
    ctx.steps.insert(
        step_id.to_string(),
        json_object([
            ("status", Value::String("ok".to_string())),
            ("path", Value::String(path.to_string_lossy().to_string())),
            ("value", value.clone()),
        ]),
    );
    ctx.log(json_object([
        ("event", Value::String("list_files".to_string())),
        ("id", Value::String(step_id.to_string())),
        ("path", Value::String(path.to_string_lossy().to_string())),
        ("iteration", option_usize_value(iteration)),
    ]))?;
    let (stdout_digest, stdout_preview) = compact_value(&value);
    ctx.records.push(StepRecord {
        id: step_id.to_string(),
        status: "ok".to_string(),
        elapsed_ms: 0,
        rc: None,
        stdout_digest,
        stdout_preview,
        stderr_preview: None,
        note: Some(format!("entries={}", value.as_array().map_or(0, Vec::len))),
        iteration,
    });
    Ok(())
}

fn write_json(
    ctx: &mut WorkflowContext,
    step_id: &str,
    step: &Map<String, Value>,
    iteration: Option<usize>,
) -> anyhow::Result<()> {
    let spec = step
        .get("write_json")
        .and_then(Value::as_object)
        .with_context(|| format!("`{step_id}` write_json must be an object"))?;
    let path = resolve_output_path(
        ctx,
        spec.get("path")
            .with_context(|| format!("`{step_id}` write_json missing path"))?,
    )?;
    let value = eval_expr(
        ctx,
        spec.get("value")
            .with_context(|| format!("`{step_id}` write_json missing value"))?,
    )?;
    let pretty = spec.get("pretty").and_then(Value::as_bool).unwrap_or(true);
    let mut content = if pretty {
        serde_json::to_string_pretty(&value)?
    } else {
        serde_json::to_string(&value)?
    };
    content.push('\n');
    ensure_parent(&path)?;
    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))?;
    ctx.steps.insert(
        step_id.to_string(),
        json_object([
            ("status", Value::String("ok".to_string())),
            ("path", Value::String(path.to_string_lossy().to_string())),
            ("value", value),
        ]),
    );
    ctx.log(json_object([
        ("event", Value::String("write_json".to_string())),
        ("id", Value::String(step_id.to_string())),
        ("path", Value::String(path.to_string_lossy().to_string())),
        ("iteration", option_usize_value(iteration)),
    ]))?;
    ctx.records.push(StepRecord {
        id: step_id.to_string(),
        status: "ok".to_string(),
        elapsed_ms: 0,
        rc: None,
        stdout_digest: None,
        stdout_preview: None,
        stderr_preview: None,
        note: None,
        iteration,
    });
    Ok(())
}

fn assert_step(
    ctx: &mut WorkflowContext,
    step_id: &str,
    step: &Map<String, Value>,
    iteration: Option<usize>,
) -> anyhow::Result<()> {
    let condition = step
        .get("assert")
        .with_context(|| format!("`{step_id}` assert missing expression"))?;
    let assert_spec = assert_spec(ctx, step_id, step, condition)?;
    let value = eval_assert_condition(ctx, assert_spec.condition)?;
    if !truthy(&value) {
        let message = if let Some(message) = assert_spec.message {
            message
        } else {
            format!("assertion `{step_id}` failed")
        };
        bail!("{message}");
    }
    ctx.steps.insert(
        step_id.to_string(),
        json_object([
            ("status", Value::String("ok".to_string())),
            ("value", value.clone()),
        ]),
    );
    ctx.log(json_object([
        ("event", Value::String("assert".to_string())),
        ("id", Value::String(step_id.to_string())),
        ("iteration", option_usize_value(iteration)),
    ]))?;
    let (stdout_digest, stdout_preview) = compact_value(&value);
    ctx.records.push(StepRecord {
        id: step_id.to_string(),
        status: "ok".to_string(),
        elapsed_ms: 0,
        rc: None,
        stdout_digest,
        stdout_preview,
        stderr_preview: None,
        note: None,
        iteration,
    });
    Ok(())
}

struct AssertSpec<'a> {
    condition: &'a Value,
    message: Option<String>,
}

fn assert_spec<'a>(
    ctx: &WorkflowContext,
    step_id: &str,
    step: &'a Map<String, Value>,
    condition: &'a Value,
) -> anyhow::Result<AssertSpec<'a>> {
    let step_message = step
        .get("message")
        .map(|message| render_value(ctx, message).map(|value| value_to_string(&value)))
        .transpose()?;

    if let Some(spec) = condition.as_object()
        && let Some(condition) = spec.get("expr").or_else(|| spec.get("condition"))
    {
        let message = spec
            .get("message")
            .map(|message| render_value(ctx, message).map(|value| value_to_string(&value)))
            .transpose()?
            .or(step_message);
        return Ok(AssertSpec { condition, message });
    }

    if condition.as_object().is_some_and(|spec| {
        spec.contains_key("expr") || spec.contains_key("condition") || spec.contains_key("message")
    }) {
        bail!("`{step_id}` assert object must include `expr` or `condition`");
    }

    Ok(AssertSpec {
        condition,
        message: step_message,
    })
}

fn eval_assert_condition(ctx: &mut WorkflowContext, condition: &Value) -> anyhow::Result<Value> {
    if let Some(condition) = condition.as_str()
        && let Some(value) = eval_assert_string_condition(ctx, condition)?
    {
        return Ok(value);
    }
    eval_expr(ctx, condition)
}

fn eval_assert_string_condition(
    ctx: &WorkflowContext,
    condition: &str,
) -> anyhow::Result<Option<Value>> {
    for op in ["==", "!="] {
        if let Some((left, right)) = split_assert_infix(condition, op) {
            let left = eval_assert_term(ctx, left)?;
            let right = eval_assert_term(ctx, right)?;
            let result = match op {
                "==" => left == right,
                "!=" => left != right,
                _ => unreachable!("operator list is fixed"),
            };
            return Ok(Some(Value::Bool(result)));
        }
    }
    Ok(None)
}

fn split_assert_infix<'a>(condition: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
    let mut quote = None;
    let mut escaped = false;
    let indices = condition.char_indices().peekable();
    for (index, ch) in indices {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            }
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
            continue;
        }
        if condition[index..].starts_with(op) {
            return Some((
                condition[..index].trim(),
                condition[index + op.len()..].trim(),
            ));
        }
    }
    None
}

fn eval_assert_term(ctx: &WorkflowContext, term: &str) -> anyhow::Result<Value> {
    if let Some(value) = quoted_assert_string(term) {
        return Ok(Value::String(value));
    }
    if let Ok(value) = serde_json::from_str::<Value>(term) {
        return Ok(value);
    }
    if let Some(value) = ctx.vars.get(term) {
        return Ok(value.clone());
    }
    resolve_ref(ctx, term).or_else(|_| render_template(ctx, term).map(Value::String))
}

fn quoted_assert_string(term: &str) -> Option<String> {
    let mut chars = term.chars();
    let quote = chars.next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    if !term.ends_with(quote) || term.len() < 2 {
        return None;
    }
    let inner = &term[quote.len_utf8()..term.len() - quote.len_utf8()];
    let mut out = String::with_capacity(inner.len());
    let mut escaped = false;
    for ch in inner.chars() {
        if escaped {
            let resolved = match ch {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '\\' => '\\',
                '\'' => '\'',
                '"' => '"',
                other => other,
            };
            out.push(resolved);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else {
            out.push(ch);
        }
    }
    if escaped {
        out.push('\\');
    }
    Some(out)
}

fn collect_list_entries(
    ctx: &WorkflowContext,
    path: &Path,
    recursive: bool,
    include_dirs: bool,
    details: bool,
    pattern: Option<&Regex>,
    max_entries: usize,
    entries: &mut Vec<Value>,
) -> anyhow::Result<()> {
    if entries.len() >= max_entries {
        return Ok(());
    }

    let metadata =
        fs::metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
    if metadata.is_file() {
        push_list_entry(ctx, path, &metadata, details, pattern, max_entries, entries)?;
        return Ok(());
    }
    if !metadata.is_dir() {
        if include_dirs {
            push_list_entry(ctx, path, &metadata, details, pattern, max_entries, entries)?;
        }
        return Ok(());
    }

    if include_dirs {
        push_list_entry(ctx, path, &metadata, details, pattern, max_entries, entries)?;
    }

    let mut children = fs::read_dir(path)
        .with_context(|| format!("failed to list {}", path.display()))?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("failed to list {}", path.display()))?;
    children.sort_by_key(std::fs::DirEntry::path);
    for entry in children {
        if entries.len() >= max_entries {
            break;
        }
        let child_path = entry.path();
        let child_metadata = entry
            .metadata()
            .with_context(|| format!("failed to stat {}", child_path.display()))?;
        if child_metadata.is_file() || include_dirs && child_metadata.is_dir() {
            push_list_entry(
                ctx,
                &child_path,
                &child_metadata,
                details,
                pattern,
                max_entries,
                entries,
            )?;
        }
        if recursive && child_metadata.is_dir() {
            collect_list_entries(
                ctx,
                &child_path,
                recursive,
                include_dirs,
                details,
                pattern,
                max_entries,
                entries,
            )?;
        }
    }
    Ok(())
}

fn push_list_entry(
    ctx: &WorkflowContext,
    path: &Path,
    metadata: &fs::Metadata,
    details: bool,
    pattern: Option<&Regex>,
    max_entries: usize,
    entries: &mut Vec<Value>,
) -> anyhow::Result<()> {
    if entries.len() >= max_entries {
        return Ok(());
    }
    let relative = relative_path_string(ctx, path);
    if pattern.is_some_and(|pattern| !pattern.is_match(&relative)) {
        return Ok(());
    }
    if details {
        entries.push(path_metadata_value(ctx, path, metadata, false)?);
    } else {
        entries.push(Value::String(relative));
    }
    Ok(())
}

fn path_status_value(
    ctx: &WorkflowContext,
    path: &Path,
    include_sha1: bool,
) -> anyhow::Result<Value> {
    match fs::metadata(path) {
        Ok(metadata) => path_metadata_value(ctx, path, &metadata, include_sha1),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(json_object([
            ("exists", Value::Bool(false)),
            ("path", Value::String(relative_path_string(ctx, path))),
            ("kind", Value::String("missing".to_string())),
            ("is_file", Value::Bool(false)),
            ("is_dir", Value::Bool(false)),
            ("len", Value::Null),
            ("modified_unix", Value::Null),
        ])),
        Err(error) => Err(error).with_context(|| format!("failed to stat {}", path.display())),
    }
}

fn path_metadata_value(
    ctx: &WorkflowContext,
    path: &Path,
    metadata: &fs::Metadata,
    include_sha1: bool,
) -> anyhow::Result<Value> {
    let kind = if metadata.is_file() {
        "file"
    } else if metadata.is_dir() {
        "dir"
    } else {
        "other"
    };
    let mut object = Map::from_iter([
        ("exists".to_string(), Value::Bool(true)),
        (
            "path".to_string(),
            Value::String(relative_path_string(ctx, path)),
        ),
        ("kind".to_string(), Value::String(kind.to_string())),
        ("is_file".to_string(), Value::Bool(metadata.is_file())),
        ("is_dir".to_string(), Value::Bool(metadata.is_dir())),
        (
            "len".to_string(),
            Value::Number(Number::from(metadata.len())),
        ),
        (
            "modified_unix".to_string(),
            metadata
                .modified()
                .ok()
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                .map(|duration| Value::Number(Number::from(duration.as_secs())))
                .unwrap_or(Value::Null),
        ),
    ]);
    if include_sha1 && metadata.is_file() {
        let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
        object.insert("sha1".to_string(), Value::String(hex_sha1_bytes(&bytes)));
    }
    Ok(Value::Object(object))
}

fn relative_path_string(ctx: &WorkflowContext, path: &Path) -> String {
    let relative = path.strip_prefix(&ctx.root).unwrap_or(path);
    if relative.as_os_str().is_empty() {
        ".".to_string()
    } else {
        relative.to_string_lossy().replace('\\', "/")
    }
}

fn resolve_input_path(ctx: &WorkflowContext, value: &Value) -> anyhow::Result<PathBuf> {
    let rendered = value_to_string(&render_value(ctx, value)?);
    let path = PathBuf::from(rendered);
    ctx.options
        .ensure_path_allowed(&ctx.root, &path, "read source")?;
    Ok(if path.is_absolute() {
        path
    } else {
        ctx.root.join(path)
    })
}

fn resolve_output_path(ctx: &WorkflowContext, value: &Value) -> anyhow::Result<PathBuf> {
    let rendered = value_to_string(&render_value(ctx, value)?);
    let path = PathBuf::from(rendered);
    ctx.options
        .ensure_path_allowed(&ctx.root, &path, "write destination")?;
    let path = if path.is_absolute() {
        path
    } else {
        ctx.root.join(path)
    };
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        bail!(
            "write destination {} must not contain parent-directory components",
            path.display()
        );
    }
    if !path.starts_with(&ctx.root) {
        bail!(
            "write destination {} is outside workflow root {}",
            path.display(),
            ctx.root.display()
        );
    }
    Ok(path)
}

struct CommandOutput {
    rc: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
}

fn run_with_timeout(
    argv: &[String],
    cwd: &Path,
    timeout: Duration,
) -> anyhow::Result<CommandOutput> {
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {}", argv[0]))?;
    let started = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            let output = child.wait_with_output()?;
            return Ok(CommandOutput {
                rc: output.status.code().unwrap_or(1),
                stdout: output.stdout,
                stderr: output.stderr,
                timed_out: false,
            });
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let output = child.wait_with_output()?;
            return Ok(CommandOutput {
                rc: 124,
                stdout: output.stdout,
                stderr: output.stderr,
                timed_out: true,
            });
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn eval_expr(ctx: &mut WorkflowContext, expr: &Value) -> anyhow::Result<Value> {
    match expr {
        Value::Null | Value::Bool(_) | Value::Number(_) => Ok(expr.clone()),
        Value::String(value) => render_template(ctx, value).map(Value::String),
        Value::Array(values) => values
            .iter()
            .map(|value| eval_expr(ctx, value))
            .collect::<anyhow::Result<Vec<_>>>()
            .map(Value::Array),
        Value::Object(object) => eval_object_expr(ctx, object),
    }
}

fn eval_object_expr(ctx: &mut WorkflowContext, expr: &Map<String, Value>) -> anyhow::Result<Value> {
    if let Some(value) = expr.get("literal") {
        return Ok(value.clone());
    }
    if let Some(value) = expr.get("ref") {
        return resolve_ref(ctx, value_to_string(value).as_str());
    }
    if let Some(value) = expr.get("lines") {
        return Ok(Value::Array(
            value_to_string(&eval_expr(ctx, value)?)
                .lines()
                .map(|line| Value::String(line.to_string()))
                .collect(),
        ));
    }
    if let Some(value) = expr.get("strip") {
        return Ok(Value::String(
            value_to_string(&eval_expr(ctx, value)?).trim().to_string(),
        ));
    }
    if let Some(value) = expr.get("len") {
        let evaluated = eval_expr(ctx, value)?;
        return Ok(Value::Number(Number::from(value_len(&evaluated))));
    }
    if let Some(value) = expr.get("range") {
        return eval_range(ctx, value);
    }
    if let Some(value) = expr.get("split") {
        return eval_split(ctx, value);
    }
    if let Some(value) = expr.get("sort") {
        return eval_sort(ctx, value);
    }
    if let Some(value) = expr.get("unique") {
        return eval_unique(ctx, value);
    }
    if let Some(value) = expr.get("take") {
        return eval_take(ctx, value);
    }
    if let Some(value) = expr.get("get") {
        return eval_get(ctx, value);
    }
    if let Some(value) = expr.get("join") {
        return eval_join(ctx, value);
    }
    if let Some(value) = expr.get("map") {
        return eval_map(ctx, value);
    }
    if let Some(value) = expr.get("filter") {
        return eval_filter(ctx, value);
    }
    if let Some(value) = expr.get("reduce") {
        return eval_reduce(ctx, value);
    }
    if let Some(value) = expr.get("all_of") {
        return eval_quantifier(ctx, value, Quantifier::All);
    }
    if let Some(value) = expr.get("any_of") {
        return eval_quantifier(ctx, value, Quantifier::Any);
    }
    if let Some(value) = expr.get("none_of") {
        return eval_quantifier(ctx, value, Quantifier::None);
    }
    if let Some(value) = expr.get("count_if") {
        return eval_count_if(ctx, value);
    }
    if let Some(value) = expr.get("find_if") {
        return eval_find_if(ctx, value);
    }
    if let Some(value) = expr.get("partition") {
        return eval_partition(ctx, value);
    }
    if let Some(value) = expr.get("group_by") {
        return eval_group_by(ctx, value);
    }
    if let Some(value) = expr.get("scan") {
        return eval_scan(ctx, value);
    }
    if let Some(value) = expr.get("set_union") {
        return eval_set_union(ctx, value);
    }
    if let Some(value) = expr.get("set_intersection") {
        return eval_set_intersection(ctx, value);
    }
    if let Some(value) = expr.get("set_difference") {
        return eval_set_difference(ctx, value);
    }
    if let Some(value) = expr.get("set_includes") {
        return eval_set_includes(ctx, value);
    }
    if let Some(value) = expr.get("min") {
        return eval_min_max(ctx, value, MinMax::Min);
    }
    if let Some(value) = expr.get("max") {
        return eval_min_max(ctx, value, MinMax::Max);
    }
    if let Some(value) = expr.get("enumerate") {
        return eval_enumerate(ctx, value);
    }
    if let Some(value) = expr.get("zip") {
        return eval_zip(ctx, value);
    }
    if let Some(value) = expr.get("parse_json") {
        let text = value_to_string(&eval_expr(ctx, value)?);
        return serde_json::from_str(&text).context("parse_json expression failed");
    }
    if let Some(value) = expr.get("to_json") {
        return serde_json::to_string(&eval_expr(ctx, value)?)
            .map(Value::String)
            .context("to_json expression failed");
    }
    if let Some(value) = expr.get("keys") {
        return eval_keys(ctx, value);
    }
    if let Some(value) = expr.get("values") {
        return eval_values(ctx, value);
    }
    if let Some(value) = expr.get("entries") {
        return eval_entries(ctx, value);
    }
    if let Some(value) = expr.get("from_entries") {
        return eval_from_entries(ctx, value);
    }
    if let Some(value) = expr.get("merge") {
        return eval_merge(ctx, value);
    }
    if let Some(value) = expr.get("pick") {
        return eval_pick_omit(ctx, value, PickOmit::Pick);
    }
    if let Some(value) = expr.get("omit") {
        return eval_pick_omit(ctx, value, PickOmit::Omit);
    }
    if let Some(value) = expr.get("not") {
        return Ok(Value::Bool(!truthy(&eval_expr(ctx, value)?)));
    }
    if let Some(value) = expr.get("and") {
        let values = value
            .as_array()
            .context("and expression must be an array")?;
        for item in values {
            if !truthy(&eval_expr(ctx, item)?) {
                return Ok(Value::Bool(false));
            }
        }
        return Ok(Value::Bool(true));
    }
    if let Some(value) = expr.get("or") {
        let values = value.as_array().context("or expression must be an array")?;
        for item in values {
            if truthy(&eval_expr(ctx, item)?) {
                return Ok(Value::Bool(true));
            }
        }
        return Ok(Value::Bool(false));
    }
    if let Some(value) = expr.get("contains") {
        let (haystack, needle) = eval_pair(ctx, value, "contains")?;
        return Ok(Value::Bool(
            value_to_string(&haystack).contains(&value_to_string(&needle)),
        ));
    }
    if let Some(value) = expr.get("starts_with") {
        let (haystack, needle) = eval_pair(ctx, value, "starts_with")?;
        return Ok(Value::Bool(
            value_to_string(&haystack).starts_with(&value_to_string(&needle)),
        ));
    }
    if let Some(value) = expr.get("ends_with") {
        let (haystack, needle) = eval_pair(ctx, value, "ends_with")?;
        return Ok(Value::Bool(
            value_to_string(&haystack).ends_with(&value_to_string(&needle)),
        ));
    }
    if let Some(value) = expr.get("matches") {
        let (text, pattern) = eval_pair(ctx, value, "matches")?;
        let re = Regex::new(&value_to_string(&pattern))?;
        return Ok(Value::Bool(re.is_match(&value_to_string(&text))));
    }
    for op in ["eq", "ne", "lt", "lte", "gt", "gte", "add", "sub"] {
        if let Some(value) = expr.get(op) {
            let (left, right) = eval_pair(ctx, value, op)?;
            return eval_binary(op, left, right);
        }
    }
    bail!("unknown expression operator: {expr:?}");
}

fn eval_range(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let args = value
        .as_array()
        .context("range expression must be an array")?
        .iter()
        .map(|item| value_to_i64(&eval_expr(ctx, item)?))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let (start, stop, step) = match args.as_slice() {
        [stop] => (0, *stop, 1),
        [start, stop] => (*start, *stop, 1),
        [start, stop, step] => (*start, *stop, *step),
        _ => bail!("range expects 1 to 3 arguments"),
    };
    if step == 0 {
        bail!("range step cannot be 0");
    }
    let mut out = Vec::new();
    let mut current = start;
    while (step > 0 && current < stop) || (step < 0 && current > stop) {
        out.push(Value::Number(Number::from(current)));
        current += step;
    }
    Ok(Value::Array(out))
}

fn eval_split(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let (text, sep) = eval_pair(ctx, value, "split")?;
    let text = value_to_string(&text);
    let sep = value_to_string(&sep);
    let items = if sep.is_empty() {
        text.chars()
            .map(|ch| Value::String(ch.to_string()))
            .collect()
    } else {
        text.split(&sep)
            .map(|item| Value::String(item.to_string()))
            .collect()
    };
    Ok(Value::Array(items))
}

fn eval_sort(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let mut items = eval_expr(ctx, value)?
        .as_array()
        .cloned()
        .context("sort expression must evaluate to an array")?;
    items.sort_by_key(value_to_string);
    Ok(Value::Array(items))
}

fn eval_unique(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let items = eval_expr(ctx, value)?
        .as_array()
        .cloned()
        .context("unique expression must evaluate to an array")?;
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for item in items {
        if seen.insert(value_to_string(&item)) {
            out.push(item);
        }
    }
    Ok(Value::Array(out))
}

fn eval_take(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let (items, count) = eval_pair(ctx, value, "take")?;
    let items = items
        .as_array()
        .cloned()
        .context("take items must be an array")?;
    let count = usize::try_from(value_to_i64(&count)?.max(0)).unwrap_or(usize::MAX);
    Ok(Value::Array(items.into_iter().take(count).collect()))
}

fn eval_get(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let spec = value
        .as_object()
        .context("get expression must be an object")?;
    let container = eval_expr(
        ctx,
        spec.get("from").context("get expression missing from")?,
    )?;
    let key = eval_expr(ctx, spec.get("key").context("get expression missing key")?)?;
    let fallback = spec
        .get("default")
        .map(|value| eval_expr(ctx, value))
        .transpose()?;
    let resolved = if let Some(index) = key.as_i64() {
        container
            .as_array()
            .and_then(|items| {
                usize::try_from(index)
                    .ok()
                    .and_then(|index| items.get(index))
            })
            .cloned()
    } else {
        container
            .as_object()
            .and_then(|object| object.get(&value_to_string(&key)))
            .cloned()
    };
    Ok(resolved.or(fallback).unwrap_or(Value::Null))
}

fn eval_join(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let spec = value
        .as_object()
        .context("join expression must be an object")?;
    let items = eval_expr(
        ctx,
        spec.get("items").context("join expression missing items")?,
    )?
    .as_array()
    .cloned()
    .context("join items must be an array")?;
    let sep = spec
        .get("sep")
        .map(|value| eval_expr(ctx, value).map(|evaluated| value_to_string(&evaluated)))
        .transpose()?
        .unwrap_or_default();
    Ok(Value::String(
        items
            .iter()
            .map(value_to_string)
            .collect::<Vec<_>>()
            .join(&sep),
    ))
}

fn eval_map(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let spec = value
        .as_object()
        .context("map expression must be an object")?;
    let var = spec.get("as").and_then(Value::as_str).unwrap_or("item");
    let items = eval_expr(
        ctx,
        spec.get("items").context("map expression missing items")?,
    )?
    .as_array()
    .cloned()
    .context("map items must be an array")?;
    let expr = spec.get("expr").context("map expression missing expr")?;
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let snapshot = apply_temp_vars(ctx, [(var.to_string(), item)]);
        let evaluated = eval_expr(ctx, expr);
        restore_temp_vars(ctx, snapshot);
        out.push(evaluated?);
    }
    Ok(Value::Array(out))
}

fn eval_filter(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let spec = value
        .as_object()
        .context("filter expression must be an object")?;
    let var = spec.get("as").and_then(Value::as_str).unwrap_or("item");
    let items = eval_expr(
        ctx,
        spec.get("items")
            .context("filter expression missing items")?,
    )?
    .as_array()
    .cloned()
    .context("filter items must be an array")?;
    let condition = spec.get("if").context("filter expression missing if")?;
    let mut out = Vec::new();
    for item in items {
        let snapshot = apply_temp_vars(ctx, [(var.to_string(), item.clone())]);
        let keep = eval_expr(ctx, condition).map(|value| truthy(&value));
        restore_temp_vars(ctx, snapshot);
        if keep? {
            out.push(item);
        }
    }
    Ok(Value::Array(out))
}

fn eval_reduce(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let spec = value
        .as_object()
        .context("reduce expression must be an object")?;
    let acc_var = spec.get("acc").and_then(Value::as_str).unwrap_or("acc");
    let item_var = spec.get("as").and_then(Value::as_str).unwrap_or("item");
    let items = eval_expr(
        ctx,
        spec.get("items")
            .context("reduce expression missing items")?,
    )?
    .as_array()
    .cloned()
    .context("reduce items must be an array")?;
    let mut acc = eval_expr(
        ctx,
        spec.get("initial")
            .context("reduce expression missing initial")?,
    )?;
    let expr = spec.get("expr").context("reduce expression missing expr")?;
    for item in items {
        let snapshot = apply_temp_vars(
            ctx,
            [(acc_var.to_string(), acc), (item_var.to_string(), item)],
        );
        let next = eval_expr(ctx, expr);
        restore_temp_vars(ctx, snapshot);
        acc = next?;
    }
    Ok(acc)
}

enum Quantifier {
    All,
    Any,
    None,
}

fn eval_quantifier(
    ctx: &mut WorkflowContext,
    value: &Value,
    quantifier: Quantifier,
) -> anyhow::Result<Value> {
    let spec = value
        .as_object()
        .context("quantifier expression must be an object")?;
    let var = spec.get("as").and_then(Value::as_str).unwrap_or("item");
    let items = eval_items(ctx, spec, "quantifier")?;
    let condition = spec.get("if").context("quantifier expression missing if")?;
    let mut any_match = false;
    let mut all_match = true;
    for item in items {
        let snapshot = apply_temp_vars(ctx, [(var.to_string(), item)]);
        let matched = eval_expr(ctx, condition).map(|value| truthy(&value));
        restore_temp_vars(ctx, snapshot);
        let matched = matched?;
        any_match |= matched;
        all_match &= matched;
        match quantifier {
            Quantifier::All if !matched => return Ok(Value::Bool(false)),
            Quantifier::Any if matched => return Ok(Value::Bool(true)),
            Quantifier::None if matched => return Ok(Value::Bool(false)),
            Quantifier::All | Quantifier::Any | Quantifier::None => {}
        }
    }
    Ok(Value::Bool(match quantifier {
        Quantifier::All => all_match,
        Quantifier::Any => any_match,
        Quantifier::None => true,
    }))
}

fn eval_count_if(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let spec = value
        .as_object()
        .context("count_if expression must be an object")?;
    let var = spec.get("as").and_then(Value::as_str).unwrap_or("item");
    let items = eval_items(ctx, spec, "count_if")?;
    let condition = spec.get("if").context("count_if expression missing if")?;
    let mut count = 0_u64;
    for item in items {
        let snapshot = apply_temp_vars(ctx, [(var.to_string(), item)]);
        let matched = eval_expr(ctx, condition).map(|value| truthy(&value));
        restore_temp_vars(ctx, snapshot);
        if matched? {
            count += 1;
        }
    }
    Ok(Value::Number(Number::from(count)))
}

fn eval_find_if(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let spec = value
        .as_object()
        .context("find_if expression must be an object")?;
    let var = spec.get("as").and_then(Value::as_str).unwrap_or("item");
    let items = eval_items(ctx, spec, "find_if")?;
    let condition = spec.get("if").context("find_if expression missing if")?;
    for item in items {
        let snapshot = apply_temp_vars(ctx, [(var.to_string(), item.clone())]);
        let matched = eval_expr(ctx, condition).map(|value| truthy(&value));
        restore_temp_vars(ctx, snapshot);
        if matched? {
            return Ok(item);
        }
    }
    Ok(Value::Null)
}

fn eval_partition(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let spec = value
        .as_object()
        .context("partition expression must be an object")?;
    let var = spec.get("as").and_then(Value::as_str).unwrap_or("item");
    let items = eval_items(ctx, spec, "partition")?;
    let condition = spec.get("if").context("partition expression missing if")?;
    let mut matched = Vec::new();
    let mut rest = Vec::new();
    for item in items {
        let snapshot = apply_temp_vars(ctx, [(var.to_string(), item.clone())]);
        let keep = eval_expr(ctx, condition).map(|value| truthy(&value));
        restore_temp_vars(ctx, snapshot);
        if keep? {
            matched.push(item);
        } else {
            rest.push(item);
        }
    }
    Ok(json_object([
        ("matched", Value::Array(matched)),
        ("rest", Value::Array(rest)),
    ]))
}

fn eval_group_by(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let spec = value
        .as_object()
        .context("group_by expression must be an object")?;
    let var = spec.get("as").and_then(Value::as_str).unwrap_or("item");
    let items = eval_items(ctx, spec, "group_by")?;
    let key_expr = spec.get("key").context("group_by expression missing key")?;
    let mut groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for item in items {
        let snapshot = apply_temp_vars(ctx, [(var.to_string(), item.clone())]);
        let key = eval_expr(ctx, key_expr).map(|value| value_to_string(&value));
        restore_temp_vars(ctx, snapshot);
        groups.entry(key?).or_default().push(item);
    }
    Ok(Value::Object(
        groups
            .into_iter()
            .map(|(key, values)| (key, Value::Array(values)))
            .collect(),
    ))
}

fn eval_scan(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let spec = value
        .as_object()
        .context("scan expression must be an object")?;
    let item_var = spec.get("as").and_then(Value::as_str).unwrap_or("item");
    let acc_var = spec.get("acc").and_then(Value::as_str).unwrap_or("acc");
    let items = eval_items(ctx, spec, "scan")?;
    let mut acc = eval_expr(
        ctx,
        spec.get("initial")
            .context("scan expression missing initial")?,
    )?;
    let expr = spec.get("expr").context("scan expression missing expr")?;
    let mut out = Vec::new();
    if spec
        .get("include_initial")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        out.push(acc.clone());
    }
    for item in items {
        let snapshot = apply_temp_vars(
            ctx,
            [(acc_var.to_string(), acc), (item_var.to_string(), item)],
        );
        let next = eval_expr(ctx, expr);
        restore_temp_vars(ctx, snapshot);
        acc = next?;
        out.push(acc.clone());
    }
    Ok(Value::Array(out))
}

fn eval_set_union(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let arrays = eval_array_exprs(ctx, value, "set_union")?;
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for array in arrays {
        for item in array {
            if seen.insert(value_key(&item)) {
                out.push(item);
            }
        }
    }
    Ok(Value::Array(out))
}

fn eval_set_intersection(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let arrays = eval_array_exprs(ctx, value, "set_intersection")?;
    let [first, rest @ ..] = arrays.as_slice() else {
        bail!("set_intersection expects at least one array");
    };
    let rest_sets = rest
        .iter()
        .map(|items| items.iter().map(value_key).collect::<BTreeSet<_>>())
        .collect::<Vec<_>>();
    let mut emitted = BTreeSet::new();
    let out = first
        .iter()
        .filter(|item| {
            let key = value_key(item);
            emitted.insert(key.clone()) && rest_sets.iter().all(|set| set.contains(&key))
        })
        .cloned()
        .collect();
    Ok(Value::Array(out))
}

fn eval_set_difference(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let arrays = eval_array_exprs(ctx, value, "set_difference")?;
    let [first, rest @ ..] = arrays.as_slice() else {
        bail!("set_difference expects at least one array");
    };
    let excluded = rest
        .iter()
        .flat_map(|items| items.iter().map(value_key))
        .collect::<BTreeSet<_>>();
    let mut emitted = BTreeSet::new();
    let out = first
        .iter()
        .filter(|item| {
            let key = value_key(item);
            emitted.insert(key.clone()) && !excluded.contains(&key)
        })
        .cloned()
        .collect();
    Ok(Value::Array(out))
}

fn eval_set_includes(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let arrays = eval_array_exprs(ctx, value, "set_includes")?;
    let [haystack, needle] = arrays.as_slice() else {
        bail!("set_includes expects exactly two arrays");
    };
    let haystack = haystack.iter().map(value_key).collect::<BTreeSet<_>>();
    Ok(Value::Bool(
        needle
            .iter()
            .map(value_key)
            .all(|key| haystack.contains(&key)),
    ))
}

enum MinMax {
    Min,
    Max,
}

fn eval_min_max(ctx: &mut WorkflowContext, value: &Value, mode: MinMax) -> anyhow::Result<Value> {
    let items = eval_expr(ctx, value)?
        .as_array()
        .cloned()
        .context("min/max expression must evaluate to an array")?;
    let mut best: Option<Value> = None;
    for item in items {
        let replace = if let Some(best) = best.as_ref() {
            let left = value_to_f64(&item)?;
            let right = value_to_f64(best)?;
            match mode {
                MinMax::Min => left < right,
                MinMax::Max => left > right,
            }
        } else {
            true
        };
        if replace {
            best = Some(item);
        }
    }
    Ok(best.unwrap_or(Value::Null))
}

fn eval_enumerate(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let items = eval_expr(ctx, value)?
        .as_array()
        .cloned()
        .context("enumerate expression must evaluate to an array")?;
    Ok(Value::Array(
        items
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                json_object([
                    ("index", Value::Number(Number::from(index as u64))),
                    ("value", value),
                ])
            })
            .collect(),
    ))
}

fn eval_zip(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let arrays = eval_array_exprs(ctx, value, "zip")?;
    let length = arrays.iter().map(Vec::len).min().unwrap_or(0);
    let mut out = Vec::with_capacity(length);
    for index in 0..length {
        out.push(Value::Array(
            arrays.iter().map(|items| items[index].clone()).collect(),
        ));
    }
    Ok(Value::Array(out))
}

fn eval_keys(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let object = eval_expr(ctx, value)?
        .as_object()
        .cloned()
        .context("keys expression must evaluate to an object")?;
    Ok(Value::Array(
        object.keys().cloned().map(Value::String).collect(),
    ))
}

fn eval_values(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let object = eval_expr(ctx, value)?
        .as_object()
        .cloned()
        .context("values expression must evaluate to an object")?;
    Ok(Value::Array(object.values().cloned().collect()))
}

fn eval_entries(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let object = eval_expr(ctx, value)?
        .as_object()
        .cloned()
        .context("entries expression must evaluate to an object")?;
    Ok(Value::Array(
        object
            .into_iter()
            .map(|(key, value)| json_object([("key", Value::String(key)), ("value", value)]))
            .collect(),
    ))
}

fn eval_from_entries(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let entries = eval_expr(ctx, value)?
        .as_array()
        .cloned()
        .context("from_entries expression must evaluate to an array")?;
    let mut object = Map::new();
    for entry in entries {
        if let Some(entry) = entry.as_object() {
            let key = entry
                .get("key")
                .map(value_to_string)
                .context("from_entries object entry missing key")?;
            let value = entry
                .get("value")
                .cloned()
                .context("from_entries object entry missing value")?;
            object.insert(key, value);
        } else if let Some(entry) = entry.as_array() {
            let [key, value] = entry.as_slice() else {
                bail!("from_entries array entries must contain two values");
            };
            object.insert(value_to_string(key), value.clone());
        } else {
            bail!("from_entries expects entry objects or two-item arrays");
        }
    }
    Ok(Value::Object(object))
}

fn eval_merge(ctx: &mut WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    let objects = value
        .as_array()
        .context("merge expression must be an array")?;
    let mut merged = Map::new();
    for object in objects {
        let object = eval_expr(ctx, object)?
            .as_object()
            .cloned()
            .context("merge items must evaluate to objects")?;
        for (key, value) in object {
            merged.insert(key, value);
        }
    }
    Ok(Value::Object(merged))
}

enum PickOmit {
    Pick,
    Omit,
}

fn eval_pick_omit(
    ctx: &mut WorkflowContext,
    value: &Value,
    mode: PickOmit,
) -> anyhow::Result<Value> {
    let spec = value
        .as_object()
        .context("pick/omit expression must be an object")?;
    let object = eval_expr(
        ctx,
        spec.get("object")
            .context("pick/omit expression missing object")?,
    )?
    .as_object()
    .cloned()
    .context("pick/omit object must evaluate to an object")?;
    let keys = eval_expr(
        ctx,
        spec.get("keys")
            .context("pick/omit expression missing keys")?,
    )?
    .as_array()
    .cloned()
    .context("pick/omit keys must evaluate to an array")?
    .iter()
    .map(value_to_string)
    .collect::<BTreeSet<_>>();
    Ok(Value::Object(
        object
            .into_iter()
            .filter(|(key, _)| match mode {
                PickOmit::Pick => keys.contains(key),
                PickOmit::Omit => !keys.contains(key),
            })
            .collect(),
    ))
}

fn eval_items(
    ctx: &mut WorkflowContext,
    spec: &Map<String, Value>,
    op: &str,
) -> anyhow::Result<Vec<Value>> {
    eval_expr(
        ctx,
        spec.get("items")
            .with_context(|| format!("{op} expression missing items"))?,
    )?
    .as_array()
    .cloned()
    .with_context(|| format!("{op} items must be an array"))
}

fn eval_array_exprs(
    ctx: &mut WorkflowContext,
    value: &Value,
    op: &str,
) -> anyhow::Result<Vec<Vec<Value>>> {
    value
        .as_array()
        .with_context(|| format!("{op} expression must be an array"))?
        .iter()
        .map(|expr| {
            eval_expr(ctx, expr)?
                .as_array()
                .cloned()
                .with_context(|| format!("{op} arguments must evaluate to arrays"))
        })
        .collect()
}

fn eval_pair(ctx: &mut WorkflowContext, value: &Value, op: &str) -> anyhow::Result<(Value, Value)> {
    let values = value
        .as_array()
        .with_context(|| format!("{op} expression must be an array"))?;
    let [left, right] = values.as_slice() else {
        bail!("{op} expression expects exactly two arguments");
    };
    Ok((eval_expr(ctx, left)?, eval_expr(ctx, right)?))
}

fn eval_binary(op: &str, left: Value, right: Value) -> anyhow::Result<Value> {
    match op {
        "eq" => Ok(Value::Bool(left == right)),
        "ne" => Ok(Value::Bool(left != right)),
        "lt" => compare_values(&left, &right, |a, b| a < b),
        "lte" => compare_values(&left, &right, |a, b| a <= b),
        "gt" => compare_values(&left, &right, |a, b| a > b),
        "gte" => compare_values(&left, &right, |a, b| a >= b),
        "add" => add_values(&left, &right),
        "sub" => sub_values(&left, &right),
        _ => bail!("unknown binary operator {op}"),
    }
}

fn add_values(left: &Value, right: &Value) -> anyhow::Result<Value> {
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_i64()) {
        return Ok(Value::Number(Number::from(left + right)));
    }
    if left.is_string() || right.is_string() {
        return Ok(Value::String(format!(
            "{}{}",
            value_to_string(left),
            value_to_string(right)
        )));
    }
    number_from_f64(value_to_f64(left)? + value_to_f64(right)?)
}

fn sub_values(left: &Value, right: &Value) -> anyhow::Result<Value> {
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_i64()) {
        return Ok(Value::Number(Number::from(left - right)));
    }
    number_from_f64(value_to_f64(left)? - value_to_f64(right)?)
}

fn compare_values<F>(left: &Value, right: &Value, compare: F) -> anyhow::Result<Value>
where
    F: FnOnce(f64, f64) -> bool,
{
    Ok(Value::Bool(compare(
        value_to_f64(left)?,
        value_to_f64(right)?,
    )))
}

fn should_run(ctx: &mut WorkflowContext, step: &Map<String, Value>) -> anyhow::Result<bool> {
    step.get("if")
        .map(|condition| eval_expr(ctx, condition).map(|value| truthy(&value)))
        .unwrap_or(Ok(true))
}

fn render_template(ctx: &WorkflowContext, value: &str) -> anyhow::Result<String> {
    let re = Regex::new(r"\$\{([^}]+)\}")?;
    let mut rendered = String::with_capacity(value.len());
    let mut last = 0;
    for captures in re.captures_iter(value) {
        let Some(matched) = captures.get(0) else {
            continue;
        };
        rendered.push_str(&value[last..matched.start()]);
        let Some(ref_name) = captures.get(1).map(|capture| capture.as_str()) else {
            continue;
        };
        rendered.push_str(&value_to_string(&resolve_ref(ctx, ref_name)?));
        last = matched.end();
    }
    rendered.push_str(&value[last..]);
    Ok(rendered)
}

fn render_value(ctx: &WorkflowContext, value: &Value) -> anyhow::Result<Value> {
    match value {
        Value::String(text) => render_template(ctx, text).map(Value::String),
        Value::Array(items) => items
            .iter()
            .map(|item| render_value(ctx, item))
            .collect::<anyhow::Result<Vec<_>>>()
            .map(Value::Array),
        Value::Object(object) => object
            .iter()
            .map(|(key, value)| Ok((key.clone(), render_value(ctx, value)?)))
            .collect::<anyhow::Result<Map<_, _>>>()
            .map(Value::Object),
        value => Ok(value.clone()),
    }
}

fn resolve_ref(ctx: &WorkflowContext, reference: &str) -> anyhow::Result<Value> {
    let mut parts = reference.split('.');
    let root = parts.next().context("empty ref")?;
    let mut current = match root {
        "vars" => Value::Object(ctx.vars.clone().into_iter().collect()),
        "steps" => Value::Object(ctx.steps.clone().into_iter().collect()),
        _ => ctx
            .vars
            .get(root)
            .cloned()
            .with_context(|| format!("unknown ref root `{root}`"))?,
    };
    for part in parts {
        current = current
            .as_object()
            .and_then(|object| object.get(part))
            .cloned()
            .with_context(|| format!("missing ref `{reference}` at `{part}`"))?;
    }
    Ok(current)
}

fn apply_temp_vars<I>(ctx: &mut WorkflowContext, updates: I) -> TempVars
where
    I: IntoIterator<Item = (String, Value)>,
{
    let mut previous = Vec::new();
    for (key, value) in updates {
        previous.push((key.clone(), ctx.vars.get(&key).cloned()));
        ctx.vars.insert(key, value);
    }
    TempVars { previous }
}

fn restore_temp_vars(ctx: &mut WorkflowContext, snapshot: TempVars) {
    for (key, value) in snapshot.previous.into_iter().rev() {
        if let Some(value) = value {
            ctx.vars.insert(key, value);
        } else {
            ctx.vars.remove(&key);
        }
    }
}

fn compact_value(value: &Value) -> (Option<String>, Option<String>) {
    compact_text(&value_to_string(value))
}

fn compact_text(text: &str) -> (Option<String>, Option<String>) {
    let normalized = text
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    if normalized.is_empty() {
        return (Some(String::new()), None);
    }
    let digest = hex_sha1(&normalized);
    let preview = if normalized.chars().count() <= 180 {
        normalized
    } else {
        format!(
            "{}...[truncated]",
            normalized.chars().take(180).collect::<String>()
        )
    };
    (Some(digest), Some(preview))
}

fn hex_sha1(text: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(text.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_sha1_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(value) => !value.is_empty(),
        Value::Object(value) => !value.is_empty(),
    }
}

fn value_len(value: &Value) -> usize {
    match value {
        Value::String(value) => value.len(),
        Value::Array(value) => value.len(),
        Value::Object(value) => value.len(),
        _ => value_to_string(value).len(),
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => "None".to_string(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn value_key(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| value_to_string(value))
}

fn value_to_i64(value: &Value) -> anyhow::Result<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
        .with_context(|| format!("expected integer, got {value:?}"))
}

fn value_to_f64(value: &Value) -> anyhow::Result<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
        .with_context(|| format!("expected number, got {value:?}"))
}

fn number_from_f64(value: f64) -> anyhow::Result<Value> {
    Number::from_f64(value)
        .map(Value::Number)
        .context("number operation produced a non-finite value")
}

fn object_to_btree(object: Option<&Map<String, Value>>) -> BTreeMap<String, Value> {
    object
        .into_iter()
        .flat_map(|object| object.iter())
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn json_object<const N: usize>(items: [(&str, Value); N]) -> Value {
    Value::Object(
        items
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

fn option_string_value(value: Option<&str>) -> Value {
    value
        .map(|value| Value::String(value.to_string()))
        .unwrap_or(Value::Null)
}

fn option_usize_value(value: Option<usize>) -> Value {
    value
        .map(|value| Value::Number(Number::from(value)))
        .unwrap_or(Value::Null)
}

fn ensure_parent(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    Ok(())
}

impl WorkflowContext {
    fn log(&self, event: Value) -> anyhow::Result<()> {
        let mut object = event.as_object().cloned().unwrap_or_default();
        object.insert("ts".to_string(), timestamp_value());
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .with_context(|| format!("failed to open {}", self.log_path.display()))?;
        serde_json::to_writer(&mut file, &Value::Object(object))?;
        file.write_all(b"\n")?;
        Ok(())
    }
}

fn timestamp_value() -> Value {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default();
    Number::from_f64(seconds)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::WorkflowOptions;
    use super::run_workflow_value_with_options;
    use super::run_workflow_with_options;

    #[test]
    fn compact_text_truncates_on_char_boundary() {
        let text = format!("a{}", "\u{00e9}".repeat(180));
        let expected_head = format!("a{}", "\u{00e9}".repeat(179));

        let (_, preview) = super::compact_text(&text);

        assert_eq!(Some(format!("{expected_head}...[truncated]")), preview);
    }

    #[test]
    fn compact_value_bounds_large_json_preview() {
        let value = json!({ "entries": ["x".repeat(512)] });

        let (_, preview) = super::compact_value(&value);
        let preview = preview.expect("large preview should be present");

        assert!(preview.ends_with("...[truncated]"));
        assert!(preview.chars().count() <= 194);
    }

    #[test]
    fn runs_json_edit_and_assert_steps() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let cases = root.join("cases");
        fs::create_dir_all(&cases)?;
        let spec_path = cases.join("workflow.json");
        let report_path = root.join("reports").join("report.json");
        let log_path = root.join("reports").join("report.jsonl");

        fs::write(
            &spec_path,
            serde_json::to_string_pretty(&json!({
                "name": "workflow-batch-test",
                "steps": [
                    {
                        "id": "write_text",
                        "write_file": {
                            "path": "reports/work.txt",
                            "content": "alpha\nbeta\ngamma\n"
                        }
                    },
                    {
                        "id": "edit_text",
                        "edit_file": {
                            "path": "reports/work.txt",
                            "operations": [
                                {
                                    "op": "insert_at_position",
                                    "line": 2,
                                    "column": 5,
                                    "content": "-MID"
                                },
                                {
                                    "op": "replace_span",
                                    "start_line": 3,
                                    "start_column": 1,
                                    "end_line": 3,
                                    "end_column": 6,
                                    "content": "GAMMA"
                                }
                            ]
                        }
                    },
                    {
                        "id": "read_text",
                        "read_file": {
                            "path": "reports/work.txt",
                            "var": "edited"
                        }
                    },
                    {
                        "id": "write_json",
                        "write_json": {
                            "path": "reports/data.json",
                            "value": {
                                "merge": [
                                    {
                                        "literal": {
                                            "alpha": 1,
                                            "beta": 2
                                        }
                                    },
                                    {
                                        "literal": {
                                            "beta": 3
                                        }
                                    }
                                ]
                            }
                        }
                    },
                    {
                        "id": "read_json",
                        "read_json": {
                            "path": "reports/data.json",
                            "var": "data"
                        }
                    },
                    {
                        "id": "assert_outputs",
                        "assert": {
                            "and": [
                                {
                                    "contains": [
                                        {
                                            "ref": "vars.edited"
                                        },
                                        "beta-MID"
                                    ]
                                },
                                {
                                    "eq": [
                                        {
                                            "get": {
                                                "from": {
                                                    "ref": "vars.data"
                                                },
                                                "key": "beta"
                                            }
                                        },
                                        3
                                    ]
                                },
                                {
                                    "set_includes": [
                                        {
                                            "keys": {
                                                "ref": "vars.data"
                                            }
                                        },
                                        [
                                            "alpha",
                                            "beta"
                                        ]
                                    ]
                                }
                            ]
                        }
                    }
                ]
            }))?,
        )?;

        let summary = run_workflow_with_options(
            &spec_path,
            &report_path,
            &log_path,
            WorkflowOptions::unrestricted_with_root(root),
        )?;

        assert_eq!("ok", summary.status, "{summary:#?}");
        assert_eq!(6, summary.steps_total);
        assert_eq!(0, summary.steps_failed);
        assert_eq!(
            "alpha\nbeta-MID\nGAMMA\n",
            fs::read_to_string(root.join("reports/work.txt"))?
        );
        assert!(report_path.exists());
        assert!(log_path.exists());

        Ok(())
    }

    #[test]
    fn runs_inline_spec_without_spec_file() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let report_path = root.join("reports").join("report.json");
        let log_path = root.join("reports").join("report.jsonl");
        fs::write(root.join("input.txt"), "alpha\nbeta\ngamma\n")?;

        let summary = run_workflow_value_with_options(
            json!({
                "name": "inline-workflow-batch-test",
                "steps": [
                    {
                        "id": "read_input",
                        "read_file": {
                            "path": "input.txt",
                            "var": "body"
                        }
                    },
                    {
                        "id": "assert_body",
                        "assert": {
                            "contains": [
                                {
                                    "ref": "vars.body"
                                },
                                "beta"
                            ]
                        }
                    }
                ]
            }),
            &report_path,
            &log_path,
            WorkflowOptions::root_confined(root),
        )?;

        assert_eq!("ok", summary.status, "{summary:#?}");
        assert_eq!("<inline>", summary.spec);
        assert_eq!(2, summary.steps_total);
        assert_eq!(0, summary.steps_failed);
        assert!(report_path.exists());
        assert!(log_path.exists());

        Ok(())
    }

    #[test]
    fn accepts_assert_object_expr_and_set_vars_alias() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let report_path = root.join("reports").join("report.json");
        let log_path = root.join("reports").join("report.jsonl");

        let summary = run_workflow_value_with_options(
            json!({
                "steps": [
                    {
                        "id": "write_readme",
                        "write_file": {
                            "path": "output/readme.txt",
                            "content": "sum=10\nproduct=30"
                        }
                    },
                    {
                        "id": "read_readme",
                        "read_file": {
                            "path": "output/readme.txt",
                            "var": "readme"
                        }
                    },
                    {
                        "id": "set_summary",
                        "set_vars": {
                            "summary": {
                                "literal": {
                                    "sum": 10,
                                    "product": 30
                                }
                            }
                        }
                    },
                    {
                        "id": "assert_readme",
                        "assert": {
                            "expr": "readme == 'sum=10\\nproduct=30'",
                            "message": "readme mismatch"
                        }
                    },
                    {
                        "id": "assert_summary",
                        "assert": {
                            "expr": {
                                "eq": [
                                    {
                                        "ref": "summary"
                                    },
                                    {
                                        "literal": {
                                            "sum": 10,
                                            "product": 30
                                        }
                                    }
                                ]
                            },
                            "message": "summary mismatch"
                        }
                    }
                ]
            }),
            &report_path,
            &log_path,
            WorkflowOptions::root_confined(root),
        )?;

        assert_eq!("ok", summary.status, "{summary:#?}");
        assert_eq!(5, summary.steps_total);
        assert_eq!(0, summary.steps_failed);

        Ok(())
    }

    #[test]
    fn supports_powershell_like_file_substitutions() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let report_path = root.join("reports").join("report.json");
        let log_path = root.join("reports").join("report.jsonl");

        let summary = run_workflow_value_with_options(
            json!({
                "steps": [
                    {
                        "id": "mkdir",
                        "ensure_dir": {
                            "path": "data/nested"
                        }
                    },
                    {
                        "id": "write_a",
                        "write_file": {
                            "path": "data/a.txt",
                            "content": "alpha\n"
                        }
                    },
                    {
                        "id": "write_b",
                        "write_file": {
                            "path": "data/nested/b.txt",
                            "content": "beta\n"
                        }
                    },
                    {
                        "id": "stat_a",
                        "stat_path": {
                            "path": "data/a.txt",
                            "var": "a_stat",
                            "sha1": true
                        }
                    },
                    {
                        "id": "list_data",
                        "list_files": {
                            "path": "data",
                            "recursive": true,
                            "var": "paths"
                        }
                    },
                    {
                        "id": "assert_kind",
                        "assert": {
                            "expr": {
                                "eq": [
                                    {
                                        "get": {
                                            "from": {
                                                "ref": "a_stat"
                                            },
                                            "key": "kind"
                                        }
                                    },
                                    "file"
                                ]
                            }
                        }
                    },
                    {
                        "id": "assert_listing",
                        "assert": {
                            "expr": {
                                "and": [
                                    {
                                        "set_includes": [
                                            {
                                                "ref": "paths"
                                            },
                                            [
                                                "data/a.txt"
                                            ]
                                        ]
                                    },
                                    {
                                        "set_includes": [
                                            {
                                                "ref": "paths"
                                            },
                                            [
                                                "data/nested/b.txt"
                                            ]
                                        ]
                                    }
                                ]
                            }
                        }
                    }
                ]
            }),
            &report_path,
            &log_path,
            WorkflowOptions::root_confined(root),
        )?;

        assert_eq!("ok", summary.status, "{summary:#?}");
        assert_eq!(7, summary.steps_total);
        assert_eq!(0, summary.steps_failed);

        Ok(())
    }

    #[test]
    fn context_tool_options_reject_run_steps() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        fs::create_dir_all(root.join("plans"))?;
        fs::create_dir_all(root.join("reports"))?;
        let spec_path = root.join("plans").join("run.json");
        let report_path = root.join("reports").join("report.json");
        let log_path = root.join("reports").join("report.jsonl");

        fs::write(
            &spec_path,
            serde_json::to_string_pretty(&json!({
                "steps": [
                    {
                        "id": "attempt_command",
                        "run": ["definitely-not-executed"]
                    }
                ]
            }))?,
        )?;

        let summary = run_workflow_with_options(
            &spec_path,
            &report_path,
            &log_path,
            WorkflowOptions::context_tool(root),
        )?;

        assert_eq!("failed", summary.status);
        assert_eq!(1, summary.steps_total);
        assert_eq!(1, summary.steps_failed);
        assert_eq!("attempt_command", summary.steps[0].id);
        assert_eq!("failed", summary.steps[0].status);
        assert!(
            summary.steps[0]
                .note
                .as_deref()
                .is_some_and(|note| note.contains("disabled"))
        );
        assert!(report_path.exists());
        assert!(log_path.exists());

        Ok(())
    }

    #[test]
    fn run_step_cwd_is_resolved_against_workflow_root() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        fs::create_dir_all(root.join("plans"))?;
        fs::create_dir_all(root.join("reports"))?;
        fs::create_dir_all(root.join("work"))?;
        let spec_path = root.join("plans").join("cwd.json");
        let report_path = root.join("reports").join("report.json");
        let log_path = root.join("reports").join("report.jsonl");
        let script_path = if cfg!(windows) {
            let path = root.join("write-cwd.cmd");
            fs::write(&path, "@echo %CD%>cwd.txt\r\n")?;
            path
        } else {
            let path = root.join("write-cwd.sh");
            fs::write(&path, "pwd > cwd.txt\n")?;
            path
        };
        let run = if cfg!(windows) {
            json!(["cmd", "/C", script_path.to_string_lossy()])
        } else {
            json!(["sh", script_path.to_string_lossy()])
        };

        fs::write(
            &spec_path,
            serde_json::to_string_pretty(&json!({
                "steps": [
                    {
                        "id": "write_cwd",
                        "cwd": "work",
                        "run": run
                    }
                ]
            }))?,
        )?;

        let summary = run_workflow_with_options(
            &spec_path,
            &report_path,
            &log_path,
            WorkflowOptions::unrestricted_with_root(root),
        )?;
        let observed_cwd = fs::read_to_string(root.join("work").join("cwd.txt"))?;

        assert_eq!("ok", summary.status);
        assert_eq!(
            root.join("work").canonicalize()?,
            PathBuf::from(observed_cwd.trim()).canonicalize()?
        );

        Ok(())
    }

    #[test]
    fn output_paths_reject_parent_dir_escape() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        fs::create_dir_all(root.join("plans"))?;
        fs::create_dir_all(root.join("reports"))?;
        let spec_path = root.join("plans").join("escape.json");
        let report_path = root.join("reports").join("report.json");
        let log_path = root.join("reports").join("report.jsonl");
        let escaped_name = format!(
            "{}-escape.txt",
            root.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("workflow-batch")
        );
        let escaped_path = root
            .parent()
            .expect("tempdir should have parent")
            .join(&escaped_name);
        if escaped_path.exists() {
            fs::remove_file(&escaped_path)?;
        }

        fs::write(
            &spec_path,
            serde_json::to_string_pretty(&json!({
                "steps": [
                    {
                        "id": "escape",
                        "write_file": {
                            "path": format!("../{escaped_name}"),
                            "content": "escaped"
                        }
                    }
                ]
            }))?,
        )?;

        let summary = run_workflow_with_options(
            &spec_path,
            &report_path,
            &log_path,
            WorkflowOptions::unrestricted_with_root(root),
        )?;

        assert_eq!("failed", summary.status);
        assert!(!escaped_path.exists());
        assert!(report_path.exists());
        assert!(log_path.exists());

        Ok(())
    }
}
