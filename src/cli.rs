use clap::{Parser, Subcommand, ValueEnum};
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
        help = "Read account mappings from this file (default: ~/.config/multigh/accounts.conf)"
    )]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(
        about = "Add a GitHub account, sign in and set up its identity",
        after_help = include_str!("../docs/help/new.txt").trim()
    )]
    New {
        #[arg(help = "New case-insensitive account alias; omit to be prompted")]
        account: Option<String>,

        #[arg(long, help = "GitHub username; omit to be prompted")]
        username: Option<String>,

        #[arg(long, help = "Default commit email; omit to be prompted")]
        email: Option<String>,

        #[arg(long, help = "Commit name; defaults to the GitHub username")]
        name: Option<String>,

        #[arg(
            long,
            help = "Also authorize the new account for the current repository"
        )]
        repo: bool,
    },

    #[command(
        about = "Switch GitHub authentication and global Git identity",
        after_help = include_str!("../docs/help/switch.txt").trim()
    )]
    Switch {
        #[arg(
            help = "Case-insensitive account section in accounts.conf, such as personal, school or work"
        )]
        account: String,

        #[arg(long, help = "Also authorize this account for the current repository")]
        repo: bool,
    },

    #[command(
        about = "Show identities and repository protection",
        after_help = include_str!("../docs/help/status.txt").trim()
    )]
    Status {
        #[arg(
            long,
            help = "Also run gh auth status for detailed authentication information"
        )]
        full: bool,
    },

    #[command(
        about = "Show or toggle the fast shell welcome",
        after_help = include_str!("../docs/help/welcome.txt").trim()
    )]
    Welcome {
        #[arg(help = "Enable or disable the welcome; omit to print it when enabled")]
        state: Option<Toggle>,
    },

    #[command(
        about = "Refresh identity rules and enable protections and verbose output",
        after_help = include_str!("../docs/help/setup.txt").trim()
    )]
    Setup,

    #[command(
        about = "Toggle protections or choose this repository's allowed accounts",
        after_help = include_str!("../docs/help/protections.txt").trim()
    )]
    Protections {
        #[arg(help = "Enable or disable account enforcement; omit to show its status")]
        state: Option<Toggle>,

        #[arg(long, conflicts_with_all = ["state", "allow"], help = "Open the account checklist for this repository")]
        repo: bool,

        #[arg(long, value_delimiter = ',', num_args = 1.., conflicts_with = "state", help = "Set allowed account aliases without a picker, e.g. personal,school,work")]
        allow: Vec<String>,
    },

    #[command(about = "Toggle allowed-account output on directory entry and terminal startup", after_help = include_str!("../docs/help/verbose.txt").trim())]
    Verbose {
        #[arg(help = "Enable or disable entry output; omit to show its status")]
        state: Option<Toggle>,
    },

    #[command(about = "Generate shell integration for directory entry and terminal startup", after_help = include_str!("../docs/help/init.txt").trim())]
    Init { shell: IntegrationShell },

    #[command(
        hide = true,
        about = "Handle entering a repository from shell integration"
    )]
    Enter,

    #[command(
        about = "Check authentication and commit identity in this repository",
        after_help = include_str!("../docs/help/check.txt").trim()
    )]
    Check,

    #[command(
        about = "Generate shell tab completions",
        after_help = include_str!("../docs/help/completions.txt").trim()
    )]
    Completions {
        #[arg(help = "Shell to generate completion definitions for; writes to stdout")]
        shell: Shell,
    },

    #[command(
        hide = true,
        about = "Run an internal check invoked by installed Git hooks",
        after_help = include_str!("../docs/help/hook.txt").trim()
    )]
    Hook {
        #[command(subcommand)]
        kind: Hook,
    },
}

#[derive(Subcommand)]
pub enum Hook {
    #[command(
        about = "Check live authentication and actual author/committer identities before a commit"
    )]
    Commit,

    #[command(
        about = "Check live authentication and outgoing commits using Git's push input on stdin"
    )]
    Push {
        #[arg(help = "Remote name supplied by Git")]
        remote: String,

        #[arg(help = "Actual push destination URL supplied by Git")]
        url: String,
    },

    #[command(hide = true)]
    Run {
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
