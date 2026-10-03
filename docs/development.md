# Development

Read [AGENTS.md](../AGENTS.md) and the relevant documentation in `docs/` before
changing the project. Update affected docs, README and command help in the same
change when code changes their behavior or make their guidance inaccurate.

## Source layout

| Module | Purpose |
| --- | --- |
| `main.rs` | Parse arguments, dispatch commands and report errors. |
| `cli.rs` | Define clap commands, flags and help. |
| `commands/` | Implement account creation, switching, status, welcome, modes and repo entry. |
| `config.rs` | Load and validate account mappings. |
| `git.rs` | Read Git settings and maintain conditional identity files. |
| `github.rs` | Read, verify browser login or switch GitHub CLI authentication. |
| `policy.rs` | Store allowed accounts, choose them with cliclack, and select identities. |
| `hooks.rs` | Install shared hooks, dispatch checks and forward existing hooks. |
| `guard.rs` | Validate live authentication, commit identities and outgoing history. |
| `settings.rs` | Store preference markers in the private state directory. |
| `output.rs` | Format output with spacing and optional color. |
| `process.rs` | Capture or inherit external command I/O and quote shell arguments. |
| `shell/` | Embedded Fish, Bash and Zsh integration for directory entry. |

CLI summaries and options are clap attributes. Detailed help is embedded from
`docs/help/` into both `-h` and `--help`. All interactive forms use cliclack. The
account checklist uses multiselect with Space toggling, saved initial selections,
seven visible rows and at least one selection required. Esc and Ctrl+C cancel
without saving; entry catches returned cancellation errors and keeps an
unconfigured repo blocked. Text inputs and checklists use cliclack intro/outro session framing.
Forms render on stderr, so `policy::interactive` requires stdin, stdout and stderr
to be terminals. Noninteractive configuration continues using clap options.

`commands/new.rs` uses cliclack input prompts for missing account fields, or accepts
clap options for noninteractive configuration. The heading is Add new identity;
labels use a colon and space and show the username in the commit-name default.
Native `default_input` displays and saves the commit-name default on Enter;
trimmed empty names also fall back to the username. Input is collected before
validation, preserving existing rejection behavior.
It validates existing config and the combined file through `Config::load` before
authentication. New fields must be single-line values. A private temporary file in the destination directory
preserves existing text and is atomically persisted after `github::login` confirms
a successful matching account, ignoring username case. Existing symlinks are
rejected, and edits made during login are detected before replacement.
GitHub login inherits terminal I/O so browser/device instructions remain visible.
After saving, the command reuses setup and switch, including their provenance
comments and output. Errors after saving include recovery guidance and retain the
account. Only explicit `--repo` expands repository authorization.

Status renders signed-in accounts as stacked identity entries. It matches logins
to account configuration ignoring case, uses each configured commit email and
identity filepath, and marks the live active account in green. Missing mappings
remain visible as unconfigured. Global commit defaults are omitted from this
report; repository identity checks and full authentication output remain intact.

Global mutations use `git::update_global` and its `GlobalConfig` writer to attach
the originating command and collect a single change notice, including changes
completed before an error. Successful commands with no global changes print an
unchanged confirmation; failed commands with no changes do not. Setup, switch and
protections-on share this writer. Setup calls its report after the final global
write, beside its filepath summary and before mode explanations. The wrapper
reports at most once and retains partial-change reporting on errors. Setup reads
the selected global filepath from Git's `--global --no-includes --show-origin`
output, preserving Git's home, XDG, override and symlink location choices.
Matching values are skipped. Git's native
[`config --comment`](https://git-scm.com/docs/git-config#_options) option annotates
new or changed entries and generated identity files without parsing or rewriting
unrelated config text; this requires Git 2.45 or newer. Removed entries take their
inline comments with them. Local and worktree settings continue using the existing
Git helpers.

Shared hook launchers retain a managed marker. By default, existing repo hooks are
executed from the common Git directory. Local and worktree hook paths are saved as
`mgh.originalHooksPath` in their original config scope, and that scope's
`core.hooksPath` switches to shared hooks.
The dispatcher forwards to the saved directory, resolving relative paths from
Git's hook working directory. Entry repairs resets while protections are on;
setup and enabling protections also integrate the current repo. Canonical paths
prevent chaining back into shared hooks. Effective configuration is re-checked
after integration. Global and command overrides are preserved and reported.

Readiness verifies each managed launcher exists, is executable for the invoking
user, is readable, and retains its marker. Unix execution permission uses access,
matching Git's executable-hook check. Verbose reports enumerate preserved hooks,
including inactive entries; Husky's empty generated wrappers are omitted in favor
of actual project scripts. Legacy wrappers resolve to their original backups.
Pre-push input is read once for enforcement, then replayed to the original
hook. Other hooks inherit input normally. Only commit, merge-commit, push and
initial checkout have mgh behavior; other standard hooks forward existing hooks.

Fish entry restores terminal input when the init script is piped into `source`,
so the account checklist can still read the keyboard. Explicit `--config` paths
are resolved without requiring the default home or config directory.

Account aliases use lowercase keys and identity filenames internally. Lookups,
allowed-list entries and legacy pins ignore case. Duplicate sections differing
only in case are rejected before Git settings change.

Account configuration guidance is in [Usage](usage.md#accounts). The sample config
contains brief user guidance; implementation explanations live here and in the
[guard documentation](../README.md#how-the-guard-works). Cargo manages Cargo.lock,
including its generated header. The root .editorconfig sets four-space indentation.

## Checks

```sh
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

Tests are organized under `tests/accounts/`, `tests/commands/`, `tests/guards/`,
`tests/hooks/` and `tests/shell/`, with shared isolated setup in `tests/support/`.
The [testing guide](testing.md) maps behaviors to files, lists prerequisites and
explains focused runs and isolation. Git and the shells are real; GitHub account
responses are simulated.
