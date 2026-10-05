# Development

Read [AGENTS.md](../AGENTS.md) and the relevant documentation before changing
behavior. [Usage](usage.md) describes the user contract; [testing](testing.md)
describes the regression suite and prerequisites.

## Start here

`main` parses `cli::Cli`, calls `commands::run`, and reports returned errors.
`commands::run` is the only CLI dispatcher. Public command folders mirror the
CLI groups; hidden shell and hook entry points use the same dispatcher.

Follow a command's entry function first. Its input, sequence of actions and report
live together in the owning command module. Reused actions and reports have separate
named functions in that module; workflow reuse calls those functions directly.
One-caller helpers stay in that file; shared configuration, policy, Git, GitHub
and hook helpers own reusable behavior.
Those helpers do not call command modules or open forms. There are no service
registries or interchangeable backend traits.

## Source ownership

| Location | Owns |
| --- | --- |
| [`cli.rs`](../src/cli.rs) | clap command definitions, argument types and embedded help. |
| [`commands/`](../src/commands/mod.rs) | Dispatch, command orchestration, forms, reports and recovery messages. |
| [`config.rs`](../src/config.rs) | Configuration paths, identity types and matching, named commit details, parsing and validation. |
| [`config/editor.rs`](../src/config/editor.rs) | JSONC edits, validated pending files, concurrent-change checks and atomic saves. |
| [`repository.rs`](../src/repository.rs) | Repository discovery, scoped Git operations, worktree/bare roots and common config paths. |
| [`policy.rs`](../src/policy.rs) | Permission interpretation, compatibility migration and local identity writes. |
| [`git.rs`](../src/git.rs) | Shared Git execution, optional configuration reads and repeated-entry parsing. |
| [`git/global.rs`](../src/git/global.rs) | Global configuration writes, provenance comments and change reporting. |
| [`git/identity_files.rs`](../src/git/identity_files.rs) | Generated identity files and conditional include rules. |
| [`github.rs`](../src/github.rs) | Typed account responses, selected/live authentication, login and switching. |
| [`guard.rs`](../src/guard.rs) | Commit details, named push-input/history parsing and outgoing-history enforcement. |
| [`hooks/install.rs`](../src/hooks/install.rs) | Managed launchers and preservation of local/worktree hook overrides. |
| [`hooks/readiness.rs`](../src/hooks/readiness.rs) | Shared launcher health and effective hook-path checks. |
| [`hooks/preserved.rs`](../src/hooks/preserved.rs) | Existing-hook discovery and forwarding arguments, input and rejection. |
| [`settings.rs`](../src/settings.rs) | Typed global preferences, setup completion and repository-local protections. |
| [`storage.rs`](../src/storage.rs) | Home/XDG directories, private permissions and executable-file checks. |
| [`process.rs`](../src/process.rs) | Captured/inherited external I/O, expected absence and shell quoting. |
| [`terminal.rs`](../src/terminal.rs) | Terminal availability for interactive forms. |
| [`output.rs`](../src/output.rs) | User-facing reports, warnings and errors. |
| [`shell/`](../shell/) | Fish, Bash and Zsh scripts embedded by `commands/shell`. |

`commands/identity/form.rs` shares identity text inputs. Permission forms and
reports belong to `commands/repo/selection.rs`; autoswitch's picker belongs to
`commands/settings/autoswitch.rs`. Policy helpers have no terminal dependency.

## Main call paths

| Trigger | Call path |
| --- | --- |
| Setup with identities | `commands::setup::run` → repository discovery → `setup::install` → identity files, hooks and setup preferences → `setup::report`. |
| Add an identity | `commands::identity::new::run` → `new::create` (form, editor, login, save) and `new::report` → repository discovery → `setup::install` and `setup::report` → `switch::select` and `switch::report`. |
| Edit or remove an identity | `commands::identity::edit::edit` or `edit::remove` → editor edit/remove → login for edits → shared save/report → identity-file refresh. |
| Manual or automatic switching | `commands::switch::run` or `settings::autoswitch::select` → `switch::select` (preflight, identity files, authentication, defaults, allowed local details) → `switch::report`. |
| Repository entry | `commands::enter::run` → repository discovery → `enter::enter` → readiness/repair → optional permission picker → optional autoswitch → entry report. |
| Commit or push hook | `commands::hook::run` → repository discovery → protection setting → guard → `hooks::preserved::forward`. |
| Initial clone checkout | `commands::hook::run` → `commands::enter::enter` with the discovered repository → preserved checkout hook. |
| Interactive Git initialization | shell Git wrapper → `commands::shell::git::run` → native Git → command-scoped alias → internal entry. |

Setup without identities calls `identity::new::create` for the first identity,
then explicitly installs integration, selects it and reports both actions. Identity
creation uses the same actions and reports without calling setup or switch command
entry points. These functions stay in their owning files. Each caller chooses the
report it needs and retains recovery guidance if integration fails after saving.

## Configuration ownership and scope

| Data | Storage and writer |
| --- | --- |
| Identity definitions | Private JSONC selected by `--config` or XDG/home defaults; only the shared editor saves command edits. |
| Identity files and include rules | State-directory files and global Git config; `git::identity_files::refresh`. |
| Allowed/current identity | Repeated local `mgh.allowed-identity` and local `mgh.current-identity`; `policy`. |
| Protections | Local `mgh.protections`, default true; repository commands. |
| Autoswitch, verbose, welcome | State markers addressed through `settings::Preference`; settings commands. |
| Hook overrides | Local/worktree `mgh.originalHooksPath` and `core.hooksPath`; hook integration preserves the original scope. |
| GitHub tokens | GitHub CLI storage; mgh does not store them. |

Every repo command requires a repository before loading identity configuration or
changing settings. Linked worktrees share the common repository config for
permissions and protections. `Repository::config_path` identifies that file;
hook overrides retain their local or worktree scope. Commands discover or require
the repository once and pass it to policy, settings, guards, hooks and reports.
Repository Git methods use the original invocation directory, preserving relative
`GIT_DIR` and `GIT_WORK_TREE` paths. The root resolves hook paths; the common
directory identifies shared config, including in bare repositories. Hooks that run
during initialization before discovery succeeds still use Git's environment to
forward existing hooks.

`policy::decode` is the single interpretation of canonical and legacy permission
values. Stored policy and the fast welcome report use it. Explicit permissions,
including an empty list, take precedence over pins and old allowed lists. Pins
must agree when they are the only source. Canonical names are case insensitive.
`policy::migrate` writes separately on entry, setup and permission/identity updates;
status, welcome and guards remain read-only. Legacy Git keys remain compatible.
Global identity and preference commands never add repository permissions.

`Config::load` reads a file and delegates to `Config::parse`; pending edits use the
same parser without rereading the original. Typed deserialization rejects unknown
or duplicate fields, including case-colliding identity keys. JSONC comments and
trailing commas are accepted. Identity names, usernames and commit fields are
validated, and identities cannot share usernames or accepted email addresses.
Empty identity maps are valid. Primary and additional emails form the accepted
email set; commit names default to usernames. `Identity::matches_username`,
`accepts_email` and `matches_commit` own the pure matching rules;
`Config::identity_for_username` resolves the configured name. Callers explicitly
choose local selected-account reads or live authentication. Push-history enforcement
retains its own rule for detecting a disallowed identity's names or emails.

## Workflow stages and failure guarantees

Identity commands first open one original snapshot and collect input. Editor
add/edit/remove methods preserve the concrete syntax tree and validate the entire
result before preparing a private temporary file. GitHub login occurs before
`PendingUpdate::save`. Saving rechecks the original bytes and symlink status;
changed, newly created or deleted originals are preserved. Dropping a pending
update removes its temporary file. Edits preserve additional emails, comments,
formatting and other identities.

Recoverable errors belong at boundaries: user input, configuration parsing,
filesystem access, repository policy and external commands. The editor keeps its
validated original snapshot private; its later syntax-tree access uses established
invariants instead of errors for invalid shapes already rejected on open. Edit and
remove callers resolve the identity before invoking the editor. A prepared edit
still validates new input and checks concurrent changes before saving. Policy
selection returns the resolved identity so guards and reports do not repeat lookup
validation. Authorization receives a nonempty set from the required checklist or
an add/remove command; those callers handle an empty set separately.

After a save, integration refresh or switching can still fail. The saved identity
remains available; identity creation reports recovery commands with its selected
config path. Removal preserves GitHub authentication and repository permissions,
so stale references remain blocked until the user updates them.

Switching first resolves the requested identity, validates repository permissions
and reads previous commit details. Invalid or conflicting permissions fail before
identity-file, authentication or configuration changes. It then refreshes identity
files, changes authentication, sets global commit defaults and applies local details
only for an already allowed identity. `SwitchResult` retains the resolved identity
and named previous details for a separate report; `CommitDetails` replaces positional
name/email arrays. These changes are not one transaction: completed global writes
are reported even if a later step
fails. First setup enables verbose once, then preserves preferences and explicit
local protection exceptions. `setup::install` returns the identity-file directory
for `setup::report`.

`git::global::update` creates a writer for one originating command. Matching values
are skipped. Changed entries use Git's native `config --comment` provenance;
removal leaves no stray inline comment. Completed writes are reported even if a
later step fails. Git chooses the global filepath, preserving home,
XDG, overrides and symlink locations. Config comments require Git 2.45+.

Optional process calls name their accepted missing-result exit codes. Other
failures retain command errors. Git config reads accept code 1 for no match;
repository discovery recognizes Git's no-repository diagnostic. Malformed Git
config is an error, not an absent repository. During initialization, hooks may
run before local config exists; protections still default on and preserved hooks
continue forwarding. Selected GitHub accounts are local reads; guards use live
authentication and never rely on a cached selection.

## Hooks, forms and output

Managed hooks keep their format marker. Readiness checks every launcher's
existence, executable permission and marker, plus the effective hook path.
Integration saves repo/worktree overrides in their original scope and rechecks
readiness. Global and command overrides remain untouched. Existing hooks keep
arguments, rejection behavior and working-directory path resolution. Pre-push
input is read once, checked and replayed as bytes; other hooks inherit input.
Husky and legacy backup hooks remain supported, with recursion checks.

Commit/push enforcement never prompts or switches accounts. Entry catches picker
and autoswitch failures so the shell stays usable; protections continue rejecting
invalid operations. A sole allowed identity avoids writes when already selected.
Multiple allowed identities require a selection. Autoswitch's boolean result means
selection was handled, including when the sole identity was already active;
`enter::enter` uses `selection_handled` to decide whether an entry report is needed.

`guard.rs` parses external push updates into `PushUpdate` and Git history into
`OutgoingCommit` before enforcing rules. Named OID, author and committer fields
keep parsing details out of the enforcement loop. Both parsers stay in that file.

Interactive forms use cliclack. `terminal::interactive` requires stdin, stdout and
stderr terminals. Forms preserve existing defaults and selections and cancel
without saving. Permission checklists require at least one allowed identity.
Noninteractive options remain supported.

Full GitHub authentication reports capture stdout and stderr. Status and doctor
only read; welcome uses selected local login without a live authentication request.

Interactive shell initialization loads clap-generated command and option
completions through `mgh shell completions`. Zsh initializes its completion system
only when `compdef` is unavailable and does not write a completion dump. Completion
loading is skipped in noninteractive shells and requires no identity configuration
or GitHub authentication.

Shell scripts preserve existing Git functions/aliases and prompt handlers. The
hidden Git forwarder runs Git first, keeps its failures, and verifies initialization
arguments before entering the target. A temporary command-scoped Git alias carries
working directory/configuration to entry without saving an alias. Noninteractive
calls pass through. Fish restores terminal input when initialization is piped into
`source`. Direct Git executable calls bypass the wrapper; shared hooks still apply.

## Adding a feature

1. Define the observable behavior and scope: global identity, global preference,
   repository policy, hook enforcement or shell integration.
2. Add arguments in `cli.rs` and route them through the owning command group.
3. Keep the command flow and one-caller helpers together. Reuse the existing storage
   or policy owner; extract shared behavior when multiple callers need it.
4. For identity fields, update parsing/validation and the shared editor. For policy
   compatibility, update the decoder and keep migration separate. Preserve existing
   permission and scope guarantees.
5. Add behavior tests in the matching `tests/` directory using `Sandbox`; use small
   unit tests for shared parsing and pending-save boundaries. Register new integration
   files in their owning `mod.rs`.
6. Update affected usage, README and `docs/help/` text. Update this call map only
   when an ownership boundary changes; update the testing guide when the harness,
   prerequisites or test organization change.
7. Run the required checks and inspect the diff for unrelated changes.

### Keep ordinary features local

Aim for fewer than ten changed files for an ordinary feature, including tests and
user documentation. This is a design target, not a limit on necessary changes.

| Feature | Typical places to change |
| --- | --- |
| Add a subcommand to an existing group | `cli.rs`, the owning command module, its existing behavior test file, and affected help/usage/README text. The root dispatcher already routes the group. |
| Add a global preference | `cli.rs`, `settings.rs`, `commands/settings/mod.rs`, a settings test file, and affected help/usage/README text. |
| Add a commit or push rule | `guard.rs`, a matching guard test file, and affected help/usage text. |
| Change an identity form | `commands/identity/form.rs` or the owning new/edit command, its matching test file, and affected help text. |

Prefer extending an existing behavior test file; a new file also needs registration
in its parent `mod.rs`. Do not create separate action, result or adapter files for
a single command. Add a module when it has a distinct responsibility that would
otherwise obscure its owner, or when behavior needs reuse. Shared helpers should
remove duplication without forcing unrelated command groups to change together.

## Checks

Follow the [commit guide](committing.md) for Conventional Commits and Cocogitto
hook setup. From the repository:

```sh
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

Tests use isolated temporary configuration, real Git/local remotes and simulated GitHub
responses. See [testing](testing.md) for focused runs and platform limits.
