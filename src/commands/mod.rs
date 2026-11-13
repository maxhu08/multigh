use crate::{
    application::{Autoswitch, IdentityFields, Interaction, Invocation, Preference, ShellKind},
    cli::*,
};
use anyhow::Result;
use clap::CommandFactory;

pub fn run(cli: Cli, ui: &mut dyn Interaction) -> Result<i32> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let origin = args
        .iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    let invocation = Invocation::new(cli.config, format!("mgh {origin}"));
    match cli.command {
        Command::Setup => invocation.setup(ui)?,
        Command::Status { full } => invocation.status(full, ui)?,
        Command::Switch { identity } => invocation.switch_identity(&identity, ui)?,
        Command::Doctor => invocation.doctor(ui)?,
        Command::Identity { command } => match command.unwrap_or(IdentityCommand::List) {
            IdentityCommand::List => invocation.list_identities(ui)?,
            IdentityCommand::New { identity, fields } => {
                invocation.create_identity(identity, fields.into(), ui)?
            }
            IdentityCommand::Edit { identity, fields } => {
                invocation.edit_identity(&identity, fields.into(), ui)?
            }
            IdentityCommand::Remove { identity } => invocation.remove_identity(&identity, ui)?,
        },
        Command::Repo { command } => {
            let repo = invocation.require_repository()?;
            match command.unwrap_or(RepoCommand::Status) {
                RepoCommand::Status => repo.status(ui)?,
                RepoCommand::Check => repo.check()?,
                RepoCommand::Protections { state } => {
                    repo.protections(state.map(OnOff::enabled), ui)?
                }
                RepoCommand::Allowed { command } => match command.unwrap_or(AllowedCommand::List) {
                    AllowedCommand::List => repo.list_allowed(ui)?,
                    AllowedCommand::Add { identity } => repo.add_allowed(&identity, ui)?,
                    AllowedCommand::Remove { identity } => repo.remove_allowed(&identity, ui)?,
                    AllowedCommand::Update => repo.update_allowed(ui)?,
                },
            }
        }
        Command::Settings { command } => match command {
            None => invocation.show_preferences(None, ui)?,
            Some(SettingsCommand::Autoswitch { mode }) => match mode {
                Some(mode) => invocation.set_autoswitch(mode.into(), ui)?,
                None => invocation.show_autoswitch(ui)?,
            },
            Some(SettingsCommand::Verbose { state }) => {
                preference(&invocation, Preference::Verbose, state, ui)?
            }
            Some(SettingsCommand::Welcome { state }) => {
                preference(&invocation, Preference::Welcome, state, ui)?
            }
        },
        Command::Internal { command } => match command {
            InternalCommand::Startup => invocation.startup(ui)?,
            InternalCommand::Enter => invocation.enter_repository(ui)?,
            InternalCommand::ShellEnter { startup } => invocation.shell_entry(startup, ui)?,
            InternalCommand::Poll => invocation.poll_background(ui)?,
            InternalCommand::Worker { request } => invocation.background_worker(request)?,
            InternalCommand::Welcome => invocation.welcome(ui)?,
            InternalCommand::Hook { name, args } => invocation.handle_hook(&name, &args, ui)?,
            InternalCommand::Git { args } => return invocation.forward_git(&args, ui),
        },
        Command::Shell { command } => match command {
            ShellCommand::Completions { shell } => {
                clap_complete::generate(shell, &mut Cli::command(), "mgh", &mut std::io::stdout());
            }
            ShellCommand::Init { shell } => {
                let kind = match shell {
                    InitShell::Fish => ShellKind::Fish,
                    InitShell::Bash => ShellKind::Bash,
                    InitShell::Zsh => ShellKind::Zsh,
                };
                print!("{}", invocation.shell_initialization(kind)?);
            }
        },
    }
    Ok(0)
}

fn preference(
    invocation: &Invocation,
    pref: Preference,
    state: Option<OnOff>,
    ui: &mut dyn Interaction,
) -> Result<()> {
    match state {
        Some(state) => invocation.set_preference(pref, state.enabled(), ui),
        None => invocation.show_preferences(Some(pref), ui),
    }
}

impl From<Fields> for IdentityFields {
    fn from(fields: Fields) -> Self {
        Self {
            username: fields.username,
            email: fields.email,
            name: fields.name,
        }
    }
}

impl From<AutoswitchMode> for Autoswitch {
    fn from(mode: AutoswitchMode) -> Self {
        match mode {
            AutoswitchMode::Off => Self::Off,
            AutoswitchMode::Foreground => Self::Foreground,
            AutoswitchMode::Background => Self::Background,
        }
    }
}
