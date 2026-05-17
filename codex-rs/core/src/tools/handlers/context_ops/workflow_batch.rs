use serde::Deserialize;

use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::handlers::parse_arguments;

use super::execution;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkflowBatchArgs {
    spec_path: String,
    report_path: String,
    log_path: String,
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
    let spec_path = base_path.join(args.spec_path);
    let report_path = base_path.join(args.report_path);
    let log_path = base_path.join(args.log_path);

    let summary = codex_workflow_batch::run_workflow_with_options(
        spec_path.as_path(),
        report_path.as_path(),
        log_path.as_path(),
        codex_workflow_batch::WorkflowOptions::context_tool(base_path.to_path_buf()),
    )
    .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;
    let success = summary.status == "ok";
    let output = serde_json::to_string_pretty(&summary)
        .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;

    Ok(FunctionToolOutput::from_text(output, Some(success)))
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
    async fn workflow_batch_context_op_rejects_command_steps() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let cases = root.join("cases");
        fs::create_dir_all(&cases)?;
        fs::write(
            cases.join("workflow.json"),
            serde_json::to_string_pretty(&json!({
                "name": "handler-command-test",
                "steps": [
                    {
                        "id": "attempt_command",
                        "run": ["definitely-not-executed"]
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
        assert_eq!(summary["steps"][0]["id"], "attempt_command");
        assert_eq!(summary["steps"][0]["status"], "failed");
        assert!(
            summary["steps"][0]["note"]
                .as_str()
                .is_some_and(|note| note.contains("disabled"))
        );
        assert!(root.join("reports/summary.json").exists());
        assert!(root.join("reports/events.jsonl").exists());

        Ok(())
    }
}
