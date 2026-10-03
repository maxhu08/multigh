# Usage

`mgh` requires Git 2.45 or newer and the GitHub CLI (`gh`). Add accounts with
`mgh new`, or sign in with `gh auth login` when configuring accounts manually.
Installation and a complete multi-account example are in the
[getting started guide](../README.md#getting-started).

## Install and update

multigh is not yet published on crates.io. From this repository, with Rust installed:

```sh
cargo install --path . --locked --root "$HOME/.local"
```

Add `~/.local/bin` to PATH. Re-run this after updating the source. If you move the
installed binary, run `mgh setup` again to refresh the hooks' executable path.

## Accounts

Run `mgh new` to create or extend your account configuration, or copy
[`example/accounts.conf`](../example/accounts.conf) to
`~/.config/multigh/accounts.conf` and replace the example identities with your own.
The private file stays outside this repo; private `accounts.conf` copies here are
also gitignored. Account names are arbitrary, case insensitive, and must be unique
ignoring case. They start with a letter and contain letters, digits, underscores or
hyphens. Add as many accounts as you need.

Each INI section has `username` (GitHub login), `email` (default commit email),
optional `name` (defaults to username), and optional `allowed_emails` (additional
emails on indented lines). The default email is automatically allowed. Usernames
and emails must be distinct between accounts. Use addresses belonging to the
matching account, including noreply addresses needed by older commits.
Tokens and passwords remain in `gh`.

### Add an account

```sh
# Enter all account details through terminal prompts.
mgh new

# Choose the alias here and enter the remaining fields through prompts.
mgh new personal

# Supply required fields directly; name defaults to the username.
mgh new school --username school-login --email school@example.edu

# Supply a commit name and authorize the current repo too.
mgh new work --username work-login --email work@example.com --name "Your Name" --repo
```

The command shows **Add new identity** and asks for missing fields in a cliclack
form, with a colon and space after each label and input on the following line.
The commit-name label shows
`Commit name (default: <username>):`, using the entered GitHub username.
An empty commit-name response uses the GitHub username. Esc or Ctrl+C cancels
without saving. Interactive forms require terminal input, stdout and stderr;
cliclack renders prompts on stderr. Without a terminal, supply the alias, `--username` and
`--email`; `--name` is optional. Existing aliases, usernames and emails cannot be
reused. Alias lookup and uniqueness ignore case.

A successful existing `gh` login is reused. Otherwise, `gh auth login --hostname
github.com --web` runs with terminal input and output available. Authorize the
requested username in the browser; `mgh` verifies that login before saving.
Validation, cancellation and authentication failures leave the accounts file and
Git settings unchanged. Browser authentication itself can change gh's active
account even if later verification fails.

The account file is created if absent or appended without rewriting its existing
text, then saved with private permissions. Its filepath is printed. If it is a
symlink, use `--config` with the target instead. Edits made to the accounts file
during authentication are preserved and cause the command to stop.

After saving, `mgh new` runs setup and switches to the new account. Setup enables
protections and verbose and installs hooks; switching changes global Git identity
and local identity only when allowed. `--repo` explicitly adds the account to the
current repository's allowed list; outside a repository it is rejected before
authentication or saving. Other repositories retain their restrictions. Setup
and switch keep their usual output and generated-by comments. If either fails,
the account remains saved; fix the reported problem and rerun setup and switch,
using the same `--config` if applicable.

`XDG_CONFIG_HOME` changes the default config directory. Override the file with
`mgh --config /path/to/accounts.conf ...`. Generated identity files, hooks and
preference markers use `XDG_STATE_HOME/multigh`, defaulting to `~/.local/state/multigh`.

## Setup and switching

```sh
# Generate identity files, install shared hooks and enable both modes.
mgh setup

# Switch gh authentication and global Git identity.
mgh switch personal

# Also add personal to the current repository's allowed accounts.
mgh switch personal --repo

# Show live authentication, effective identity and repository protections.
mgh status

# Also show gh's full authentication report.
mgh status --full
```

Status lists signed-in GitHub accounts under **Identities**, with one block per
account: GitHub login and configured commit email on the first line, then the
identity filepath. The active account has **(Active)** in green when color is
available; other entries have no marker. Logins match configured accounts ignoring
case. Unmapped accounts or unavailable account configuration show `not configured`
for the email and `Identity not configured` for the filepath. **Accounts** is a
purple heading with the account config path beneath it. Status omits global
commit defaults and retains the current repository's effective identity,
protection checks and warnings.

Setup groups the Accounts path, global Git config filepath, generated Identity
paths and global updated/unchanged notice at the top, before the ON states and
mode explanations. The global filepath comes from Git, including custom locations
and XDG fallback. The final shell integration guidance shares the output's
indentation. Setup leaves the account file intact. Re-running it after editing the
account file refreshes identity files and re-enables protections and verbose
output.

Switching refreshes conditional identity rules for repos owned by configured
accounts on github.com over HTTPS or standard SSH URLs. In the current repo it
updates local identity when the new account is allowed. Other repos keep their
local identities. Conflicting conditional identity rules are reported.

Setup, switching and enabling protections print `Global Git config updated by ...`
once when they change global settings, including removal of obsolete identity
rules. The notice also appears if changes completed before a later operation
fails. Successful commands print `Global Git config unchanged` when the global
settings already match, even if identity files or local settings change.
Failed commands that made no global changes report their error without an
unchanged confirmation.

New or changed global entries have inline provenance comments, for example:

```gitconfig
[core]
    hooksPath = /home/you/.local/state/multigh/hooks # Generated by multigh (mgh setup)
```

Comments name the originating mgh subcommand; switch includes its account argument
and `--repo` when supplied. They describe the operation rather than reproducing
global options such as `--config`. Generated identity files are also annotated.
Matching existing entries keep their previous comments, including entries created
before this feature. Unrelated settings and comments are preserved. Git performs
the writes, respecting its normal global config location, `GIT_CONFIG_GLOBAL` and
symlink targets. Local repository policy and hook integration keep their existing
format.

## Protections and verbose

`mgh protections on` installs shared Git hooks and enables account enforcement.
Normal clones prompt after checkout. Shell integration prompts when entering an
unconfigured repo or starting a terminal there. These checklists use cliclack.
Use arrows to move, Space to toggle accounts, Enter to save at least one, or Esc
or Ctrl+C to cancel and leave it blocked. Saved selections start checked when
reopening a form. Long lists scroll with seven accounts visible at a time.
Empty selection is rejected with `Input required` until you select an account
or cancel. Forms honor `NO_COLOR`.

```sh
# Show the mode and this repository's allowed accounts.
mgh protections

# Open the account checklist, including to change existing selections.
mgh protections --repo

# Replace the list directly without a terminal picker.
mgh protections --allow personal,school,work

# Disable enforcement while keeping selections and original hooks.
mgh protections off

# Show or change allowed-account entry output independently.
mgh verbose
mgh verbose off
mgh verbose on

# Manually verify current authentication and commit identity.
mgh check
```

`mgh check` validates even when automatic protections are off. Outside a repository
it does nothing. Inside an unconfigured repo it reports an error. The push hook
additionally checks outgoing commit history; `check` does not.

[How the guard works](../README.md#how-the-guard-works) explains local Git settings,
hook forwarding, legacy pins, identity checks and coverage limits. Git hook-aware
clones use the shared hooks. Repo-specific paths such as Husky's `.husky/_` are
saved as `mgh.originalHooksPath` in the local or worktree Git config scope and
chained automatically on entry while protections are on. Setup and `mgh protections on` integrate the current repo too.
If a tool resets its path, re-enter the repo or run `mgh protections on` to repair
it before committing or pushing. Original hook files are preserved; turning
protections off still forwards to them. Custom global hook paths require manual
integration, and clients bypassing hooks cannot be intercepted. Entry and status
verify the effective path and all managed launchers before reporting protections
as active. Missing, non-executable or changed launchers are reported individually;
run `mgh setup` to recreate missing launchers and restore executable permissions.
Unrecognized replacement files are preserved and need manual review.

With verbose on, existing hooks are listed one per line under “Existing hooks
detected”, with paths and inactive hooks marked as skipped. Husky lists actual
project scripts rather than empty generated wrappers. The active-protection
confirmation appears only after successful verification. Reports remain quiet
when verbose is off, except for protection failures.

Noninteractive, bare, empty or `--no-checkout` clones can remain unconfigured after
download. Select accounts later in a terminal or with `--allow`;
commits and pushes stay blocked in the meantime.

## Shell integration

Load one of these in the interactive shell's config, after `mgh` is on PATH:

```fish
# Fish: ~/.config/fish/config.fish.
mgh init fish | source
```

```sh
# Bash: ~/.bashrc.
eval "$(mgh init bash)"

# Zsh: ~/.zshrc.
eval "$(mgh init zsh)"
```

Fish watches PWD changes. Bash adds a prompt handler without replacing your
existing PROMPT_COMMAND. Zsh adds a chpwd handler. Each checks the starting repo
as well. These handlers follow the saved protections and verbose toggles.
Restart the shell or load its updated config to activate them.

The optional welcome is separate. Its heading is **Identity**, followed by the
selected account and commit email. It reads gh's locally selected account for fast
startup; enforcement and status use live authentication. Toggle it with
`mgh welcome on` or `mgh welcome off`. In Fish, add this to `fish_greeting`:

```fish
if command -sq mgh
    command mgh welcome
end
```

For Bash or Zsh, add `mgh welcome` to interactive startup. The welcome preference
is stored alongside the protections and verbose markers.

## Help and completions

Every command's `-h`, `--help` and `mgh help <command>` describe behavior and examples.
Generate completions with `mgh completions fish`, `mgh completions bash`, or
`mgh completions zsh`, then save them in the shell's completion directory.
