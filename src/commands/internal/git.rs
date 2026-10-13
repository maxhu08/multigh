use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::internal::subcommands::GIT, disable_help_flag = true)]
pub struct GitArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

pub(super) fn run(_args: GitArgs) -> ExitCode {
    not_implemented("internal git")
}
