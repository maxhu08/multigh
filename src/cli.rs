use crate::{commands::Command, descriptions};
use clap::Parser;

#[derive(Parser)]
#[command(
    name = env!("CARGO_BIN_NAME"),
    version,
    about = descriptions::SELF,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}
