use crate::{commands::Command, descriptions};
use clap::Parser;

#[derive(Parser)]
#[command(
    version,
    about = descriptions::SELF,
    arg_required_else_help = true,
    after_help = descriptions::AFTER_HELP
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}
