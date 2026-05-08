use serde::Deserialize;

use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::handlers::parse_arguments;

use super::execution;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchTextArgs {
    pattern: String,
    #[serde(default)]
    workdir: Option<String>,
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    glob: Option<String>,
    #[serde(default)]
    globs: Vec<String>,
    #[serde(default)]
    max_files: Option<usize>,
    #[serde(default)]
    max_matches_per_file: Option<usize>,
}

pub(super) async fn handle(
    invocation: ToolInvocation,
    arguments: &str,
) -> Result<FunctionToolOutput, FunctionCallError> {
    let args: SearchTextArgs = parse_arguments(arguments)?;
    if args.pattern.trim().is_empty() {
        return Err(FunctionCallError::RespondToModel(
            "pattern must not be empty".to_string(),
        ));
    }

    let max_files = args
        .max_files
        .map_or(codex_context_ops_impl::DEFAULT_MAX_FILES, |max_files| {
            codex_context_ops_impl::clamp_max_files(max_files)
        });
    let max_matches_per_file = args.max_matches_per_file.map_or(
        codex_context_ops_impl::DEFAULT_MAX_MATCHES_PER_FILE,
        |max_matches_per_file| {
            codex_context_ops_impl::clamp_max_matches_per_file(max_matches_per_file)
        },
    );
    let turn_environment = execution::primary_environment(&invocation)?;
    let workdir = execution::resolve_workdir(turn_environment, args.workdir.as_deref());
    let globs = codex_context_ops_impl::combined_globs(args.glob.as_deref(), &args.globs);

    let output = if turn_environment.environment.is_remote() {
        let output = execution::run_command(
            &invocation,
            turn_environment,
            &workdir,
            codex_context_ops_impl::rg_args(
                &args.pattern,
                &globs,
                &args.paths,
                max_matches_per_file + 1,
            ),
        )
        .await?;
        if output.timed_out {
            return Err(FunctionCallError::RespondToModel(
                "rg timed out while searching".to_string(),
            ));
        }
        if !matches!(output.exit_code, 0 | 1) {
            let stderr = output.stderr_text();
            let message = if stderr.is_empty() {
                format!("rg exited with status {}", output.exit_code)
            } else {
                stderr
            };
            return Err(FunctionCallError::RespondToModel(message));
        }
        let stdout = output.stdout_text();
        codex_context_ops_impl::search_text_from_rg_json_output(
            workdir.as_path(),
            &args.pattern,
            &globs,
            &args.paths,
            max_files,
            max_matches_per_file,
            stdout.lines(),
        )
    } else {
        codex_context_ops_impl::search_text(
            workdir.as_path(),
            &args.pattern,
            &globs,
            &args.paths,
            max_files,
            max_matches_per_file,
        )
        .await
        .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?
    };

    Ok(FunctionToolOutput::from_text(output, Some(true)))
}
