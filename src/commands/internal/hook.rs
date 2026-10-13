use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::internal::subcommands::HOOK)]
pub struct HookArgs {
    pub name: String,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

pub(super) fn run(_args: HookArgs) -> ExitCode {
    not_implemented("internal hook")
}
