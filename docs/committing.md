# Commits

This repository uses [Cocogitto](https://docs.cocogitto.io/) to validate new commit
messages. It is a Rust tool and requires no Node.js dependencies. Contributors
need Cocogitto 7 or newer (`cog`) on PATH; installed `mgh` users do not.

## Setup

From this repository, install Cocogitto and the hook:

```sh
# Install Cocogitto on macOS.
brew install cocogitto

# Alternatively, install it through Cargo.
cargo install cocogitto --locked

# Install the commit-message hook in this repository.
cog install-hook --all
```

Choose one installation method. The hook is defined in `cog.toml`. Cocogitto 7
writes `.git/hooks/commit-msg`; mgh forwards to it while retaining identity checks
and leaving its shared hooks untouched. Cocogitto's installer does not follow
`core.hooksPath`. If another hook directory is in use, integrate the check there.
The installer asks before overwriting existing hooks; combine their checks before
replacing them.
Repeat the hook installation after cloning or changing `cog.toml`.

## Message format

Use `type: description` or `type(scope): description`, for example:

```text
feat: add identity switching
fix(config): preserve comments when adding an identity
docs: clarify setup instructions
```

Supported types are `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`,
`build`, `ci`, `chore` and `revert`. Use a short lowercase description. Scopes
are optional. A breaking change can use `!` after the type or scope, or a
`BREAKING CHANGE:` footer.

Normal `git commit` runs the hook. To check a message without committing:

```sh
cog verify 'fix(config): preserve existing identities'
```

The hook checks the proposed message rather than the entire history. Existing
messages remain unchanged, and Git-generated merge messages are accepted.
`cog check` checks history separately and will report older messages that do not
follow this format.

See the [testing guide](testing.md) for isolated hook tests.
