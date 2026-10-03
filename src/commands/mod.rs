pub mod enter;
mod modes;
mod new;
mod status;
mod switch;
mod welcome;

use crate::{
    cli::{Cli, Command, Hook, IntegrationShell},
    config::{self, Config},
    git, guard, hooks, output, settings,
};
use anyhow::Result;
use clap::CommandFactory;

pub fn run(cli: Cli) -> Result<()> {
    if let Command::Completions { shell } = cli.command {
        clap_complete::generate(shell, &mut Cli::command(), "mgh", &mut std::io::stdout());

        return Ok(());
    }

    if let Command::Init { shell } = cli.command {
        print!(
            "{}",
            match shell {
                IntegrationShell::Fish => include_str!("../../shell/fish.fish"),
                IntegrationShell::Bash => include_str!("../../shell/bash.bash"),
                IntegrationShell::Zsh => include_str!("../../shell/zsh.zsh"),
            }
        );

        return Ok(());
    }

    let path = config::path(cli.config)?;

    match cli.command {
        Command::New {
            account,
            username,
            email,
            name,
            repo,
        } => new::run(path, account, username, email, name, repo),
        Command::Welcome { state } => welcome::run(path, state),
        Command::Protections { state, repo, allow } => modes::protections(path, state, repo, allow),
        Command::Verbose { state } => modes::verbose(state),
        Command::Enter => enter::run(path),
        Command::Hook {
            kind: Hook::Run { name, args },
        } => hooks::run(path, &name, &args),
        Command::Hook { .. } if !settings::enabled("protections")? => Ok(()),
        Command::Status { full } => status::run(path, full),
        command => {
            let config = Config::load(path)?;

            match command {
                Command::Switch { account, repo } => switch::run(&config, &account, repo),

                Command::Setup => setup(&config),

                Command::Check | Command::Hook { kind: Hook::Commit } => guard::check(&config),

                Command::Hook {
                    kind: Hook::Push { .. },
                } => {
                    use std::io::Read;

                    let mut updates = String::new();

                    std::io::stdin().read_to_string(&mut updates)?;
                    guard::push(&config, &updates)
                }

                _ => unreachable!(),
            }
        }
    }
}

fn setup(config: &Config) -> Result<()> {
    git::update_global("mgh setup", |global| {
        hooks::compatible()?;

        let identity_directory = git::setup_identities(config, global)?;

        hooks::install(config, global)?;
        settings::set("protections", true)?;
        settings::set("verbose", true)?;

        output::section("✓ Git account identity rules updated");
        output::row(
            "Accounts",
            &config.path.to_string_lossy(),
            output::Color::Muted,
        );
        output::row(
            "Git config",
            &git::global_path()?.to_string_lossy(),
            output::Color::Muted,
        );

        for alias in config.accounts.keys() {
            output::row(
                "Identity",
                &identity_directory
                    .join(format!("git-{alias}.conf"))
                    .to_string_lossy(),
                output::Color::Changed,
            );
        }

        global.report();
        modes::report("protections")?;
        modes::report("verbose")?;
        println!(
            "  Load mgh init <fish|bash|zsh> in your shell config for entry prompts and reports.\n"
        );

        Ok(())
    })
}
