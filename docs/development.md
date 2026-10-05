# Development

Read [AGENTS.md](../AGENTS.md) and the relevant documentation in `docs/` before
changing the project. Update affected docs, README and command help in the same
change when code changes their behavior or make their guidance inaccurate.

## Source layout

| Module | Purpose |
| --- | --- |
| `main.rs` | Parse arguments, dispatch commands and report errors. |
| `cli.rs` | Define clap commands, flags and help. |
| `commands/` | Implement global identities, repository controls, preferences, setup, diagnostics and shell integration. |
| `config.rs` | Load and validate identity mappings. |
| `git.rs` | Read Git settings and maintain conditional identity files. |
| `github.rs` | Read, verify browser login or switch GitHub CLI authentication. |
| `policy.rs` | Store allowed identities, choose them with cliclack, and select identities. |
| `hooks.rs` | Install shared hooks, dispatch checks and forward existing hooks. |
| `guard.rs` | Validate live authentication, commit details and outgoing history. |
| `settings.rs` | Store global preference markers and read repository-local toggles. |
| `output.rs` | Format output with spacing and optional color. |
| `process.rs` | Capture or inherit external command I/O and quote shell arguments. |
| `shell/` | Embedded Fish, Bash and Zsh integration for directory entry and Git initialization. |

Command modules mirror the public groups: `commands/identity/`,
`commands/repo/`, `commands/settings/` and `commands/shell/`. The dispatcher has
one route for each group. Removed root commands have no aliases or separate
implementations. A hidden `internal` group provides shell entry, welcome rendering
and hook dispatch; generated scripts use these handlers directly. The hidden Git
forwarder runs native Git before invoking entry behavior after initialization.

Every `repo` command checks for a repository before loading configuration or
mutating settings. Protections read local `mgh.protections`, defaulting to true;
invalid boolean values cause an error. Protections and permissions live in the
repository's common config and are shared by linked worktrees. Status obtains its
absolute config filepath through `git::local_path`, so linked worktrees and bare
repos report the file that stores local settings. Global autoswitch, verbose and
welcome preferences use state markers. Autoswitch defaults off and ignores the
obsolete local `mgh.autoswitch` key. Setup enables verbose once, then preserves
global preferences and explicit repository protection exceptions.

Setup with a missing or empty config uses the same first-identity form as
`identity new`; a populated config only refreshes integration. Identity editing
changes individual CST properties and preserves additional emails, comments and
other identities. Editing validates before login and checks for concurrent changes
before a private atomic save. Removal leaves GitHub authentication and repository
permissions intact; stale permission references fail closed until explicitly
updated. An empty global identity map is valid so users can remove their last
identity and onboard again.

Repository permission updates use the local `mgh.allowed-identity` list. Removing
its last entry saves an explicit empty value, preventing the old preferred
`mgh.current-identity` value from silently restoring permission. The checklist requires
at least one selection; individual removal can leave a repo with none.

Autoswitch runs only during repository entry or explicit enabling. A sole allowed
identity switches without a picker when needed. Multiple allowed identities always
use a cliclack select, highlighting the current one if allowed. Cancellation or
nonterminal input leaves authentication unchanged. Commit and push hooks enforce
policy without prompting or switching. Directory entry catches failures so the
shell remains usable while protections continue to reject invalid operations.

Successful autoswitch uses the shared switch implementation's compact report:
selected identity, GitHub account and global commit defaults. It suppresses the
successful global-config notice and returns a handled-selection flag so entry
does not append verbose hook or permission reports. Manual switching keeps its
full report. Errors and partial global-config changes remain reported. A sole
identity gets an explicit selection message, including when already active;
the latter case retains the existing no-switch/no-write behavior.

Doctor only reads configuration, authentication, tools and hook readiness. Reports
use shared output helpers; `output::columns` centralizes label padding and
a fixed absolute value column for headings, rows, changes and form titles, wrapping
long labels and aligning nested values; every warning continuation and full GitHub auth report
line receives the common indentation. Raw shell integration and completion output
remains executable text.

Full authentication reports use a child-only `CLICOLOR_FORCE=1` to preserve
GitHub CLI's ANSI colors and bold text while capturing both output streams for
indentation. Styling requires both streams to be color-capable terminals and
respects `NO_COLOR` and `CLICOLOR=0`; otherwise the child receives `NO_COLOR=1`,
including when inherited GitHub CLI environment settings force terminal output.
Other GitHub CLI calls retain their original environment and JSON behavior.

CLI summaries and options are clap attributes. Detailed help is embedded from
`docs/help/` into both `-h` and `--help`. All interactive forms use cliclack. The
identity checklist uses multiselect with Space toggling, saved initial selections,
seven visible rows and at least one selection required. Esc and Ctrl+C cancel
without saving; entry catches returned cancellation errors and keeps an
unconfigured repo blocked. Text inputs and checklists use cliclack intro/outro session framing.
Forms render on stderr, so `policy::interactive` requires stdin, stdout and stderr
to be terminals. Noninteractive configuration continues using clap options.

`commands/identity/new.rs` uses cliclack input prompts for missing identity fields, or accepts
clap options for noninteractive configuration. The heading is Add new identity;
labels use a colon and space and show the username in the commit-name default.
Native `default_input` displays and saves the commit-name default on Enter;
trimmed empty names also fall back to the username. Input is collected before
validation, preserving existing rejection behavior.
It validates existing config and the combined file through `Config::load` before
authentication. New fields must be single-line values. A private temporary file in the destination directory
preserves existing text and is atomically persisted after `github::login` confirms
a successful matching GitHub account, ignoring username case. Existing symlinks are
rejected, and edits made during login are detected before replacement.
GitHub login inherits terminal I/O so browser/device instructions remain visible.
After saving, the command reuses setup and switch, including their provenance
comments and output. Errors after saving include recovery guidance and retain the
identity. Creating an identity never expands repository authorization. Use `mgh repo allowed add` or `update` separately.

Status renders each configured identity name above labeled GitHub username,
commit name, commit email and generated filepath rows. It matches each identity's GitHub username to live
GitHub CLI accounts ignoring case, marking the active identity in green and
identities without a login as Not signed in. Unmapped GitHub logins are reported
separately. Global commit defaults are omitted; repository checks and full
authentication output remain intact. Welcome maps gh's locally selected login to
an identity name without a live authentication request, and prints that name
in green in the second column of the Identity heading. GitHub username and effective commit email appear on separate labeled rows.

Global mutations use `git::update_global` and its `GlobalConfig` writer to attach
the originating command and collect a single change notice, including changes
completed before an error. By default, successful commands with no global changes print an
unchanged confirmation; failed commands with no changes do not. Setup, switch and
`mgh repo protections on` share this writer. Setup calls its report after the final global
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
so the identity checklist can still read the keyboard. Explicit `--config` paths
are resolved without requiring the default home or config directory.

Git has no post-init hook. Interactive shell integration installs a small `git`
function only when no existing function or alias would be replaced. Ordinary
commands call Git directly; arguments containing `init` use `internal git`, which
forwards original arguments and inherited I/O, preserves Git failures, and confirms
that the actual command was init before doing anything else. Nonterminal calls
remain passthrough. The initialized target is resolved from Git's global and init
arguments. A temporary command-scoped shell alias lets Git establish the target
working directory and propagate command configuration to `internal enter`; the
alias is not written to any config file. Entry reuses existing permission,
autoswitch and verbosity behavior. Selection failures do not undo a successful
initialization. Direct executable calls and existing custom Git wrappers bypass
this integration.

`config::Identity` holds a configured identity's GitHub username, commit name,
commit email and allowed emails; `Config::identities` is keyed by identity name.
Identity names use lowercase keys and identity filenames internally. Lookups,
allowed-list entries and legacy pins ignore case. Duplicate identity keys differing
only in case are rejected before Git settings change. New writes use the local
Git keys `mgh.allowed-identity` and `mgh.current-identity`; underscores are invalid
in Git variable names. Repeated allowed keys represent multiple identity names.
Legacy `mgh.allowedAccount`, `mgh.account` and `ghguard.account` remain readable.
Entry, setup and repository permission/identity writes migrate old mgh keys,
preserving repeated values and explicit emptiness, then removing old keys.
Existing canonical values take precedence over stale old ones, preventing
removed access from returning. Status, welcome and guard checks stay read-only.
All these values are identity names, not GitHub usernames.

Configuration is loaded from `identities.jsonc`, with identity names as keys
and nested `commit.name`, `commit.email` and `commit.additional_emails` fields.
`jsonc-parser` uses typed Serde deserialization to reject unknown or duplicate
fields. A root map visitor normalizes identity keys and rejects exact and
case-colliding duplicates before values can overwrite them. Explicit parser
options allow comments and trailing commas, while rejecting other JSON extensions.
Primary and additional emails are merged into the runtime allowed-email set.

`mgh identity new` validates the existing configuration and edits its concrete syntax tree
using `jsonc-parser`'s `cst` feature. This preserves existing comments, field order
and formatting while appending an identity.
The pending file is validated before authentication and saved atomically with
private permissions only if the original file remains unchanged. Failed input or
login leaves the original bytes unchanged.

Identity configuration guidance is in [Usage](usage.md#global-identities). The sample config
contains brief user guidance; implementation explanations live here and in the
[guard documentation](../README.md#how-the-guard-works). Cargo manages Cargo.lock,
including its generated header. The root .editorconfig defines formatting rules.

## Checks

Install the development commit-message hook and follow the format in the
[commit guide](committing.md). Cocogitto validates new messages alongside mgh's
identity protections.

```sh
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

Tests are organized under `tests/identities/`, `tests/commands/`, `tests/guards/`,
`tests/hooks/` and `tests/shell/`, with shared isolated setup in `tests/support/`.
The [testing guide](testing.md) maps behaviors to files, lists prerequisites and
explains focused runs and isolation. Git and the shells are real; GitHub account
responses are simulated.
