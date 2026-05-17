use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::handlers::parse_arguments;

use super::execution;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkflowBatchArgs {
    #[serde(default)]
    spec_path: Option<String>,
    #[serde(default)]
    spec: Option<Value>,
    #[serde(default)]
    report_path: Option<String>,
    #[serde(default)]
    log_path: Option<String>,
    #[serde(default)]
    workdir: Option<String>,
}

pub(super) async fn handle(
    invocation: ToolInvocation,
    arguments: &str,
) -> Result<FunctionToolOutput, FunctionCallError> {
    let args: WorkflowBatchArgs = parse_arguments(arguments)?;
    let turn_environment = execution::primary_environment(&invocation)?;
    if turn_environment.environment.is_remote() {
        return Err(FunctionCallError::RespondToModel(
            "workflow_batch is only available for local environments".to_string(),
        ));
    }

    let base_path = execution::resolve_workdir(turn_environment, args.workdir.as_deref());
    let sanitized_call_id: String = invocation
        .call_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let output_stem = if sanitized_call_id.is_empty() {
        "workflow-batch".to_string()
    } else {
        sanitized_call_id
    };
    let default_output_dir = base_path.join(".codex").join("workflow-batch");
    let report_path = args.report_path.map_or_else(
        || default_output_dir.join(format!("{output_stem}-report.json")),
        |path| base_path.join(path),
    );
    let log_path = args.log_path.map_or_else(
        || default_output_dir.join(format!("{output_stem}-events.jsonl")),
        |path| base_path.join(path),
    );

    let options = codex_workflow_batch::WorkflowOptions::root_confined(base_path.to_path_buf());
    let summary = match (args.spec_path, args.spec) {
        (Some(spec_path), None) => codex_workflow_batch::run_workflow_with_options(
            base_path.join(spec_path).as_path(),
            report_path.as_path(),
            log_path.as_path(),
            options,
        ),
        (None, Some(spec)) => codex_workflow_batch::run_workflow_value_with_options(
            spec,
            report_path.as_path(),
            log_path.as_path(),
            options,
        ),
        (Some(_), Some(_)) => Err(anyhow::anyhow!(
            "workflow_batch accepts exactly one of `spec_path` or `spec`"
        )),
        (None, None) => Err(anyhow::anyhow!(
            "workflow_batch requires one of `spec_path` or `spec`"
        )),
    }
    .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;
    let success = summary.status == "ok";
    let compact_summary = WorkflowBatchToolSummary::new(&summary, &report_path);
    let output = serde_json::to_string_pretty(&compact_summary)
        .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;

    Ok(FunctionToolOutput::from_text(output, Some(success)))
}

#[derive(Debug, Serialize)]
struct WorkflowBatchToolSummary<'a> {
    name: Option<&'a str>,
    status: &'a str,
    spec: &'a str,
    report: String,
    log: &'a str,
    vars: Vec<&'a str>,
    steps_total: usize,
    steps_failed: usize,
    steps_skipped: usize,
    steps: &'a [codex_workflow_batch::StepRecord],
}

impl<'a> WorkflowBatchToolSummary<'a> {
    fn new(
        summary: &'a codex_workflow_batch::WorkflowSummary,
        report_path: &std::path::Path,
    ) -> Self {
        Self {
            name: summary.name.as_deref(),
            status: &summary.status,
            spec: &summary.spec,
            report: report_path.to_string_lossy().into_owned(),
            log: &summary.log,
            vars: summary.vars.keys().map(String::as_str).collect(),
            steps_total: summary.steps_total,
            steps_failed: summary.steps_failed,
            steps_skipped: summary.steps_skipped,
            steps: &summary.steps,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;

    use codex_protocol::models::FunctionCallOutputContentItem;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tokio::sync::Mutex;

    use super::super::ContextOpsHandler;
    use crate::function_tool::FunctionCallError;
    use crate::session::tests::make_session_and_context;
    use crate::tools::context::FunctionToolOutput;
    use crate::tools::context::ToolCallSource;
    use crate::tools::context::ToolInvocation;
    use crate::tools::context::ToolPayload;
    use crate::tools::registry::ToolExecutor;
    use crate::turn_diff_tracker::TurnDiffTracker;

    async fn invocation_for_arguments(arguments: String) -> ToolInvocation {
        let (session, turn) = make_session_and_context().await;
        ToolInvocation {
            session: session.into(),
            turn: turn.into(),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
            tracker: Arc::new(Mutex::new(TurnDiffTracker::new())),
            call_id: "call-workflow-batch".to_string(),
            tool_name: codex_tools::ToolName::plain(codex_tools::WORKFLOW_BATCH_TOOL_NAME),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function { arguments },
        }
    }

    fn text_output(output: &FunctionToolOutput) -> &str {
        let [FunctionCallOutputContentItem::InputText { text }] = output.body.as_slice() else {
            panic!("expected one text output item");
        };
        text
    }

    fn workflow_batch_handler() -> ContextOpsHandler {
        ContextOpsHandler::new(
            codex_tools::create_context_ops_tools()
                .into_iter()
                .find(|tool| tool.name() == codex_tools::WORKFLOW_BATCH_TOOL_NAME)
                .expect("workflow_batch tool spec"),
        )
    }

    #[tokio::test]
    async fn workflow_batch_reports_failed_summary_as_unsuccessful_tool_output()
    -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let cases = root.join("cases");
        fs::create_dir_all(&cases)?;
        fs::write(
            cases.join("workflow.json"),
            serde_json::to_string_pretty(&json!({
                "name": "handler-failure-test",
                "steps": [
                    {
                        "id": "fails",
                        "assert": {
                            "eq": [1, 2]
                        }
                    }
                ]
            }))?,
        )?;

        let arguments = json!({
            "spec_path": "cases/workflow.json",
            "report_path": "reports/summary.json",
            "log_path": "reports/events.jsonl",
            "workdir": root.to_string_lossy(),
        })
        .to_string();
        let handler = workflow_batch_handler();
        let output = handler
            .handle(invocation_for_arguments(arguments).await)
            .await?;

        assert_eq!(output.success, Some(false));
        let summary: serde_json::Value = serde_json::from_str(text_output(&output))?;
        assert_eq!(summary["status"], "failed");
        assert!(root.join("reports/summary.json").exists());
        assert!(root.join("reports/events.jsonl").exists());

        Ok(())
    }

    #[tokio::test]
    async fn workflow_batch_accepts_inline_spec() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        fs::write(root.join("input.txt"), "alpha\nbeta\n")?;

        let arguments = json!({
            "spec": {
                "name": "handler-inline-test",
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
            },
            "report_path": "reports/summary.json",
            "log_path": "reports/events.jsonl",
            "workdir": root.to_string_lossy(),
        })
        .to_string();
        let handler = workflow_batch_handler();
        let output = handler
            .handle(invocation_for_arguments(arguments).await)
            .await?;

        assert_eq!(output.success, Some(true));
        let summary: serde_json::Value = serde_json::from_str(text_output(&output))?;
        assert_eq!(summary["status"], "ok");
        assert_eq!(summary["spec"], "<inline>");
        assert_eq!(summary["vars"], json!(["body"]));
        assert!(!text_output(&output).contains("alpha"));
        assert!(root.join("reports/summary.json").exists());
        assert!(root.join("reports/events.jsonl").exists());

        Ok(())
    }

    #[tokio::test]
    async fn workflow_batch_rejects_ambiguous_spec_sources() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        let arguments = json!({
            "spec_path": "cases/workflow.json",
            "spec": {
                "steps": []
            },
            "report_path": "reports/summary.json",
            "log_path": "reports/events.jsonl",
            "workdir": root.to_string_lossy(),
        })
        .to_string();
        let handler = workflow_batch_handler();
        let error = match handler
            .handle(invocation_for_arguments(arguments).await)
            .await
        {
            Ok(_) => panic!("ambiguous workflow_batch spec sources should be rejected"),
            Err(error) => error,
        };

        let FunctionCallError::RespondToModel(message) = error else {
            panic!("expected RespondToModel error");
        };
        assert!(message.contains("exactly one of `spec_path` or `spec`"));

        Ok(())
    }

    #[tokio::test]
    async fn workflow_batch_context_op_defaults_report_and_log_paths_for_inline_spec()
    -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let large_input = format!("{}TAIL_SENTINEL", "x".repeat(256));
        fs::write(root.join("input.txt"), &large_input)?;

        let arguments = json!({
            "spec": {
                "name": "inline-default-output-test",
                "steps": [
                    {
                        "id": "write_output",
                        "write_file": {
                            "path": "output.txt",
                            "content": "inline-ok"
                        }
                    },
                    {
                        "id": "read_input",
                        "read_file": {
                            "path": "input.txt",
                            "var": "payload"
                        }
                    }
                ]
            },
            "workdir": root.to_string_lossy(),
        })
        .to_string();
        let handler = workflow_batch_handler();
        let output = handler
            .handle(invocation_for_arguments(arguments).await)
            .await?;

        assert_eq!(output.success, Some(true));
        let output_text = text_output(&output);
        let summary: serde_json::Value = serde_json::from_str(output_text)?;
        assert_eq!(summary["status"], "ok");
        assert_eq!(summary["vars"], json!(["payload"]));
        assert!(!output_text.contains("TAIL_SENTINEL"));
        assert_eq!(fs::read_to_string(root.join("output.txt"))?, "inline-ok");
        assert!(
            root.join(".codex/workflow-batch/call-workflow-batch-report.json")
                .exists()
        );
        assert!(
            root.join(".codex/workflow-batch/call-workflow-batch-events.jsonl")
                .exists()
        );

        Ok(())
    }

    #[tokio::test]
    async fn workflow_batch_context_op_allows_file_mutation_steps() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let cases = root.join("cases");
        fs::create_dir_all(&cases)?;
        fs::write(
            cases.join("workflow.json"),
            serde_json::to_string_pretty(&json!({
                "name": "handler-mutation-test",
                "steps": [
                    {
                        "id": "write_input",
                        "write_file": {
                            "path": "input.txt",
                            "content": "file-ok"
                        }
                    }
                ]
            }))?,
        )?;

        let arguments = json!({
            "spec_path": "cases/workflow.json",
            "report_path": "reports/summary.json",
            "log_path": "reports/events.jsonl",
            "workdir": root.to_string_lossy(),
        })
        .to_string();
        let handler = workflow_batch_handler();
        let output = handler
            .handle(invocation_for_arguments(arguments).await)
            .await?;

        assert_eq!(output.success, Some(true));
        let summary: serde_json::Value = serde_json::from_str(text_output(&output))?;
        assert_eq!(summary["status"], "ok");
        assert_eq!(summary["steps"][0]["id"], "write_input");
        assert_eq!(summary["steps"][0]["status"], "ok");
        assert_eq!(fs::read_to_string(root.join("input.txt"))?, "file-ok");
        assert!(root.join("reports/summary.json").exists());
        assert!(root.join("reports/events.jsonl").exists());

        Ok(())
    }

    #[tokio::test]
    async fn workflow_batch_context_op_rejects_command_steps() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let cases = root.join("cases");
        fs::create_dir_all(&cases)?;
        let run = if cfg!(windows) {
            json!(["cmd", "/C", "echo command-ok> command.txt"])
        } else {
            json!(["sh", "-c", "printf command-ok > command.txt"])
        };
        fs::write(
            cases.join("workflow.json"),
            serde_json::to_string_pretty(&json!({
                "name": "handler-command-test",
                "steps": [
                    {
                        "id": "run_command",
                        "run": run
                    }
                ]
            }))?,
        )?;

        let arguments = json!({
            "spec_path": "cases/workflow.json",
            "report_path": "reports/summary.json",
            "log_path": "reports/events.jsonl",
            "workdir": root.to_string_lossy(),
        })
        .to_string();
        let handler = workflow_batch_handler();
        let error = match handler
            .handle(invocation_for_arguments(arguments).await)
            .await
        {
            Ok(_) => panic!("command steps should be disabled in the Codex tool surface"),
            Err(error) => error,
        };

        let FunctionCallError::RespondToModel(message) = error else {
            panic!("expected RespondToModel error");
        };
        assert!(message.contains("run steps are disabled"));
        assert!(!root.join("command.txt").exists());

        Ok(())
    }
}
