# multigh

`mgh` switches between GitHub accounts and keeps authentication and commit details aligned across repositories.

This is a small personal utility with automated tests, not a substitute for server-side protections.

## Install

You need Git 2.45+, GitHub CLI (`gh`), and Rust 1.89+. Install your platform's prerequisites, then run `cargo install` from this checkout. For shell integration, copy only your shell's profile command, without the leading `#`.

### macOS

Install [Homebrew](https://brew.sh/) if you don't already have it.

```sh
# Uncomment if you don't have Xcode or the Command Line Tools:
# xcode-select --install

# Git, GitHub CLI, and Rust.
brew install git gh rust

# Clone and enter the repository.
git clone https://github.com/maxhu08/multigh.git
cd multigh

# Install mgh. Make sure ~/.local/bin is on PATH.
cargo install --path . --locked --bin mgh --root "$HOME/.local"

# Uncomment for Zsh and add to ~/.zshrc:
# eval "$(mgh shell init zsh)"
# Uncomment for Bash and add to ~/.bash_profile:
# eval "$(mgh shell init bash)"
# Uncomment for Fish and add to ~/.config/fish/config.fish:
# mgh shell init fish | source
```

### Linux

```sh
# Install git, gh, and a C compiler and linker with your distro's package manager
# (e.g. build-essential provides the build tools on Debian and Ubuntu).
# Install Rust 1.89+ with cargo via your distro's packages or rustup.

# Clone and enter the repository.
git clone https://github.com/maxhu08/multigh.git
cd multigh

# Install mgh. Make sure ~/.local/bin is on PATH.
cargo install --path . --locked --bin mgh --root "$HOME/.local"

# Uncomment for Bash and add to ~/.bashrc:
# eval "$(mgh shell init bash)"
# Uncomment for Zsh and add to ~/.zshrc:
# eval "$(mgh shell init zsh)"
# Uncomment for Fish and add to ~/.config/fish/config.fish:
# mgh shell init fish | source
```

### Windows

```powershell
# Git for Windows and GitHub CLI via WinGet.
winget install -e --id Git.Git
winget install -e --id GitHub.cli

# Visual Studio Build Tools with the C++ workload for Rust's MSVC toolchain.
winget install -e --id Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install -e --id Rustlang.Rustup

# Reopen PowerShell to refresh PATH, then clone and enter the repository.
git clone https://github.com/maxhu08/multigh.git
cd multigh

# Install both binaries. Add %USERPROFILE%\.cargo\bin to PATH if needed.
cargo install --path . --locked --bin mgh --bin mgh-git

# Uncomment for PowerShell 5.1 or 7 and add to $PROFILE after any custom prompt setup:
# . ([ScriptBlock]::Create((mgh shell init powershell | Out-String)))
```

PowerShell checks for repository changes at the next prompt. Existing `git` aliases and functions are not replaced, and calling `git.exe` directly bypasses autoswitch coordination.

## First use

Run `mgh setup` to create your first identity and sign in. If you added a shell profile line, restart your shell afterward to enable repository-entry checks and Tab completion. Add more identities with `mgh identity new`.

Inside each repository:

```sh
mgh repo allowed update  # Choose identities allowed to commit and push here.
mgh switch personal     # Select an identity you've allowed.
mgh status              # Inspect identities and the current repository.
```

In the picker, use Space to select, Enter to save, and Esc or Ctrl+C to cancel.

## Commands

| Command | Purpose |
| --- | --- |
| `mgh setup` | Create the first identity or refresh integration. |
| `mgh status [--full]` | Show identities and repository status; optionally include GitHub's authentication report. |
| `mgh switch <identity>` | Switch GitHub accounts and global commit defaults; update local details only if allowed here. |
| `mgh identity [list\|new\|edit\|remove]` | Manage global identity definitions. |
| `mgh repo [status\|check\|allowed\|protections]` | Inspect or change this repository's permissions and protections. |
| `mgh settings autoswitch [off\|foreground\|background]` | Show or change automatic switching mode. |
| `mgh settings [verbose\|welcome] [on\|off]` | Show or change informational preferences; no arguments shows all settings. |
| `mgh doctor` | Diagnose tools, configuration, authentication and hooks without changing them. |
| `mgh shell <init\|completions> <shell>` | Print shell integration or completion definitions. |

Use `mgh <command> --help` for flags and subcommands. Editing an identity does not select it; run `mgh switch <identity>` afterward. Removing an identity preserves its GitHub login and repository permissions; revoke stale permissions with `mgh repo allowed remove <identity>`.

## Configuration

Identities live in `$XDG_CONFIG_HOME/multigh/identities.jsonc`, defaulting to `~/.config/multigh/identities.jsonc`. Both JSON and JSONC are accepted, including comments and trailing commas. See [the example configuration](examples/identities.jsonc). Use `mgh --config <path> ...` to select another file.

Each identity requires `username` and `commit.email`. `commit.name` defaults to the username; optional `commit.additional_emails` accepts older or noreply commit addresses without using them for new commits. Usernames and accepted email addresses must be unique across identities. Identity names start with an ASCII letter and may contain ASCII letters, digits, underscores and hyphens.

Generated identity files, hooks and preferences live in `$XDG_STATE_HOME/multigh`, defaulting to `~/.local/state/multigh`. On Windows, `%USERPROFILE%` is used when `HOME` is unset. GitHub tokens remain in GitHub CLI storage.

## Permissions and switching

Protections default to on after setup. Protected commits and pushes are blocked until the repository allows an identity. Linked worktrees share permissions; switching accounts never grants permission.

Autoswitch is off by default. Enable it with:

- `mgh settings autoswitch foreground` — switch before returning to the prompt; choose an identity if several are allowed.
- `mgh settings autoswitch background` — switch without blocking the prompt. If several identities are allowed, run `mgh switch <identity>` yourself.

GitHub CLI's active account is shared across terminals. Integrated Git waits for background switching; direct `gh` commands and unwrapped Git do not.

Existing project hooks still run. `mgh setup` reports custom global hook paths rather than replacing them.

### HTTPS credentials

To make HTTPS Git operations for `github.com` use the active `gh` account instead of another credential helper, run:

```sh
gh auth setup-git -h github.com
```

`mgh setup` does not change credential helpers. SSH remotes are unaffected.

### Git LFS

From the root of a normal checkout (where `.git` is a directory), install LFS into the preserved repository hooks rather than mgh's shared hooks:

```sh
git -c core.hooksPath=.git/hooks lfs install --local
```

Run this from the main checkout, not a linked worktree. If your project uses a custom hook directory (such as Husky), use `git lfs install --local --manual` and add LFS to those hooks instead. Don't overwrite mgh's shared hooks.

For problems, run `mgh doctor`, then `mgh setup` to refresh integration. For build and test instructions, see [Development](docs/development.md).
