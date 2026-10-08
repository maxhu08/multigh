use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "mgh",
    version,
    about,
    arg_required_else_help = true,
    after_help = include_str!("../docs/help/mgh.txt").trim()
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        help = "Identity configuration (default: ~/.config/multigh/identities.jsonc)"
    )]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(
        about = "Set up identities, Git hooks and shell integration",
        after_help = include_str!("../docs/help/setup.txt").trim()
    )]
    Setup,

    #[command(
        about = "Show global identities and current repository settings",
        after_help = include_str!("../docs/help/status.txt").trim()
    )]
    Status {
        #[arg(long, help = "Include the full GitHub authentication report")]
        full: bool,
    },

    #[command(
        about = "Switch GitHub authentication and commit details",
        after_help = include_str!("../docs/help/switch.txt").trim()
    )]
    Switch {
        #[arg(help = "Case-insensitive identity name, such as personal, school or work")]
        identity: String,
    },

    #[command(
        about = "Diagnose configuration, authentication, tools and hooks",
        after_help = include_str!("../docs/help/doctor.txt").trim()
    )]
    Doctor,

    #[command(
        about = "Manage global identities; omit a subcommand to list them",
        after_help = include_str!("../docs/help/identity.txt").trim()
    )]
    Identity {
        #[command(subcommand)]
        command: Option<IdentityCommand>,
    },

    #[command(
        about = "Manage only this repository; omit a subcommand for status",
        after_help = include_str!("../docs/help/repo.txt").trim()
    )]
    Repo {
        #[command(subcommand)]
        command: Option<RepoCommand>,
    },

    #[command(
        about = "Manage global preferences; omit a subcommand to show them",
        after_help = include_str!("../docs/help/settings.txt").trim()
    )]
    Settings {
        #[command(subcommand)]
        command: Option<SettingsCommand>,
    },

    #[command(
        about = "Generate shell integration and completions",
        after_help = include_str!("../docs/help/shell.txt").trim()
    )]
    Shell {
        #[command(subcommand)]
        command: ShellCommand,
    },

    #[command(hide = true)]
    Internal {
        #[command(subcommand)]
        command: InternalCommand,
    },
}

#[derive(Args, Default)]
pub struct IdentityFields {
    #[arg(
        long,
        help = "GitHub username; prompted for new identities, preserved for edits"
    )]
    pub username: Option<String>,

    #[arg(
        long,
        help = "Commit email; prompted for new identities, preserved for edits"
    )]
    pub email: Option<String>,

    #[arg(
        long,
        help = "Commit name; defaults to username for new identities, preserved for edits"
    )]
    pub name: Option<String>,
}

#[derive(Subcommand)]
pub enum IdentityCommand {
    #[command(about = "List global identity names and GitHub/commit details")]
    List,

    #[command(
        about = "Add a global identity and sign into GitHub",
        after_help = include_str!("../docs/help/new.txt").trim()
    )]
    New {
        #[arg(help = "New case-insensitive identity name; omit to be prompted")]
        identity: Option<String>,

        #[command(flatten)]
        fields: IdentityFields,
    },

    #[command(
        about = "Edit a global identity while preserving config comments",
        after_help = include_str!("../docs/help/edit.txt").trim()
    )]
    Edit {
        identity: String,

        #[command(flatten)]
        fields: IdentityFields,
    },

    #[command(
        about = "Remove a global identity without signing out of GitHub",
        after_help = include_str!("../docs/help/remove.txt").trim()
    )]
    Remove { identity: String },
}

#[derive(Subcommand)]
pub enum RepoCommand {
    #[command(about = "Show this repository's permissions, commit identity and hooks")]
    Status,

    #[command(
        about = "Verify authentication and commit identity, even with protections off",
        after_help = include_str!("../docs/help/check.txt").trim()
    )]
    Check,

    #[command(
        about = "Toggle protection here; default ON, omit state to show",
        after_help = include_str!("../docs/help/protections.txt").trim()
    )]
    Protections { state: Option<Toggle> },

    #[command(
        about = "Manage allowed identities here; omit a subcommand to list",
        after_help = include_str!("../docs/help/allowed.txt").trim()
    )]
    Allowed {
        #[command(subcommand)]
        command: Option<AllowedCommand>,
    },
}

#[derive(Subcommand)]
pub enum AllowedCommand {
    #[command(about = "Show identities permitted in this repository")]
    List,

    #[command(about = "Allow an existing global identity in this repository")]
    Add { identity: String },

    #[command(about = "Remove an identity's permission in this repository")]
    Remove { identity: String },

    #[command(about = "Choose allowed identities with a Space/Enter checklist; no arguments")]
    Update,
}

#[derive(Subcommand)]
pub enum SettingsCommand {
    #[command(
        about = "Toggle identity switching on entry to every repo; omit state to show",
        after_help = include_str!("../docs/help/autoswitch.txt").trim()
    )]
    Autoswitch { state: Option<Toggle> },

    #[command(
        about = "Toggle global repo-entry reports; omit state to show",
        after_help = include_str!("../docs/help/verbose.txt").trim()
    )]
    Verbose { state: Option<Toggle> },

    #[command(
        about = "Toggle the terminal welcome globally; omit state to show",
        after_help = include_str!("../docs/help/welcome.txt").trim()
    )]
    Welcome { state: Option<Toggle> },
}

#[derive(Subcommand)]
pub enum ShellCommand {
    #[command(
        about = "Print Fish, Bash or Zsh startup, entry and git init integration",
        after_help = include_str!("../docs/help/init.txt").trim()
    )]
    Init { shell: IntegrationShell },

    #[command(
        about = "Print tab completion definitions",
        after_help = include_str!("../docs/help/completions.txt").trim()
    )]
    Completions { shell: Shell },
}

#[derive(Subcommand)]
pub enum InternalCommand {
    Enter,
    Welcome,

    #[command(disable_help_flag = true)]
    Git {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    Hook {
        name: String,

        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Clone, ValueEnum)]
pub enum Toggle {
    On,
    Off,
}

#[derive(Clone, ValueEnum)]
pub enum IntegrationShell {
    Fish,
    Bash,
    Zsh,
}
