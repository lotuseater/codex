use clap::CommandFactory;
use clap::Parser;
use clap_complete::Shell;
use clap_complete::generate;

use crate::MultitoolCli;

#[derive(Debug, Parser)]
pub(crate) struct CompletionCommand {
    /// Shell to generate completions for
    #[clap(value_enum, default_value_t = Shell::Bash)]
    shell: Shell,
}

pub(crate) fn print_completion(cmd: CompletionCommand) {
    let mut app = MultitoolCli::command();
    let name = "codex";
    generate(cmd.shell, &mut app, name, &mut std::io::stdout());
}
