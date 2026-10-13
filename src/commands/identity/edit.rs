use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::identity::subcommands::EDIT)]
pub struct EditArgs {
    pub identity: String,
    #[command(flatten)]
    pub fields: super::IdentityFields,
}

pub(super) fn run(_args: EditArgs) -> ExitCode {
    not_implemented("identity edit")
}
