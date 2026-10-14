# Usage

For installation and initial setup, see [Getting started](../README.md#getting-started).
The [command table](../README.md#commands-and-scope) lists all commands; use
`mgh help <command>` for details.

## Global identities

An identity is a name such as `personal`, `school` or `work`, with a GitHub account
and commit details. Identity names are case insensitive. Use `mgh identity` to list
them.

`mgh identity new` prompts for missing details, opens browser sign-in if needed,
runs setup and selects the new identity. Supply the identity name, `--username`,
`--email` and optional `--name` to enter details without prompts. The commit name
defaults to the GitHub username. Adding an identity does not give it permission
to use any repository.
If setup or switching fails after the identity is saved, follow the recovery
commands in the error message.

`mgh identity edit <identity>` prompts with the existing values. Supply flags to
change only those fields. Comments, formatting and additional emails in your
configuration are preserved. Run `mgh switch <identity>` afterward to apply the
updated commit details.

`mgh identity remove <identity>` removes the identity without signing out of
GitHub. Remove its permissions from affected repositories with
`mgh repo allowed remove <identity>`, or choose a new list with
`mgh repo allowed update`.

Use `mgh switch <identity>` inside or outside a repository to switch GitHub
accounts and global commit defaults. It also updates this repository's commit
details if the identity is allowed here. Switching does not change permissions.
The active GitHub account is shared across terminals, so switching in one affects
the others.

## Identity configuration

Identities are saved in `~/.config/multigh/identities.jsonc`, or
`$XDG_CONFIG_HOME/multigh/identities.jsonc` if `XDG_CONFIG_HOME` is set. Use
`--config <path>` to choose another file. You can edit it yourself or use the
identity commands. Both JSON and JSONC are accepted, including comments and
trailing commas.

For example:

```jsonc
{
  "personal": {
    "username": "alice",
    "commit": {
      "name": "Alice Example",
      "email": "alice@example.com",
      "additional_emails": ["123+alice@users.noreply.github.com"]
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

Each identity needs its own GitHub username and accepted email addresses; they
cannot be shared with another identity. See the [full example](../examples/identities.jsonc)
for multiple identities. GitHub tokens stay in GitHub CLI storage.

## Current repository

Run `repo` commands from inside a Git repository. `mgh repo` shows its active
identity, allowed identities, commit details and protection status.
`mgh repo allowed` lists permissions, including references to identities you have
removed from your global configuration.

```sh
mgh repo allowed add personal
mgh repo allowed add school
mgh repo allowed add work
mgh repo allowed remove school
mgh repo allowed update
mgh switch personal
```

In the checklist, use arrows to move, Space to select or deselect, and Enter to
save at least one selection. Esc/Ctrl+C cancels. Current permissions are
preselected. Removing the final allowed identity blocks protected commits and
pushes until you allow one again.

Protections are ON by default. Use `mgh repo protections off` to disable them only
in this repository; setup preserves that choice. Use `mgh repo protections on` to
repair hooks and re-enable checks. Linked worktrees share permissions and the
protection setting.

Run `mgh repo check` to check GitHub authentication and current commit details,
even when protections are off. It is silent on success. It does not check outgoing
commit history; push hooks do that.

## Autoswitch

`mgh settings autoswitch on|off` controls automatic switching for all repositories.
It is OFF by default. Enable [shell integration](#global-settings-and-shell-integration)
to switch when entering repositories. With one allowed identity, mgh selects it.
With several, it asks which to use. Canceling leaves the current account unchanged.
If a choice is needed outside an interactive terminal, run `mgh switch <identity>`
yourself.

Enabling autoswitch also checks the current repository and may ask you to choose
its allowed identities. Git commit and push hooks check the active identity;
they do not switch it for you.

## Global settings and shell integration

`mgh settings` shows your global preferences. Each setting accepts `on` or `off`;
omit the state to see its current value.

- `autoswitch` selects an allowed identity when entering a repository.
- `verbose` shows allowed identities and existing hooks on repository entry.
  Turning it off still allows errors and autoswitch prompts to appear.
- `welcome` enables the terminal greeting. Add `mgh internal welcome` to your
  shell's greeting configuration to display it.

Add the line for your shell to its configuration, then restart or reload the shell:

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

Shell integration checks identities at interactive startup and when entering a
repository. It also enables Tab completion for commands and options. For standalone
completion definitions or other supported shells, use `mgh shell completions <shell>`.

Running `git init` interactively opens the identity checklist when protections are
on and the repository has no allowed identities. Canceling leaves the repository
created, with protected commits and pushes blocked until you choose permissions.
Reinitializing a configured repository does not ask again.

Scripts, IDEs, `command git init` and calls to the Git executable by its full path
skip the checklist. Existing custom `git` functions and aliases are preserved and
may also bypass it. In these cases, run `mgh repo allowed update` yourself inside
the repository.

## Hook integration

Setup installs shared Git hooks to check GitHub authentication and commit details
before commits, merges and pushes. Push checks also look for outgoing commits
from your other configured identities that are not allowed in this repository.
Missing or invalid identity configuration and empty permissions block protected
operations.

Existing executable project hooks, including Husky and Cocogitto checks, continue
to run alongside mgh. If another tool changes the repository's hook settings,
re-enter the repository or run `mgh repo protections on` to repair integration.
Custom global hook paths require manual integration; setup reports this instead
of replacing them.

Clones can prompt for allowed identities after checkout. If no prompt appears,
run `mgh repo allowed update` before committing or pushing. These are local
safeguards: tools that bypass Git hooks can bypass the checks.

## Diagnostics

`mgh status` shows global identities and current repository settings without
changing them. Add `--full` to include the GitHub CLI authentication report.

Use `mgh doctor` to diagnose tool, configuration, authentication and hook problems.
Run `mgh setup` to refresh identity rules and hooks when repairs are needed.
If switching reports invalid or conflicting repository identity settings, correct
those settings before retrying.
