use crate::commands::not_implemented;
use std::process::ExitCode;

#[derive(clap::Args)]
#[command(about = crate::descriptions::subcommands::DOCTOR)]
pub struct DoctorArgs;

pub(super) fn run(_args: DoctorArgs) -> ExitCode {
    not_implemented("doctor")
}
