# AGENTS.md

multigh is a small Rust CLI with one executable, `mgh`. It keeps GitHub
authentication, commit details and repository identity protections aligned.

## Coding Approach

### Think Before Coding

- State assumptions that affect the implementation. Ask when uncertainty changes
  the intended behavior; resolve routine choices using existing patterns.
- Surface meaningful tradeoffs and simpler alternatives before implementing.
- Read the relevant code and tests before changing behavior. Consult `docs/`,
  especially `docs/development.md`, `docs/usage.md`, `docs/testing.md` and the
  relevant files in `docs/help/`, to understand the existing design and behavior.

### Simplicity First

- Write the minimum code that solves the requested problem.
- Avoid speculative features, configuration and abstractions for single-use code.
- Extract helpers when they make reuse, composition or readability clearer.
- Use existing dependencies and the standard library before adding a crate.
- Every line should serve a clear purpose.

### Surgical Changes

- Keep every change tied to the request. Preserve unrelated work.
- Match the existing style; avoid adjacent cleanup or refactoring unless needed.
- Remove imports, variables and functions made unused by your changes.
- Mention unrelated problems instead of expanding the task to fix them.

### Goal-Driven Execution

- Define observable success criteria before implementing.
- For a bug, reproduce the failure in a regression test, then make it pass.
- For a behavior change, verify the intended outcome and relevant failure cases.
- For a refactor, preserve behavior and verify the affected tests before and after.
- Give a brief plan for multi-step work and report what was verified.

## Rust Style

- Keep functions and modules small and direct. Follow the current module boundaries.
- Prefer immutable bindings; use `mut` when the operation needs mutation.
- Prefer borrowing to unnecessary cloning. Let local types be inferred when clear.
- Use early returns for exits and paired `if`/`else` branches for paired decisions.
- Choose loops, iterators and pattern matching according to readability.
- Use concise names that retain context; inline one-use values only when clearer.
- Propagate recoverable errors with `?` and useful context. Reserve `unwrap` and
  `expect` for tests or established invariants.
- Use four-space indentation and blank lines between logical phases. Keep related
  declarations, operations and assertions together.

## Project Conventions

- Keep argument definitions in `src/cli.rs` and command behavior in `src/commands/`.
- Mirror public command groups in command modules. Keep one dispatcher for the
  current CLI; remove replaced commands and implementations instead of retaining
  aliases. Keep shell and hook entry points under the hidden internal group.
- Global identity and preference commands must not expand repository permissions.
  Every repo command requires a repository. Protections default on unless explicitly
  disabled locally; setup must preserve local exceptions and saved preferences.
- Use clap for parsing, help and completions; cliclack for all interactive forms,
  including text inputs and identity checklists.
- Use the shared Git, process and output helpers instead of duplicating them.
- An identity is a configured name such as personal, school or work, mapped to a
  GitHub account and commit details. Identity names are case insensitive. Keep
  identity names distinct from GitHub usernames and commit names in code and output.
- Call generated per-identity Git configs identity files and use Identity for their
  output label. Store identities in JSONC keyed by case-insensitive names, with
  username and nested commit details. Preserve comments and existing formatting
  when editing. Keep existing Git policy keys compatible.
- Keep private identity configuration outside the repository.
  `examples/identities.jsonc` contains fictional example identities.
- Preserve existing hook files, arguments, input, rejection behavior and config
  scope when changing hook integration. Check effective readiness before reporting
  that protections are active.
- Keep implementation explanations in `docs/`; retain comments or markers required
  by a file format or tool.
- Update affected documentation in the same change whenever behavior, commands,
  configuration, setup, architecture or testing guidance changes. Check `docs/`,
  README and embedded command help for anything that would become inaccurate.

## Testing

- Test observable behavior through the actual implementation; do not reproduce
  production logic inside tests.
- Use real Git repositories, local remotes and real shells where practical.
  Simulate GitHub authentication to keep tests offline and independent of identities.
- Isolate home, Git config, identity config and state with `Sandbox` in `tests/support/`.
  Tests must not change real identity settings or user repositories.
- Organize tests by behavior under `tests/identities/`, `tests/commands/`,
  `tests/guards/`, `tests/hooks/` and `tests/shell/`. Reuse shared fixtures and helpers.
- Cover successful operations, rejected operations and preservation of existing
  settings where relevant. Use terminal tests for keyboard-driven behavior.
- See [the testing guide](docs/testing.md) for prerequisites and focused runs.

## Checks

Follow [the commit guide](docs/committing.md) for Conventional Commit messages and
Cocogitto hook setup. Keep commit-message validation compatible with mgh's hooks.

For code changes, run:

```sh
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

Use `cargo fmt` to apply formatting. For documentation-only changes, verify the
content and links; run code checks when executable examples or behavior are affected.
Report failures or unavailable prerequisites instead of treating them as a pass.

See [the development guide](docs/development.md) for the source layout.
