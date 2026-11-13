# multigh

`mgh` manages multiple GitHub accounts and keeps authentication and commit details aligned across repositories. Name identities `personal`, `school`, `work` or anything that fits your workflow. Identity names are case-insensitive.

This is a small personal utility, put together quickly. It has automated regression tests, but has not been rigorously tested across real-world environments or varied setups.

Requires Git 2.45+ and GitHub CLI. Installation requires Rust 1.89+; interactive shell integration supports Fish, Bash and Zsh on Unix.

## Getting started

From this checkout:

```sh
cargo install --path . --locked --root "$HOME/.local"
```

Add `~/.local/bin` to PATH if needed, then run:

```sh
mgh setup
```

Setup prompts for your first identity and opens GitHub sign-in when needed. Add more identities with `mgh identity new`.

Add the matching line to your shell configuration, then restart your shell:

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

This enables repository-entry checks and Tab completion. Inside each repository:

```sh
mgh repo allowed update  # Choose identities allowed to commit and push here.
mgh switch personal     # Select an identity you've allowed.
mgh status              # Inspect identities and the current repository.
```

Use Space to select identities and Enter to save; Esc or Ctrl+C cancels. Entering an unconfigured repository or running interactive `git init` also offers the checklist. After cloning, run `mgh repo allowed update` if no picker appears.

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

Generated identity files, hooks and preferences live in `$XDG_STATE_HOME/multigh`, defaulting to `~/.local/state/multigh`. GitHub tokens remain in GitHub CLI storage.

## Permissions and switching

Protections default to ON after integration is installed. A repository with no allowed identities blocks protected commits and pushes. Permissions are local to the repository and shared by linked worktrees; creating or switching an identity never grants permission.

```sh
mgh repo allowed add work
mgh repo protections off       # Disable protections only in this repository.
mgh repo protections on        # Repair integration and re-enable checks.
mgh settings autoswitch background  # Enable nonblocking automatic switching.
```

Autoswitch defaults to `off`. `foreground` selects the sole allowed identity, or prompts when several are allowed, and waits for switching to finish. `background` checks permissions immediately and switches without blocking the prompt; multiple choices require `mgh switch <identity>`, and only failures are reported afterward. Moving within the same repository stays silent. Entry feedback appears even when verbose reports are OFF.

GitHub's active account is shared across terminals. Integrated Git waits for background switching; direct `gh` commands and unwrapped Git bypass coordination. Verbose entry reports turn on after the first completed setup; the welcome greeting defaults to OFF.

Existing project hooks continue to run. Custom global hook paths require manual integration; setup reports them instead of replacing them. Push checks also reject outgoing commits attributed to your other disallowed configured identities. These are local safeguards: tools that bypass Git hooks can bypass the checks.

### Git LFS

From the root of a normal checkout (where `.git` is a directory), install LFS into the preserved repository hooks rather than mgh's shared hooks:

```sh
git -c core.hooksPath=.git/hooks lfs install --local
```

The override applies only to installation. Normal Git operations still use mgh's managed hooks; protected pushes run mgh checks before forwarding arguments and input to `git lfs pre-push`. Do not force LFS to overwrite the shared hooks or disable protections to install it.

For linked worktrees, run this from the main checkout, not the linked worktree where `.git` is a file. If mgh preserves a custom hook directory (for example, Husky), use `git lfs install --local --manual` and integrate LFS into those existing hooks instead; `.git/hooks` is not the forwarded directory in that case.

For problems, run `mgh doctor`, then `mgh setup` to refresh integration. For build and test instructions, see [Development](docs/development.md).
