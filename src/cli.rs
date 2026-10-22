use clap::{Args, Parser, Subcommand, ValueEnum};
use std::{ffi::OsString, path::PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "mgh",
    version,
    color = clap::ColorChoice::Never,
    about = "Global GitHub identities and explicit repository permissions",
    after_help = concat!(
        "First setup: mgh setup\n",
        "Add an identity: mgh identity new --username alice --email alice@example.com personal\n",
        "Choose repository permissions: mgh repo allowed update\n",
        "\n",
        "Identity config: $XDG_CONFIG_HOME/multigh/identities.jsonc (default: ",
        "$HOME/.config/multigh/identities.jsonc)\n",
        "State: $XDG_STATE_HOME/multigh (default: $HOME/.local/state/multigh)\n",
        "USERPROFILE is used when HOME is absent. Protections default ON; global autoswitch ",
        "defaults OFF.",
    )
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        help = "Use this absolute or invocation-relative JSONC identity configuration"
    )]
    pub config: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Onboard a first identity interactively or refresh integration; preserve permissions and preferences
    #[command(
        after_help = concat!(
            "Requires a terminal for first-identity onboarding; otherwise use mgh identity new ",
            "--username <username> --email <email> <identity>. Refreshes integration without ",
            "granting repository permissions. First completed setup enables verbose once. A later ",
            "failure preserves completed work; rerun mgh setup.",
        )
    )]
    Setup,
    /// Inspect identities, live GitHub authentication and the current repository without changing state
    #[command(
        after_help = concat!(
            "Read-only. Uses live GitHub account information and fails on query failure or a ",
            "missing active account. Bad identity configuration is reported while available ",
            "account information remains visible. --full shows indented native authentication ",
            "output, preserving colors, separate streams and failure status. Repair integration ",
            "with mgh setup.",
        )
    )]
    Status {
        #[arg(
            long,
            help = "Include native authentication stdout/stderr; fail if authentication fails"
        )]
        full: bool,
    },
    /// Switch global account/name/email; only update a repository that already permits this identity
    #[command(
        after_help = concat!(
            "Global scope: selects GitHub authentication and commit defaults. Repository policy is ",
            "validated first. Only an already permitted identity changes local details; no ",
            "permissions are granted. Earlier writes can survive a later error. Use mgh repo ",
            "allowed update to explicitly authorize identities.",
        )
    )]
    Switch { identity: String },
    /// Diagnose tools, configuration, live authentication and hook readiness without repairs
    #[command(
        after_help = concat!(
            "Read-only and noninteractive. Accumulates independent problems and returns failure ",
            "when unhealthy. Requires Git 2.45+, GitHub CLI, nonempty valid identities, live ",
            "authentication and healthy hooks. Repair the reported issues, then run mgh setup.",
        )
    )]
    Doctor,
    /// Manage global identities; defaults to list (permission changes use mgh repo allowed)
    #[command(
        after_help = concat!(
            "Global definitions; the omitted subcommand means list. Creation authenticates and ",
            "selects, editing verifies login without switching, and removal preserves ",
            "authentication and repository permissions. To grant repository access use mgh repo ",
            "allowed update.",
        )
    )]
    Identity {
        #[command(subcommand)]
        command: Option<IdentityCommand>,
    },
    /// Inspect/change this repository's explicit permissions and local protection preference
    #[command(
        after_help = concat!(
            "All repository commands require a repository before loading identity configuration. ",
            "Policy belongs to the common repository config, including linked worktrees. The ",
            "omitted subcommand means status. Authorize explicitly with mgh repo allowed add ",
            "<identity> or mgh repo allowed update.",
        )
    )]
    Repo {
        #[command(subcommand)]
        command: Option<RepoCommand>,
    },
    /// Read/change independent global preferences (initially OFF)
    #[command(
        after_help = concat!(
            "All preferences are global and independent. Autoswitch, verbose and welcome initially ",
            "default OFF; first completed setup enables verbose once. Omitted states display the ",
            "value. Settings survive later entry failures and subsequent setup.",
        )
    )]
    Settings {
        #[command(subcommand)]
        command: Option<SettingsCommand>,
    },
    /// Emit configuration-independent shell initialization or completions
    #[command(
        after_help = concat!(
            "Initialization and completions require no identity config or home directory. ",
            "Examples: mgh shell init fish | source; eval \"$(mgh shell init bash)\"; eval \"$(mgh ",
            "shell init zsh)\". Only interactive initialization performs entry checks.",
        )
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

#[derive(Subcommand, Debug)]
pub enum IdentityCommand {
    /// List configured identities; account-query errors are warnings
    #[command(
        after_help = concat!(
            "Read-only global identity and account listing. Account-query errors warn and ",
            "continue. Configure a first identity with mgh identity new.",
        )
    )]
    List,
    /// Create, authenticate, save, install and select; missing fields require a terminal
    #[command(
        after_help = concat!(
            "Noninteractive creation requires the identity argument, --username and --email. ",
            "--name defaults to the normalized username. Missing interactive fields prompt in ",
            "identity/username/commit-name/email order. All terminal streams are required; ",
            "Escape/Ctrl+C cancels. Validation and verified login precede saving. A saved identity ",
            "survives later integration failure; recover with mgh --config <path> setup and switch ",
            "<identity>.",
        )
    )]
    New {
        identity: Option<String>,
        #[command(flatten)]
        fields: Fields,
    },
    /// Change supplied fields, or prompt with defaults; preserves permissions and account selection
    #[command(
        after_help = concat!(
            "Supply any field flags to change only those fields, or use a terminal to accept/edit ",
            "existing defaults in username/email/commit-name order. Preserves comments, formatting ",
            "and additional emails. Verifies login before saving and refreshes rules afterward, ",
            "without switching accounts or granting repository access. Cancellation leaves the ",
            "original intact.",
        )
    )]
    Edit {
        identity: String,
        #[command(flatten)]
        fields: Fields,
    },
    /// Remove an identity; preserve authentication and stored repository permissions
    #[command(
        after_help = concat!(
            "Unknown identities fail; removing the final identity is valid. Does not sign out or ",
            "change repository permissions. Stale permissions remain blocked; revoke them with mgh ",
            "repo allowed remove <identity> or update the checklist.",
        )
    )]
    Remove { identity: String },
}

#[derive(Args, Debug, Default)]
pub struct Fields {
    #[arg(long, allow_hyphen_values = true)]
    pub username: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    pub email: Option<String>,
    #[arg(
        long,
        allow_hyphen_values = true,
        help = "Commit name; defaults to the normalized username on creation"
    )]
    pub name: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum RepoCommand {
    /// Report policy, effective details and actual hook readiness
    #[command(
        after_help = concat!(
            "Read-only repository report with common config path, stored permissions, effective ",
            "commit details, protection readiness and global autoswitch. Unknown stored ",
            "permissions can be revoked with mgh repo allowed remove <identity>. Repair hooks with ",
            "mgh setup.",
        )
    )]
    Status,
    /// Check live authentication and actual author/committer, even with protections OFF; silent on success
    #[command(
        after_help = concat!(
            "Requires nonempty, fully valid repository permissions, a live allowed GitHub account ",
            "and matching actual author/committer details, including environment overrides. Runs ",
            "even when protections are OFF. Silent on success; failure includes mgh switch ",
            "guidance and --reset-author guidance for amending.",
        )
    )]
    Check,
    /// Read/change local protections; on installs hooks first, off needs no identity configuration
    #[command(
        after_help = concat!(
            "Repository-local scope; defaults ON. Omitted state displays the preference. OFF needs ",
            "no identity config and keeps project hooks active. ON installs/checks integration ",
            "before saving; failed enable preserves an existing OFF exception. Global and ",
            "command-scoped mgh.protections values do not disable repository checks. Repair with ",
            "mgh setup.",
        )
    )]
    Protections { state: Option<OnOff> },
    /// Manage explicit repository permissions; defaults to list
    #[command(
        after_help = concat!(
            "The omitted subcommand lists stored names, including stale references. Only explicit ",
            "add or saved checklist selection grants access. A nonempty resolved selection applies ",
            "local commit details, preferring the selected account, then the first sorted ",
            "identity. Authentication is never switched by permission operations.",
        )
    )]
    Allowed {
        #[command(subcommand)]
        command: Option<AllowedCommand>,
    },
}

#[derive(Subcommand, Debug)]
pub enum AllowedCommand {
    #[command(
        after_help = concat!(
            "Read-only stored permissions, including unresolved names. Missing entries or the ",
            "explicit empty marker mean no identities are authorized. Revoke stale values with mgh ",
            "repo allowed remove <identity>.",
        )
    )]
    List,
    /// Add a configured identity; never sign in or switch authentication
    #[command(
        after_help = concat!(
            "Noninteractive, case-insensitive and idempotent. Requires a configured identity and a ",
            "fully resolved proposed permission set. Saves local commit details without switching ",
            "authentication. Remove stale references before adding new permissions.",
        )
    )]
    Add { identity: String },
    /// Revoke a stored identity, including unresolved references; an empty set blocks checks
    #[command(
        after_help = concat!(
            "Noninteractive and case-insensitive; can revoke unresolved stored names even while ",
            "other stale references remain. Removing all permissions writes an explicit empty ",
            "policy. Current identity selection never restores access. Grant access explicitly ",
            "with add or update.",
        )
    )]
    Remove { identity: String },
    /// Replace permissions interactively; select at least one; Escape/Ctrl+C cancels
    #[command(
        after_help = concat!(
            "Needs terminal stdin, stdout and stderr. Arrow keys navigate, Space toggles and Enter ",
            "saves at least one choice; Escape/Ctrl+C cancels without saving. Existing permissions ",
            "start checked. For automation use mgh repo allowed add <identity>.",
        )
    )]
    Update,
}

#[derive(Subcommand, Debug)]
pub enum SettingsCommand {
    /// Global automatic selection of allowed identities; enabling also enters the current repository
    #[command(
        after_help = concat!(
            "Global scope, initially OFF. Enabling saves ON before entering the current ",
            "repository; that preference survives entry failure. Only permitted identities are ",
            "considered. One already selected account is a no-op; multiple choices always require ",
            "a terminal. Use mgh switch <allowed-identity> for manual recovery.",
        )
    )]
    Autoswitch { state: Option<OnOff> },
    /// Global informational entry reports
    #[command(
        after_help = concat!(
            "Global informational entry reports, initially OFF and enabled once by first completed ",
            "setup. Omitted state displays the value. Does not suppress warnings, authorization ",
            "forms or autoswitch. Requires no identity config or authentication.",
        )
    )]
    Verbose { state: Option<OnOff> },
    /// Global optional read-only greeting
    #[command(
        after_help = concat!(
            "Global opt-in greeting, initially OFF. Omitted state displays the value. Uses local ",
            "account selection without a live query and performs no repairs or account changes. ",
            "Requires no identity config to change the preference.",
        )
    )]
    Welcome { state: Option<OnOff> },
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum OnOff {
    On,
    Off,
}

impl OnOff {
    pub fn enabled(self) -> bool {
        matches!(self, Self::On)
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum InitShell {
    Fish,
    Bash,
    Zsh,
}

#[derive(Subcommand, Debug)]
pub enum ShellCommand {
    /// Source the emitted script in an interactive Fish/Bash/Zsh shell; noninteractive startup has no effects
    #[command(
        after_help = concat!(
            "Emit a script for interactive startup and directory changes, preserving existing Git ",
            "functions/aliases and prompt handlers. Repeated initialization does not duplicate ",
            "reports. Noninteractive sourcing has no effects. Native git init runs first; ",
            "cancellation leaves Git successful and the new repository blocked. Recover with mgh ",
            "repo allowed update.",
        )
    )]
    Init { shell: InitShell },
    #[command(
        after_help = concat!(
            "Emit native completions for Fish, Bash, Zsh, PowerShell or Elvish without reading ",
            "configuration, querying accounts or changing state. Interactive Fish/Bash/Zsh ",
            "initialization loads completions automatically.",
        )
    )]
    Completions { shell: clap_complete::Shell },
}

#[derive(Subcommand, Debug)]
pub enum InternalCommand {
    Enter,
    Welcome,
    Hook {
        name: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    Git {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
}
