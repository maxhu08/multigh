# Rewrite requirements

## Scope and precedence

R-01. Implement multigh entirely from the architecture and behavior contracts
in this repository. These specifications are the complete implementation brief,
including command behavior, persistence, integration, presentation and acceptance.
The project currently contains Cargo starter code and this design material.

R-02. These specifications define the application contract. The
[architecture proposal](architecture-proposal.md) defines the intended design
direction. Its suggested internal names, dependencies, file layout and algorithms
may change while satisfying the specified interfaces and user-visible behavior.

R-03. Preserve CLI syntax, command defaults, scopes, interaction, observable
effects and preservation on failure. Preserve existing JSON/JSONC configuration,
documented Git policy, global preferences, hook integration and supported
shell behavior. A refactor must not silently grant access, lose settings, replace
existing checks or change whether an operation succeeds.

R-04. The completed executable is named `mgh`; its package is multigh. The
initial unimplemented starter intentionally uses Cargo's default binary naming
and Hello World program. It is not a working replacement for `mgh`.

## Architecture requirements

R-05. Organize complex behavior into deep owning modules with narrow semantic
interfaces. Callers request complete operations rather than assembling unchecked
Git commands, config writes, authentication and hook forwarding themselves.

R-06. Hide mutable identity collections, raw JSONC editing, unrestricted Git
setters, low-level hook forwarding and temporary-file machinery behind private
implementation boundaries. Make validated identities, configured permissions,
repository discovery, account observation types and operation outcomes explicit.

R-07. Retain one CLI dispatcher. Keep terminal presentation and forms separate
from domain decisions. Reuse workflows for setup, creation, switching, entry and
hook execution without command handlers calling one another's private helpers.

R-08. Distinguish absence, a valid value and failure. A real username such as
`unavailable` must not collide with a presentation placeholder. A locally selected
account must not substitute for live authentication in enforcement.

R-09. Preserve externally observable effect ordering, partial success, native
I/O and error handling. Internal parsing, helper structure, data types, number
of private functions and equivalent implementation techniques are unrestricted.
Choose internal algorithms and data structures independently. Do not add rollback
that changes the partial effects specified for a failed operation.

## Supported environment

R-10. Runtime requirements are Git 2.45+ and GitHub CLI. GitHub authentication is for
`github.com`; tokens remain managed by GitHub CLI. Preserve native Git repository
and configuration semantics, including linked worktrees and bare repositories.

R-11. Fish, Bash and Zsh interactive integration and generated completions remain
supported. Standalone completion formats also include PowerShell and Elvish.
Integration acceptance targets Unix. Windows support requires separate
implementation and verification before it can be claimed.

R-12. Installing or using the replacement must not depend on developer-only
Cocogitto tooling. Existing Cocogitto project hooks must continue to run.

## Specification map

- [CLI, command behavior and presentation](cli.md)
- [Identity configuration, state and repository policy](configuration.md)
- [Guards, hooks, authentication and shell integration](integration.md)
- [Messages, prompts and rendering](messages.md)
- [Acceptance criteria and verification](acceptance.md)

## Unchanged scope limits

R-13. Protections are local Git safeguards. Tools that bypass Git hooks can
bypass enforcement. Do not promise server-side or organization-wide protection.

R-14. Global identity/preferences operations do not expand repository permissions.
Only explicit permission additions or a saved permission checklist can grant
additional identities access.

R-15. Derive implementation and tests from these local requirements. Every
required outcome must be understandable without another checkout, repository,
binary, source file or test collection. Resolve missing details by extending
these specifications explicitly. A passing starter build does not establish
that the application requirements have been implemented.
