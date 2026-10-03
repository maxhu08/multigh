# Testing

Run the complete suite from the repository:

```sh
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

The integration suite requires Unix, Git, Fish, Bash and Zsh on PATH. Git must
be version 2.45 or newer for config comments and support conditional remote identity
includes, worktree configuration and SHA-256 repositories. Shell tests run the
actual shells; missing shells fail the suite rather than silently skipping coverage.
The integration entry point is Unix-only
because its hooks, permissions and terminal harness use Unix APIs. Windows is not
verified by this suite.

## Layout

`tests/integration.rs` registers the following modules as one integration test
binary. Each behavior has a focused file; shared setup stays in `support/`.

```text
tests/
    identities/
        jsonc.rs
        names.rs
        paths.rs
        validation.rs
    commands/
        check.rs
        cli.rs
        completions.rs
        git_config.rs
        hook.rs
        init.rs
        new.rs
        output.rs
        picker.rs
        protections.rs
        setup.rs
        status.rs
        switch.rs
        verbose.rs
        welcome.rs
    guards/
        commit.rs
        history.rs
        push.rs
        push_input.rs
    hooks/
        cloning.rs
        dispatch.rs
        health.rs
        husky.rs
        integration.rs
        legacy.rs
        reporting.rs
        worktrees.rs
    shell/
        bash.rs
        fish.rs
        zsh.rs
    fixtures/
        identities.jsonc
        gh.sh
    support/
        mod.rs
        terminal.rs
    integration.rs
```

## Coverage

| Behavior | Tests |
| --- | --- |
| Root help, version, invalid arguments and command help | `commands/cli.rs` |
| Every completion shell and each supported init shell | `commands/completions.rs`, `commands/init.rs` |
| cliclack identity form, colon-space prompt labels, displayed commit-name default and override, Esc/Ctrl+C cancellation, existing/browser login, failed or wrong-GitHub-account authentication, private atomic saving, preserved JSONC values, comments and formatting, duplicates and invalid input, config locations, concurrent edits, setup/switch recovery and explicit repository authorization | `commands/new.rs` |
| Switching, failed authentication changes, global/local commit details and explicit authorization | `commands/switch.rs` |
| Configured identity names with labeled GitHub usernames, commit names, commit emails and filepaths, identities without a login, green active marker, purple two-line Accounts block, case-insensitive mapping, unmapped GitHub accounts, full authentication output, read-only behavior and errors | `commands/status.rs` |
| Identity welcome heading and toggle confirmations, case-insensitive identity names aligned in the second column without a separator, green names and purple labels, NO_COLOR/dumb-terminal handling, GitHub usernames below, effective commit email, mismatch warnings and unavailable configuration | `commands/welcome.rs` |
| Setup output order and indentation, Identity labels, home/XDG/global config filepaths, mode defaults, private identity files, identity rules, repeated setup, conflicts and symlinks | `commands/setup.rs` |
| Generated config comments, updated/unchanged notices, changes to identity files, repeated commands, rule removal, partial failures and custom global config paths | `commands/git_config.rs` |
| Protections on/off, allowed-list replacement, duplicates, invalid selections and missing configuration | `commands/protections.rs` |
| Independent verbose preferences, entry reports and quiet non-repository entry | `commands/verbose.rs` |
| Explicit checks regardless of mode, missing authorization and authentication | `commands/check.rs` |
| Internal commit/push handlers, merge enforcement, native arguments/input and ordinary checkout | `commands/hook.rs` |
| cliclack checklist selection, saved selections, scrolling, minimum selection, Esc/Ctrl+C cancellation and interactive cloning | `commands/picker.rs` |
| Output spacing, change highlighting, errors, warnings, form colors/NO_COLOR, dumb terminals, redirected output and rejection of forms with redirected stderr | `commands/output.rs` |
| Identity names, legacy pins, malformed JSONC and field types, required/unknown/duplicate nested fields, line/block comments and trailing commas, identity collisions, example configuration and path overrides | `identities/` |
| JSONC line/block comments, trailing commas, CRLF and indentation preservation, escaped commit names and protection enforcement | `identities/jsonc.rs` |
| Actual author/committer overrides, allowed emails, authentication and invalid repository policy | `guards/commit.rs` |
| Outgoing author/committer names and emails, collaborators, existing remote history, multiple refs and SHA-256 | `guards/history.rs`, `guards/push.rs` |
| Malformed hook input, encoding, deletion authorization and missing remote baselines | `guards/push_input.rs` |
| Normal, bare, empty and no-checkout clones without authorization | `hooks/cloning.rs` |
| Every installed hook launcher and native forwarding with protections off | `hooks/dispatch.rs` |
| Missing, changed and non-executable hooks, overrides, repair and recursion prevention | `hooks/health.rs` |
| Preserved hooks, rejection, commit-message hooks, input, path resets, Husky and worktrees | `hooks/integration.rs`, `hooks/husky.rs`, `hooks/worktrees.rs` |
| Legacy hook migration and missing-config enforcement | `hooks/legacy.rs` |
| Individual detected-hook paths and skipped inactive hooks | `hooks/reporting.rs` |
| Real interactive startup, directory changes, startup picker and verbose off in all three shells | `shell/` |
| Noninteractive shells and preservation of Bash string/array prompt handlers | `shell/` |

These are functional regression tests, not a claim of complete line or branch
coverage. Assertions check observable outcomes: saved identity policy, commit and
push acceptance, unchanged settings on failure, forwarded input and terminal
output. Failed pushes also verify that blocked references were not created.

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
tests verify unsuccessful exit and unchanged settings; only Escape requires the
prompt's cancellation message.

## Focused runs

```sh
# Run tests for one command.
cargo test --test integration commands::switch

# Run hook safety tests.
cargo test --test integration hooks::

# Run actual shell integration tests.
cargo test --test integration shell::

# Show output from a failing interactive test.
cargo test --test integration commands::picker -- --nocapture
```

Add new tests to the directory that owns the behavior and register new files in
its `mod.rs`. Reuse `Sandbox` for isolated commands and local repositories, and
`terminal` when keyboard interaction is part of the behavior.
