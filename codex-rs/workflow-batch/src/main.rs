use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use codex_workflow_batch::run_workflow;

#[derive(Debug, Parser)]
#[command(about = "Run a JSON workflow-batch spec with deterministic step reports.")]
struct Args {
    #[arg(long)]
    spec: PathBuf,

    #[arg(long)]
    report: PathBuf,

    #[arg(long)]
    log: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let summary = run_workflow(&args.spec, &args.report, &args.log).with_context(|| {
        format!(
            "failed to run workflow spec {}",
            args.spec.to_string_lossy()
        )
    })?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    if summary.status == "ok" {
        Ok(())
    } else {
        anyhow::bail!("workflow failed")
    }
}
