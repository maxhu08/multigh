# Acceptance and verification

## Completion target

The starter builds and runs Cargo's Hello World program. Application behavior
is complete only when it satisfies the local [requirements](requirements.md),
[CLI](cli.md), [configuration](configuration.md), [integration](integration.md)
and [message](messages.md) contracts. Build the acceptance tests from those
requirements and the observable scenarios below.

## Observable acceptance matrix

| Area | Required acceptance evidence |
| --- | --- |
| CLI surface | All commands/default subcommands, global config placement, nested help, completions, hidden entry points, rejection of unsupported commands/invalid flags and specified exit statuses. |
| Onboarding | Missing and empty config; interactive first identity; noninteractive guidance; first-setup verbose; repeated setup without resetting preferences or permissions. |
| Identity creation | Defaults, flags, validation before login, existing login, missing/expired login, wrong account, query failures, saved identity recovery after later failure. |
| Identity edit/remove | Defaults/partial flags, no implicit switch/permission edits, stale permission recovery and removal of the last identity. |
| Identity persistence | JSON/JSONC, duplicates/unknowns, CRLF/comments/formatting/additional emails, concurrent edits/creation/deletion, cancellation, private modes and symlink preservation. |
| Name/email semantics | Case-insensitive identity names/usernames/emails, exact commit names, shared name exemptions, accepted additional emails, real usernames equal to display placeholders. |
| Configuration paths | Explicit absolute/relative paths, XDG/home/USERPROFILE fallbacks, missing home errors, no-home help/completions, Git-selected global locations and symlinks. |
| Identity rules | All three remote URL formats, conditional includes, changed/removed identities, foreign-rule conflict, no-op global byte preservation and provenance. |
| Repo policy | Empty/multiple/stale permissions, idempotent add, stale revoke, an explicit allowed set independent of current selection, no implicit authorization and read-only inspection. |
| Scope | Missing repository, malformed Git config, subdirectories, relative GIT_DIR/GIT_WORK_TREE, bare repos, linked worktree common policy and worktree-only hooks. |
| Switching | Global selection outside Git; allowed/disallowed local disposition; invalid policy before mutation; failed switch and earlier completed global changes. |
| Preferences/entry | Independent modes, all-off early exit, only-allowed autoswitch, always-prompt multiple choices, sole already-selected no-op, cancellation and noninteractive guidance. |
| Status/welcome/doctor | Live versus selected reads, read-only byte preservation, missing/bad config, missing/expired auth, full native reports, accurate readiness and accumulated diagnostics. |
| Commit enforcement | Actual author/committer and environment overrides, primary/additional emails, empty/stale policy, failed live auth and no native-hook execution after rejection. |
| Push enforcement | Input/UTF-8/ID rejection, deletions, new refs, remote history boundaries, duplicate commits/multiple refs, SHA-256, disallowed names/emails, collaborator acceptance and no created rejected refs. |
| Hook lifecycle | All launcher names, native arguments/binary stdin once, native rejection, disabled protections still forwarding, native project/Husky/Cocogitto hooks, recursion and initialization before discovery. |
| Hook readiness | Missing/changed/nonexecutable launcher; global/command overrides; same-scope repair; overriding includes; truthful active-state reports. |
| Shells | Real Fish/Bash/Zsh startup/directory entry, interactive/nointeractive contrast, prompt/function/alias preservation and actual Tab completions. |
| Git init | All supported options/targets, paths with quotes/spaces, successful cancellation, reinit no reprompt, native failed status, information flags and no persistent alias. |
| Presentation | stdout/stderr, cancellation/errors, help, silence, semantic colors, form colors, before/after values, shared alignment, exact blank padding and NO_COLOR/TERM behavior. |
| Partial completion | Verify the actual affected configuration, account, generated files and reports for each failure boundary; never infer preservation only from exit status. |

## Requirement-driven verification

For each requirement ID, define a starting state, user action, expected result
and the state that must remain unchanged. Exercise successful, rejected and
partially completed operations through the application's real interfaces.
Use meaningful assertions for success/failure, useful diagnostics, stdout/stderr,
prompt sequences, auth changes, actual config/permissions, hooks, native I/O,
refs and partial effects.
Normalize only irrelevant temporary root paths and terminal transport details.
Do not normalize differences in user-visible names, values, padding or semantics
when those are the behavior under test. Native tools may supply version-dependent
text; separate their unchanged forwarded output from multigh-owned output.

Test user-visible effects and the invariants promised by new module interfaces.
Private structures, process counts, source paths and parsing techniques are
implementation choices. A difference in effects, failure timing relative to
visible changes, or native hook execution is a behavior difference. If a required
decision is unspecified, document its expected behavior in these files before
implementing it; keep the implementation brief self-contained.

## Test environment and completion checks

Use temporary home/config/state/Git paths and fictional identities; simulated
GitHub CLI responses keep tests offline and avoid real authentication changes.
Use real Git, file remotes, supported shells and PTYs where behavior depends on
them. Include quote/space paths and keyboard cancellation. Never mutate real user
identity settings or repositories in tests.

Integration tests need Unix, Git 2.45+, Fish, Bash, Zsh and Cocogitto 7+ for
coexistence checks. Missing prerequisites are failures or explicitly reported
limitations, not a pass. These are development/test prerequisites, not additional
runtime requirements for users.

As behavior is implemented, verify affected contracts and then run:

```sh
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

Completion requires the full acceptance matrix and requirement IDs to be covered,
including preservation and intended diagnostics. Passing an empty starter suite
does not establish completion. Record the actual checks and any remaining
requirement gaps when evaluating the implemented application.
