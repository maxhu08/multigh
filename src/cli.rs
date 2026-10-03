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
        help = "Read identity mappings from this file (default: ~/.config/multigh/identities.jsonc)"
    )]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(
        about = "Add an identity and sign in to its GitHub account",
        after_help = include_str!("../docs/help/new.txt").trim()
    )]
    New {
        #[arg(help = "New case-insensitive identity name; omit to be prompted")]
        identity: Option<String>,

        #[arg(long, help = "GitHub username; omit to be prompted")]
        username: Option<String>,

        #[arg(long, help = "Default commit email; omit to be prompted")]
        email: Option<String>,

        #[arg(long, help = "Commit name; defaults to the GitHub username")]
        name: Option<String>,

        #[arg(
            long,
            help = "Also authorize the new identity for the current repository"
        )]
        repo: bool,
    },

    #[command(
        about = "Select an identity and switch GitHub authentication and commit details",
        after_help = include_str!("../docs/help/switch.txt").trim()
    )]
    Switch {
        #[arg(
            help = "Case-insensitive identity name in identities.jsonc, such as personal, school or work"
        )]
        identity: String,

        #[arg(long, help = "Also authorize this identity for the current repository")]
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
        about = "Toggle protections or choose this repository's allowed identities",
        after_help = include_str!("../docs/help/protections.txt").trim()
    )]
    Protections {
        #[arg(help = "Enable or disable identity enforcement; omit to show its status")]
        state: Option<Toggle>,

        #[arg(long, conflicts_with_all = ["state", "allow"], help = "Open the identity checklist for this repository")]
        repo: bool,

        #[arg(long, value_delimiter = ',', num_args = 1.., conflicts_with = "state", help = "Set allowed identity names without a picker, e.g. personal,school,work")]
        allow: Vec<String>,
    },

    #[command(about = "Toggle allowed-identity output on directory entry and terminal startup", after_help = include_str!("../docs/help/verbose.txt").trim())]
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
        about = "Check authentication and commit details against the selected identity in this repository",
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
        about = "Check live authentication and actual author/committer names and emails before a commit"
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
