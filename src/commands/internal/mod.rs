mod enter;
mod git;
mod hook;
mod welcome;

use clap::Subcommand;
use std::process::ExitCode;

#[derive(Subcommand)]
pub enum InternalCommand {
    Enter(enter::EnterArgs),
    Welcome(welcome::WelcomeArgs),
    Git(git::GitArgs),
    Hook(hook::HookArgs),
}

pub(super) fn run(command: InternalCommand) -> ExitCode {
    match command {
        InternalCommand::Enter(args) => enter::run(args),
        InternalCommand::Welcome(args) => welcome::run(args),
        InternalCommand::Git(args) => git::run(args),
        InternalCommand::Hook(args) => hook::run(args),
    }
}
