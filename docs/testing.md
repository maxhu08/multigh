# Testing

Run the complete suite from the repository:

```sh
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

The integration suite requires Unix, Git, Fish, Bash, Zsh and Cocogitto (`cog`) on
PATH. Install Cocogitto 7 or newer using the [commit guide](committing.md). Git must
be version 2.45 or newer for config comments and support conditional remote identity
includes, worktree configuration and SHA-256 repositories. Shell tests run the
actual shells; missing shells fail the suite rather than silently skipping coverage.
The integration entry point is Unix-only
because its hooks, permissions and terminal harness use Unix APIs. Windows is not
verified by this suite.

## Unit tests

Small tests beside shared modules cover policy decoding and unchecked permissions,
pending identity saves, missing-identity operations, and expected versus unexpected
process failures through
[`utils/process.rs`](../src/utils/process.rs).
They complement the integration suite; they do not replace real Git, hooks or
terminal tests. Run just these tests with `cargo test --bin mgh`.

## Layout

`tests/integration.rs` registers one integration test binary. Extend the existing
file that owns the behavior; register a new file in its parent `mod.rs` when a
separate behavior warrants it. This guide describes ownership rather than listing
every test, so adding a test does not also require updating an inventory.

| Location | Behavior |
| --- | --- |
| `identities/` | Configuration syntax, validation, identity names, comments and paths. |
| `commands/identity/` | Creating, editing and removing identities, forms and atomic saves. |
| `commands/repo/` | Repository scope, permission changes, protections and the picker. |
| `commands/settings/` | Global preferences, welcome output and autoswitch. |
| `commands/shell/` | Shell-init syntax and configuration independence. |
| Other files in `commands/` | CLI parsing, setup, switching, status, doctor, hooks and shared output. |
| `guards/` | Commit details, outgoing history, push input and rejection behavior. |
| `hooks/` | Installation, readiness, preservation, cloning, legacy hooks, Husky and worktrees. |
| `shell/` | Real Fish, Bash and Zsh startup, directory events and Git initialization. |
| `fixtures/`, `support/` | Fictional identities, simulated GitHub responses, `Sandbox` and terminal helpers. |

These are functional regression tests, not a claim of complete line or branch
coverage. Assertions check observable outcomes: saved identity policy, commit and
push acceptance, unchanged settings on failure, forwarded input and terminal
output. Outgoing-history tests also check that rejected new refs were not created.

Keep tests focused on command behavior, saved configuration, permission boundaries,
external failures and hook/shell integration. Prefer functional outcomes over
presentation details or behavior owned by dependencies. Diagnostic tests should
check useful information without requiring a particular layout. Status behavior
tests normalize report whitespace; focused output tests cover alignment separately.
Preserve existing JSONC comments and formatting in tests because editing must retain
the user's file.
Cocogitto coverage checks coexistence with mgh; it does not retest Cocogitto's parser
or installer prompts.

## Isolation

Every test owns a temporary directory, including its home, Git configuration,
identity config and state. Fixture usernames and emails are fictional. A simulated
`gh` executable records calls and supplies GitHub account selection, simulated browser
login, live authentication and controlled failures. Git itself is real, with signing disabled in test config.
Only local file transport is allowed; remotes are temporary local repositories.
No real GitHub login, identity configuration or user repository is changed.

Directory names include spaces and an apostrophe to exercise quoting in generated
hook launchers and shell commands. Interactive tests use a native pseudo-terminal,
including arrow keys, Space, Enter, Escape and Ctrl+C. Each terminal session has a deadline
and cleans up its child process if the test fails.
Ctrl+C can be delivered as SIGINT before the prompt enters raw mode. Cancellation
tests verify unsuccessful exit and unchanged settings.

## Focused runs

```sh
# Run tests for one command.
cargo test --test integration commands::switch

# Run hook safety tests.
cargo test --test integration hooks::

# Run actual shell integration tests.
cargo test --test integration shell::

# Show output from a failing interactive test.
cargo test --test integration commands::repo::picker -- --nocapture
```
