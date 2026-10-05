# Usage

Start with [Getting started](../README.md#getting-started) and the
[command table](../README.md#commands-and-scope). Git 2.45+, GitHub CLI and mgh must
be on PATH. The project is not published on crates.io; install from source:

```sh
cargo install --path . --locked --root "$HOME/.local"
mgh setup
```

## Global identities

An identity is a case-insensitive configured name, separate from its GitHub
username and commit name. Use `mgh identity` to list them. `identity new` collects
missing details with cliclack, verifies existing/browser login, privately saves
config, refreshes hooks/rules and selects the identity. `--username`, `--email`
and `--name` support noninteractive creation; the name defaults to username.
It does not expand repo permissions. Cancelled/invalid input, failed authentication,
symlinks and concurrent config edits preserve the original file. Recovery guidance
is printed if setup or switching fails after saving.

`identity edit <identity>` prompts with existing values when no flags are supplied.
Flags change only their fields. Comments, formatting, additional emails and other
identities are preserved. Username login is verified before atomic saving. Edits
refresh rules; run `mgh switch <identity>` to apply repo details.

`identity remove <identity>` removes global configuration without signing out or
changing repo permissions. Clean affected repos with `repo allowed remove` or
`update`. Empty global config is supported; setup offers a new first identity.

## Identity configuration

The default configuration is `~/.config/multigh/identities.jsonc`, changed by
`XDG_CONFIG_HOME` or `--config`. Plain JSON works too, including `.json` files.
Real comments and trailing commas are accepted; unknown fields and invalid values
are errors. Existing comments and formatting are preserved during edits. Private
configs are gitignored, while `examples/identities.jsonc` is tracked.

Here is the full [example configuration](../examples/identities.jsonc):

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

Unknown or duplicate fields, wrong types, invalid identity names, malformed
emails and shared usernames/emails are rejected. Editing through mgh preserves
comments, formatting and other identities.

Generated identity files, hooks and global preference markers use
`XDG_STATE_HOME/multigh`, defaulting to `~/.local/state/multigh`. Tokens remain in gh.

## Current repository

Every `repo` command requires Git in the current directory. `mgh repo` defaults
to status, including the active identity, GitHub username, commit details,
protections, global autoswitch and the repo config filepath; bare repositories are supported. `repo allowed`
defaults to listing permissions and identifies stale references to removed global
identities so they can be revoked. `repo check` verifies
live authentication and actual author/committer details even with protections off.
Success is silent; unlike push hooks it does not scan outgoing history.

```sh
mgh repo allowed add personal
mgh repo allowed add school
mgh repo allowed add work
mgh repo allowed remove school
mgh repo allowed update
mgh switch personal
```

The UI is a cliclack multiselect: arrows move, Space toggles, Enter saves at least
one, Esc/Ctrl+C cancels. Saved selections start checked; longer lists scroll.
Adding/removing permissions preserves gh login. Removing the final permission
blocks protected commits and pushes. Explicit empty permissions do not fall back
to a previous selected identity.

Protections default ON; `repo protections off` stores an exception only here.
Setup never resets it. `on` reinstalls/repairs hooks and restores enforcement.
Protections and permissions use local Git config, shared by linked worktrees;
existing worktree hook overrides remain integrated in their original scope.

Repository permissions are stored as repeated `mgh.allowed-identity` values;
`mgh.current-identity` records the selected repo commit identity. Old mgh keys are
read compatibly and renamed on entry, setup or permission/identity changes.
Canonical values take precedence over stale keys, including an explicitly empty
allowed list. Read-only reports and guard checks do not rewrite configuration.

## Autoswitch

`mgh settings autoswitch on|off` controls automatic selection globally, default OFF.
Enable it once to apply to every repo; former per-repo autoswitch values are ignored.
Enabling it checks the repo now. Shell entry/startup switches to one allowed
identity unless already active. Several allowed identities always show a cliclack
single-select picker, highlighting the current one when allowed. Cancellation and
noninteractive multiple-choice entry do not switch accounts. Failures are reported
without breaking startup. Missing permissions can be configured first when
protections are on. Git commit/push hooks never perform automatic switching.

After successful selection, autoswitch shows only the identity confirmation,
GitHub account and global commit details. It omits subsequent repository/hook
reports and successful global-config notices, even with verbose on. With one
allowed identity it says it found only one and names the selection; if already
active it reports that and avoids switching again. Failures remain visible.

Switching changes gh authentication, global commit defaults and allowed local
commit details. The active gh account is shared across terminals. Permissions stay
unchanged. Explicit `mgh switch` is available inside or outside repos.

## Global settings and shell integration

`mgh settings` displays autoswitch, verbose and welcome preferences. Each subcommand accepts
on/off or shows its preference without a state. Verbose controls informational
entry reports, not errors or autoswitch prompts. Welcome controls the greeting;
add `mgh internal welcome` to your interactive greeting after mgh is on PATH.

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

Fish watches PWD, Bash preserves existing PROMPT_COMMAND handlers, and Zsh uses
chpwd. All check startup and ignore noninteractive shells. Restart or reload the
shell configuration after installing. Generate completions with
`mgh shell completions <shell>`; output remains unindented for shell consumption.

Interactive `git init` runs Git first, then opens the allowed-identity checklist
if protections are on and the initialized repo has no permissions. Space toggles
identities; Enter saves; Esc/Ctrl+C cancels without undoing initialization. A
canceled selection leaves protected commits and pushes blocked. Failed Git
initialization retains its exit status and does not open a form. Reinitializing
a configured repo does not ask again, and explicit local protections off is
respected. This works with target directories, `-C`, quiet mode, bare repos and
separate Git directories.

The prompt needs loaded shell integration and terminal input/output. Scripts,
`command git init`, absolute Git executable calls and IDE initialization bypass
the shell wrapper. Existing custom `git` functions and aliases are preserved.
For these cases, run `mgh repo allowed update` inside the initialized repository;
shared commit/push hooks still enforce permissions.

## Hook integration

Setup installs shared hooks under `~/.local/state/multigh/hooks` by default and
sets global `core.hooksPath`. Commit and merge hooks check live authentication
and author/committer details. Push hooks also reject outgoing commits belonging
to your other configured identities outside the allowed list; collaborator
commits pass. Invalid config or empty permissions blocks protected operations.

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

Git initialization prompts come from shell integration, since Git has no native
initialization hook. See [shell integration](#global-settings-and-shell-integration)
for supported calls and manual setup.

## Diagnostics and output

`mgh status [--full]` is read-only and includes global identities and current repo
status. `mgh doctor` diagnoses tools, config, authentication and hook readiness,
returning failure if problems are found. Full gh reports preserve stdout/stderr
and indent every line. In color-capable terminals, full reports retain GitHub CLI
colors and bold text. `NO_COLOR`, `CLICOLOR=0`, dumb terminals and redirecting either
output stream keep the full report plain. Headings and rows share an aligned
value column without dot separators; long labels wrap to preserve alignment.
Warnings indent continuation lines too. Changed values are highlighted;
NO_COLOR, dumb terminals and redirected output suppress colors.

Setup/switch report global Git changes once and annotate changed entries with the
originating command. Matching values retain existing comments. See
[Hook integration](#hook-integration) for original hook forwarding,
Husky integration, readiness checks, clone behavior and local-hook limitations.
