// SELF describes the containing command itself.
// subcommands groups descriptions of child commands.
// flags are switches without values, such as --full.
// options take named values, such as --username alice.
// args are values identified by position without a flag or option name, such as work in switch work.

pub const SELF: &str = "Manage GitHub identities across repositories";
pub const AFTER_HELP: &str = "Commands are placeholders and are not implemented yet";

pub mod subcommands {
    pub const ONBOARD: &str = "Set up identities, shell integration and preferences";
    pub const DOCTOR: &str = "Diagnose configuration, authentication, tools and hooks";

    pub mod status {
        pub const SELF: &str = "Display current identity, repo settings, and GitHub details";

        pub mod flags {
            pub const FULL: &str = "Include the full GitHub authentication report";
        }
    }

    pub mod switch {
        pub const SELF: &str =
            "Switch identity; omit the identity to choose an identity interactively";

        pub mod args {
            pub const IDENTITY: &str = "Identity to switch to";
        }
    }

    pub mod identity {
        pub const SELF: &str = "Manage global identities; defaults to listing them";

        pub mod subcommands {
            pub const LIST: &str = "List global identities";
            pub const NEW: &str = "Add an identity; omit its name to be prompted";
            pub const EDIT: &str = "Edit an existing identity";
            pub const REMOVE: &str = "Remove an identity";
        }

        pub mod options {
            pub const USERNAME: &str = "GitHub username";
            pub const EMAIL: &str = "Commit email";
            pub const NAME: &str = "Commit name";
        }
    }

    pub mod repo {
        pub const SELF: &str =
            "Manage this repository's multigh settings; defaults to showing status";

        pub mod subcommands {
            pub const STATUS: &str =
                "Show this repository's current multigh identity and protections settings";
            pub const CHECK: &str = "Check authentication and commit identity";
            pub const PROTECTIONS: &str =
                "Turn protections on or off; omit the state to display current protections state";

            pub mod allowed {
                pub const SELF: &str =
                    "Manage identities allowed in this repository; defaults to listing them";

                pub mod subcommands {
                    pub const LIST: &str = "List identities allowed in this repository";
                    pub const ADD: &str = "Allow an identity in this repository";
                    pub const REMOVE: &str = "Remove an identity's permission in this repository";
                    pub const UPDATE: &str =
                        "Choose allowed identities for this repository interactively";
                }
            }
        }
    }

    pub mod settings {
        pub const SELF: &str = "Manage global preferences; defaults to showing them";

        pub mod subcommands {
            pub const AUTOSWITCH: &str = "Turn automatic identity switching on or off; omit the state to display current autoswitch state";
            pub const VERBOSE: &str = "Turn repository-entry reports on or off; omit the state to display current verbose state";
            pub const WELCOME: &str = "Turn the terminal greeting on or off; omit the state to display current welcome state";
        }
    }

    pub mod shell {
        pub const SELF: &str = "Generate shell integration and completions";

        pub mod subcommands {
            pub const INIT: &str = "Print a Fish, Bash or Zsh script for repository-entry checks, git init prompts and tab completion";
            pub const COMPLETIONS: &str = "Print standalone tab-completion definitions for multigh commands, options and fixed values";
        }
    }

    // Hidden commands called by shell integration and Git hooks.
    pub mod internal {
        pub mod subcommands {
            pub const ENTER: &str = "Handle repository entry";
            pub const WELCOME: &str = "Print the terminal greeting";
            pub const GIT: &str = "Forward Git arguments";
            pub const HOOK: &str = "Handle a Git hook";
        }
    }
}
