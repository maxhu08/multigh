use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::identity::subcommands::NEW)]
pub struct NewArgs {
    pub identity: Option<String>,
    #[command(flatten)]
    pub fields: super::IdentityFields,
}

pub(super) fn run(_args: NewArgs) -> ExitCode {
    not_implemented("identity new")
}
