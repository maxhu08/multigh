# multigh

`mgh` manages named identities across GitHub accounts and repositories, keeping
GitHub authentication and commit details aligned with each repo's permissions.
An identity is a name such as `personal`, `school` or `work`, mapped to a GitHub
username and commit details. Identity names are case insensitive.

Written in Rust; requires Git 2.45 or newer and the GitHub CLI. This project is
not yet published on crates.io. Install from this repository.

## Getting started

```sh
# Install from this repository with Rust installed.
cargo install --path . --locked --root "$HOME/.local"

# Create your first identity and install hooks.
mgh setup
```

Add `~/.local/bin` to PATH before running mgh if needed. Setup asks for an identity
name, GitHub username, commit name and email using cliclack. Sign in through the
browser if prompted. The configuration is saved privately; setup prints its path,
Git config paths and the next steps. With an existing config, setup refreshes its
rules and hooks without overwriting it. Initial setup enables verbose reports;
later setup preserves preferences and explicit repo protection exceptions.

Load the line for your shell in its configuration:

```fish
# Fish: ~/.config/fish/config.fish
mgh shell init fish | source
```

```sh
# Bash: ~/.bashrc
eval "$(mgh shell init bash)"

# Zsh: ~/.zshrc
eval "$(mgh shell init zsh)"
```

Open a new terminal. Inside a repo, select allowed identities when prompted, or:

```sh
# Select this repo's permissions with a checklist.
mgh repo allowed update

# Switch to an allowed identity.
mgh switch personal

# Add more global identities whenever needed.
mgh identity new
```

With shell integration loaded, `git init` also opens the allowed-identity
checklist after creating a repository. Use Space to select identities and Enter
to save. `git init <directory>` configures that directory, even before you enter it.

## Commands and scope

| Command | Purpose |
| --- | --- |
| `mgh setup` | Guided setup or refresh of identity rules and hooks. |
| `mgh status [--full]` | Global identities and current repo status; optional full authentication report. |
| `mgh switch <identity>` | Switch GitHub authentication and commit details. |
| `mgh doctor` | Read-only installation, config, authentication and hook diagnostics. |
| `mgh identity [list]` | List global identities. |
| `mgh identity new [identity]` | Add a global identity and authenticate. |
| `mgh identity edit <identity>` | Edit global username or commit details. |
| `mgh identity remove <identity>` | Remove a global identity, keeping its GitHub login. |
| `mgh repo [status]` | Show only this repo's status. |
| `mgh repo check` | Validate live authentication and actual commit details. |
| `mgh repo protections [on\|off]` | Toggle protection here; default ON. |
| `mgh repo allowed [list]` | List identities permitted here. |
| `mgh repo allowed add <identity>` | Permit one configured identity here. |
| `mgh repo allowed remove <identity>` | Revoke one permission here. |
| `mgh repo allowed update` | Edit permissions with a checklist; no arguments. |
| `mgh settings` | Show global preferences. |
| `mgh settings autoswitch [on\|off]` | Toggle identity selection on entry to every repo; default OFF. |
| `mgh settings verbose [on\|off]` | Toggle allowed-identity and hook reports globally. |
| `mgh settings welcome [on\|off]` | Toggle the fast terminal greeting globally. |
| `mgh shell init <fish\|bash\|zsh>` | Print shell integration. |
| `mgh shell completions <shell>` | Print shell completions. |
| `mgh help [command]` | Show help. |

Identity and settings commands are global. Every repo command requires a Git
repository and changes only that repo's policy. Bare groups show useful defaults;
toggle commands without a state show their current preference. `--config <path>`
is available throughout the command tree; `-h`, `--help` and `--version` are supported.
Old public command forms have been removed; use the groups above.

## Identity configuration

The default is `~/.config/multigh/identities.jsonc`, or the corresponding location
under `XDG_CONFIG_HOME`. **Both JSON and JSONC are accepted.** Use `--config` for
another filename. This is the complete [example](examples/identities.jsonc):

```jsonc
// Example configuration: choose any identity names and any number of identities.
// Names are case insensitive; personal, Personal and PERSONAL are equivalent.
// Schema: each identity requires username and a commit object.
// commit.name is optional and defaults to username.
// commit.email is required and is used for new commits; it is always accepted.
// commit.additional_emails is an optional array of other accepted commit emails.
{
  "personal": {
    "username": "alice",
    "commit": {
      "name": "Alice Example",
      "email": "alice@example.com",
      "additional_emails": [
        "123+alice@users.noreply.github.com"
      ]
    }
  },
  "school": {
    "username": "bob",
    "commit": {
      "name": "Bob Example",
      "email": "bob@example.edu",
      "additional_emails": [
        "123+bob@users.noreply.github.com"
      ]
    }
  },
  "work": {
    "username": "carol",
    "commit": {
      "name": "Carol Example",
      "email": "carol@example.org",
      "additional_emails": [
        "123+carol@users.noreply.github.com"
      ]
    }
  }
}
```

| Field | Meaning |
| --- | --- |
| `username` | Required GitHub username. |
| `commit.name` | Optional commit name; defaults to username. |
| `commit.email` | Required email used for new commits. It is always accepted by protections. |
| `commit.additional_emails` | Optional array of other accepted addresses, including older/noreply emails. |

Comments and trailing commas are supported; keys and strings require double
quotes. Unknown/duplicate fields, wrong types, invalid identity names, malformed
emails and shared usernames/emails are rejected. Config editing preserves comments,
formatting and other identities. Identity names are case insensitive everywhere.
Private configs are outside this repo; private JSON and JSONC copies are ignored.
Tokens remain in GitHub CLI storage.

```sh
# Supply identity details without form prompts.
mgh identity new work --username work-login --email work@example.com

# Edit only the supplied field; no flags opens an edit form.
mgh identity edit work --email new@example.com

# Remove global configuration without signing out of GitHub.
mgh identity remove work
```

New identities automatically refresh setup and select the account, preserving repo
permissions. Edits refresh identity rules; use `mgh switch` to apply local details.
Removal leaves repo permissions unchanged. A repo referencing a removed identity
stays blocked until updated using `repo allowed remove` or `update`. The last
identity may be removed; setup then offers to add a first identity again.

## Repository permissions and switching

Protections are **ON by default for every repo once hooks are installed**. Only
`mgh repo protections off` saves an OFF exception for that repo. It survives setup
and restarts; `on` restores enforcement. Existing project hooks keep running.

Allowed identities live in local Git config using these names:

```gitconfig
[mgh]
    allowed-identity = personal
    allowed-identity = school
    current-identity = personal
```

Repeat `allowed-identity` once per permitted identity. `current-identity` records
this repo's selected commit identity. Older mgh keys are read compatibly and
renamed on repo entry, setup or permission/identity changes without expanding
permissions. An explicitly empty allowed list stays blocked. Git config keys
use hyphens because Git does not accept underscores.

`mgh repo status` shows the repo config filepath,
including the common config used by linked worktrees and the config in bare repos.
Adding is idempotent; removal does not delete global configuration or switch
GitHub authentication. Removing the
last permission leaves the repo blocked. The checklist uses arrows, Space and
Enter; Esc/Ctrl+C cancels without saving. It requires at least one selection.

Autoswitch is a global preference, default OFF. Enable it once with
`mgh settings autoswitch on` to apply it to every repo. Enabling it checks the
current repo immediately. On entry or terminal startup, one allowed identity is
selected automatically unless already active. Multiple identities **always**
show a cliclack picker, initially highlighting the current identity if allowed.
Cancellation preserves the account; without a terminal, multiple choices print
manual-switch guidance. Switching failures are reported without breaking startup.
Autoswitch works independently of verbose and protections. An unconfigured repo
can prompt for permissions first while protections are on.

Successful autoswitch output ends after the selected identity, GitHub account
and global commit details, even with verbose enabled. Repository summaries,
existing-hook reports and successful global-config notices are omitted.
A sole allowed identity prints that it was the only choice and names it;
if already active, it is kept without switching again. Errors still appear.

Switching updates gh's active account and global commit defaults, plus local
commit details when the repo allows the identity. The active account is shared
across terminals. Commit and push hooks check it; they never switch accounts.

## How the guard works

Setup installs shared hooks under `~/.local/state/multigh/hooks` and sets global
`core.hooksPath`. Commit/merge hooks check live gh authentication and actual author
and committer details. Push hooks also reject outgoing commits belonging to your
other configured identities outside the allowed list; collaborator commits pass.
Missing/invalid configuration or empty permissions blocks protected operations.

Original executable hooks are forwarded arguments, input and rejection results.
Repo/worktree hook paths, including Husky's `.husky/_`, are preserved as
`mgh.originalHooksPath` and integrated on entry while protections are on. Paths
reset by other tools are repaired on re-entry or `repo protections on`. A custom
global path is preserved and reported for manual integration. Readiness checks
all managed launchers and the effective hook path before reporting protections.
Verbose lists detected hooks individually, including skipped non-executable files.
Cocogitto's repo commit-message hook continues running alongside identity checks.

Git hooks are local safeguards: clients that bypass hooks can bypass protection.
Normal clones can prompt after checkout; bare, empty, noninteractive or
`--no-checkout` clones may need `mgh repo allowed update` later. Their protected
commits and pushes remain blocked until configured.

Interactive `git init` prompts when protections are on and the initialized repo
has no allowed identities. Reinitializing a configured repo preserves its choices;
an explicit `mgh repo protections off` skips the permission prompt. Canceling
keeps the new repo but leaves protected commits and pushes blocked. Scripts,
`command git init`, direct Git executable calls and IDE initialization do not use
this shell prompt; configure those repos with `mgh repo allowed update`.
Existing custom `git` functions and aliases are preserved, so they also need
manual permission setup. Git has no native initialization hook.

Headings and detail rows use a shared formatter with aligned value columns and
no dot separators. Long labels wrap instead of shifting the value column.

Global Git mutations print updated/unchanged notices and annotate changed entries
and identity files with `Generated by multigh (command)`. Existing matching values,
unrelated settings and comments are preserved. Multiline warnings and full GitHub
reports are indented. `mgh status --full` preserves GitHub CLI colors and bold text
in terminals, respecting `NO_COLOR`, `CLICOLOR=0` and plain or redirected output.
Generated shell scripts/completions remain machine-readable.

## Welcome and development

Enable the greeting with `mgh settings welcome on` and put `mgh internal welcome`
in your interactive greeting. It prints a purple Identity label and green identity
in the second column, then GitHub username and commit email. It uses local gh
selection for speed; status and protections use live authentication.

```sh
cargo test --all-targets
```

- [Usage](docs/usage.md)
- [Development](docs/development.md)
- [Tests and prerequisites](docs/testing.md)
- [Commit conventions](docs/committing.md)
