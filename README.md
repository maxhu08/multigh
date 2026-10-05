# multigh

`mgh` manages multiple GitHub accounts across repositories, keeping authentication
and commit details aligned. Name your identities anything you like, such as
`personal`, `school` or `work`. Identity names are case insensitive.

Requires Git 2.45+, GitHub CLI and Rust for installation. This project is not yet
published on crates.io; install from this repository.

## Getting started

From the cloned repository:

```sh
# Install mgh.
cargo install --path . --locked --root "$HOME/.local"

# Create your first identity and enable repository protections.
mgh setup
```

Add `~/.local/bin` to PATH if needed. Follow the setup prompts and sign in to
GitHub when asked. To add more identities, run `mgh identity new`, which also
runs setup automatically.

Add the line for your shell to its configuration, then open a new terminal:

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

These lines also enable Tab completion for mgh commands and options.

Inside a repository:

```sh
# Choose which identities can commit and push here.
mgh repo allowed update

# Switch to an allowed identity.
mgh switch personal

# Check your identity and repository settings.
mgh status
```

Use **Space** to select identities and **Enter** to save. Shell integration also
prompts when entering an unconfigured repo or running `git init`.

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

`identity` and `settings` apply globally; `repo` applies only to the current Git
repository. Use `-h` with any command for details.

## Identity configuration

Identities are saved in `~/.config/multigh/identities.jsonc`. **Both JSON and JSONC
are accepted.** Use `mgh identity new` and `mgh identity edit` to manage them, or
edit the file yourself. `--config <path>` selects another file.

See the [example with personal, school and work identities](examples/identities.jsonc)
and the [configuration reference](docs/usage.md#identity-configuration) for fields
and additional commit emails. GitHub tokens stay in GitHub CLI storage.

## Repository permissions and switching

Protections are **on by default** after setup. Commits and pushes require an
allowed identity with matching commit details. A repo with no allowed identities
stays blocked until you choose them.

```sh
# Allow another identity in this repo.
mgh repo allowed add work

# Disable protections only for this repo.
mgh repo protections off

# Re-enable them.
mgh repo protections on
```

To switch automatically when entering a repository:

```sh
mgh settings autoswitch on
```

Autoswitch chooses the only allowed identity, or asks you to select one when
several are allowed. It is off by default. GitHub's active account is shared
across terminals, so switching in one terminal affects the others.

## How the guard works

mgh installs Git hooks that check authentication and commit details before
committing or pushing. Repo permissions stay in local Git config; existing
project hooks, including Husky checks, are preserved. Editors that run Git hooks
use the same protections.

These are local safeguards; tools that bypass Git hooks can bypass the checks.
Run `mgh doctor` if protections are not working, or read the
[hook integration guide](docs/usage.md#hook-integration).

## More information

- [Usage and configuration](docs/usage.md)
- [Development](docs/development.md)
- [Tests and prerequisites](docs/testing.md)
- [Commit conventions](docs/committing.md)
