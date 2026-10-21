# Proposed architecture

This document defines the proposed implementation for multigh in this project.
The [diagram](architecture-proposal.svg), [PNG](architecture-proposal.png) and
[D2 source](architecture-proposal.d2) show its boundaries. The
[requirements](requirements.md) and linked behavioral specifications define
command semantics, persistence, integration, messages and completion criteria.
The current program is only a Cargo starter; these modules are not implemented.

The completed executable is `mgh`. Build it from the contracts in `specs/`.
Private names, dependency choices and algorithms can evolve while the approved
interfaces and observable requirements remain satisfied.

## Main decision

Use `commands::run` as the only CLI dispatcher. Make command handlers small
adapters between clap, terminal interaction and named application operations.
Implement reusable workflows inside an `application` module so command handlers
use its facade. Put its implementation behind private modules and export complete,
named operations, opaque handles and read-only presentation data.

Within that boundary, use four substantive deep modules:

| Module | Simple internal interface | Complexity it hides |
| --- | --- | --- |
| `identities` | Load a catalog, resolve a selector, prepare a change, save a prepared change. | JSONC syntax and validation, matching, defaults, CST edits, original-file snapshots, temporary files, permissions, symlinks and concurrent-change checks. |
| `accounts` | Read selected account, query live account/accounts, ensure login, switch to a resolved account. | `gh` arguments, JSON responses, expected absence, browser login, post-login verification, native reports and process errors. |
| `repositories` | Discover/require a repository; inspect it; change permissions/protections; prepare/apply identity selection; refresh integration; execute a hook. | Git invocation context, local/worktree/global scope, explicit repository permissions, identity files and include rules, hook installation/readiness/preservation, commit identities, push parsing/history and Git-init forwarding. |
| `preferences` | Read/write a named global preference; finish first setup. | State paths, marker names, private directory permissions, setup-completion state and first-setup defaults. |

The repository module is deliberately deep. Repository policy, hook settings,
generated identity rules and Git execution all depend on the same scoping and
invocation rules. Keep them in private submodules under one owning boundary,
instead of exposing a general Git executor to command handlers. Independent
identity persistence and GitHub authentication retain their own modules.

Use a small private `platform` module for genuinely shared process/filesystem
mechanics. It does not own policy, authorization or application decisions.

## Proposed layout and visibility

```text
src/
  main.rs
  cli.rs
  commands/                 CLI dispatcher and command-group adapters
  presentation/             cliclack forms, semantic rendering, stream padding
  application/
    mod.rs                  approved facade, opaque types and interaction contracts
    workflows/              private command-independent operations
      identity.rs           create/edit/remove and their integration sequences
      installation.rs       setup/onboarding and first-setup behavior
      selection.rs          manual switching and shared selection sequence
      repository.rs         repository-only actions
      entry.rs              entry, permission prompting and autoswitch
      inspection.rs         status, welcome, doctor; separate read policies
      hook.rs               complete hook lifecycle
      shell.rs              native Git forwarding and initialization entry
    identities/             private persistence, validation and identity model
    accounts/               private GitHub CLI integration
    repositories/
      mod.rs                semantic repository/integration interfaces
      context.rs            invocation directory, roots and config scopes
      permissions.rs        allowed-set reads and explicit changes
      identity_rules.rs     generated files and conditional includes
      hooks/                installation, effective readiness and preserved hooks
      enforcement.rs        commit and outgoing-history checks
      git.rs                private Git backend and global write tracking
    preferences.rs          private preference store
    platform/               private process and filesystem primitives
shell/                      Fish/Bash/Zsh integration contract
tests/                      behavior-based integration coverage
```

These are ownership boundaries, not a requirement to split every operation into
an individual file. Keep small related implementations together. Use one
executable crate; dependency choices remain open. Keep abstractions focused on
the responsibilities and concrete reuse described here.

Declare application implementation modules privately. Re-export the approved
facade from `application/mod.rs`; expose implementation items only to the owning
parent where needed. Keep the Git backend private to `repositories`. Do not
re-export raw Git commands, arbitrary config setters, hook forwarding, JSONC
trees, temporary-file handles or mutable identity maps.

This makes bypassing a workflow a visibility/API change, rather than an easy
call to an already available low-level function. Rust privacy protects this
surface; it does not prevent a maintainer deliberately changing visibility or
adding a new subprocess call.

## Application interface

The following are proposed operation names and contracts, not new CLI commands.
The dispatcher maps each CLI variant directly to its named operation;
there is no second generic command dispatcher.

| Facade operation | Contract visible to callers |
| --- | --- |
| `create_identity`, `edit_identity`, `remove_identity` | Own preparation, validation, authentication where applicable, save and required integration work. Callers never assemble the save/login/refresh sequence. |
| `setup` | Own onboarding when needed, integration installation, first-setup preferences, reports and optional first-identity selection. |
| `switch_identity` | Validate repository policy before changes; switch globally; update local identity only when already permitted; preserve permissions. |
| `require_repository` → `RepositoryActions` | Fail for a missing repository before loading identities or changing settings. Return an opaque handle with only status, check, permission and protection operations. |
| `set_preference`, `show_preferences` | Keep preferences global. Enabling autoswitch invokes entry after saving and reporting the preference. |
| `enter_repository` | Own repair, permission prompting, autoswitch, warning handling and entry-report suppression. |
| `status`, `welcome`, `doctor` | Preserve each command's distinct read, warning and failure policy. Do not replace them with a single eager system snapshot. |
| `handle_hook` | Own validation, input capture, conditional enforcement, checkout-entry behavior and preserved-hook execution. |
| `forward_git` | Run native Git first, preserve failure status, then run eligible initialization entry without converting later picker failure into Git failure. |

The facade takes the current invocation and unresolved config selection, plus
only the interaction/reporting support an operation needs. Resolve the selected
configuration path for operations that need it, require a repository before
repository-only work, and validate policy before switching has side effects.
Construction itself performs no eager authentication, identity loading or repair.
Commands such as help, completions and shell-init remain configuration-independent.
Keep path resolution separate from identity loading. An explicit config path
needs no default home; help, completion and shell-init need neither.

### Interaction without exposing workflow internals

Provide narrow synchronous `Prompts` and `Reports` contracts at the application
boundary. The terminal implementation lives in `presentation` and uses cliclack.
These contracts are justified by the need to ask questions and render progress
at precise points inside a workflow, not by hypothetical alternative backends.

Prompt methods cover the specified forms: identity fields, permission
checklist and active-identity selection. Inputs contain read-only defaults and
choices. Answers and cancellation are distinct outcomes; a returned choice must
still be resolved by its owning module before use. Require terminal stdin,
stdout and stderr for interaction.

The identity workflow opens its original snapshot before requesting input and
keeps it through browser authentication. Moving forms outside that workflow must
not move snapshot acquisition after input collection.

Report methods receive semantic data such as identity changes, repository state,
hook problems and completed global writes. Deliver them synchronously at the
specified operation stages. Presentation owns colors, padding, labels and stream
selection; it performs no configuration or repository reads. Do not buffer all
output until successful completion: forms, inherited child I/O and reports on
failure make that observably different.

## Interfaces that make invalid operations difficult

| Representation | Invariant and owning constructor |
| --- | --- |
| `IdentitySelector` | Retains original user spelling for diagnostics/provenance. Lookup applies the specified case-insensitive matching; it must not introduce stricter CLI rejection rules. |
| `IdentityName` and `ResolvedIdentity` | Created by the validated identity catalog. Private fields, read-only access, no independent mutable username/name/email fields. Resolution belongs to a particular catalog snapshot. |
| `StoredPermission` | Represents a name found in Git even when no identity currently resolves. Enables listing and revoking stale or malformed references without pretending they are configured identities. |
| `PermissionSelection` | Nonempty, catalog-resolved selection for explicit authorization. A separate revoke/clear operation handles the valid empty-policy state. |
| `SelectedAccount` and `LiveAccount` | Distinct observations from distinct `gh` queries. A local selection cannot satisfy an interface requiring a live query. Preserve missing values and process errors separately. |
| `RepositoryActions` / private `RepositoryContext` | Constructed only by discovery. Captures original invocation directory, root and common directory. No public arbitrary Git runner or unchecked config path. |
| Private selection preparation | Validates the full policy and binds the target repository/catalog before any identity-rule or authentication change. Its local disposition is either preserve local state or apply an already permitted identity. |
| Private `PreparedIdentityChange` | Owns validated pending content, original snapshot and private temporary file. Saving consumes it and rechecks the original; dropping it discards the temporary file. |
| `HookInvocation` / private hook execution state | Known hook kind, native arguments and appropriate input mode. Public callers can execute the lifecycle, not call preserved forwarding independently. |
| `HookReadiness` | Distinguishes active integration from concrete installation/override problems. A successful write alone cannot produce an active result. |
| Operation completion/failure data | Retains completed effects and the underlying error. It must not label earlier completed writes as rolled back or turn an error into a default value. |

Use types for the invariants above, not a type for every scalar. Keep booleans for
binary properties, and enums for genuinely different modes/outcomes. Capability
or prepared-state values stay internal unless a caller truly needs them; most
callers receive one complete operation.

Prepared values are local to one operation and consumed promptly. They are not
durable permissions or a claim that Git/GitHub state cannot change externally.
Caching, locking and rechecks must respect the specified concurrent-edit,
live-authentication and partial-completion behavior.

### Permission and read boundaries

Repository permission changes are available only through `RepositoryActions`
operations for adding, revoking or replacing permissions. Identity editing,
switching and preference persistence do not receive that permission-edit surface.
Integration preserves the allowed identity set. Entry gains permission-edit
access only for an explicitly saved checklist selection.

Inspection uses read-only repository/account views. Permission reads never write
settings. Hook enforcement receives account-query capability, with
no login/switch capability; the complete hook workflow can still invoke entry on
the initial-clone checkout path specified in integration.md.

Keep `repo check` separate from protection-enabled hook dispatch: the explicit
check must enforce identity even when repository protections are disabled.

## Preserve operation ordering and partial completion

There is no atomic transaction across JSONC, Git configuration, generated files
and GitHub authentication. The workflow boundary owns sequencing and accurate
results. Retain the partial effects specified below when a later step fails.

| Workflow | Required visible effects and sequencing |
| --- | --- |
| Create identity | Open original snapshot → collect fields → validate complete proposed config → prepare private file → ensure normalized account login → recheck/save → report saved identity → setup/integration → select identity. Later failure retains the saved identity and custom-path recovery guidance. |
| Edit identity | Open/resolve existing identity → collect flags or defaults → prepare/validate → ensure normalized login → save → report → refresh identity rules. Preserve additional emails and do not switch or change repository permissions. |
| Remove identity | Open/resolve → prepare/save removal → report preserved authentication/permissions → refresh rules. Removing the final identity is valid. |
| Switch | Load config → discover repository → resolve target → read/validate repository policy and prior effective details → read prior account/global defaults → refresh identity files/includes → switch `gh` → set global name then email → apply local identity only if allowed → report. Earlier writes may survive later failure. |
| Setup | Obtain existing catalog or complete onboarding → discover repository → reject unsupported global hook path → refresh identity rules → install/integrate hooks → initialize preferences only on first setup → report → select newly created first identity if applicable. |
| Entry | Discover repository → read modes → early exit when all are off → load config/validate policy → check/repair effective hooks → prompt when needed → autoswitch → conditional verbose reports. Picker/autoswitch failures become warnings; discovery, configuration and policy-read errors propagate as failures. |
| Permissions | Add validates the new configured identity. Revoke can remove unresolved stored references; it must not require all remaining references to resolve merely to save a removal. Only explicit authorization requires a nonempty resolved selection. Preserve preference for selected login, otherwise first sorted selection. |
| Protections | Turning on installs hooks before saving true. Turning off saves the local exception without loading identity configuration. Setup preserves explicit off. |
| Hook | Validate hook name → capture pre-push bytes once → discover repo/read protections → perform the specified hook-specific checks/entry action → forward preserved hook. Off bypasses identity loading/enforcement, not forwarding. Other hooks inherit stdin. |
| Git wrapper | Execute Git with native arguments/I/O → preserve failed exit status → require interactive eligible init → carry original options into entry through the temporary command-scoped alias. Do not persist the alias. |

Global-write tracking and presentation remain separate. Track each completed
global write, preserve the originating command spelling in Git comments, skip
unchanged values and report successful writes even if a later stage fails.
Track report state as pending, printed or suppressed; suppress the extra report
after entry selection has supplied its own report. A failed operation with no
global changes must not print a
successful unchanged report.

Workflow error policies differ deliberately: identity listing
warns and continues after account-query failure; welcome warns on bad identity
configuration; manual switch tolerates failure reading the previous active
account for its before/after display. Represent these decisions explicitly in
their owning workflows. Keep the backend's actual errors intact and apply each
documented tolerance only at its owning workflow boundary.

## Implementation and acceptance

All behavior is described by local documents:

| Concern | Contract |
| --- | --- |
| Command syntax, identity workflows, switching, setup, settings and inspection | [CLI and user experience](cli.md) |
| Identity validation/editing, config paths, state and explicit permissions | [Configuration](configuration.md) |
| Live account checks, commit/push rules, hooks and shell forwarding | [Integration](integration.md) |
| Diagnostic strings, prompt labels, help, streams, spacing and colors | [Messages](messages.md) |
| Scenario matrix, isolated environments and completion checks | [Acceptance](acceptance.md) |

Implement coherent behaviors incrementally: encapsulated identity persistence,
account observations and repository consistency first, then complete workflows
and terminal interaction. Keep one operation's validation, writes, reports and
failure results reviewable together. Implementation order can vary with actual
module dependencies.

Each requirement becomes a local executable acceptance case with explicit input,
expected outcome and preservation assertions. Use real Git repositories, local
remotes, supported shells and PTYs for integration behavior; simulate GitHub
responses to keep tests isolated. Add focused interface tests where opaque types
and state transitions promise invariants.

Private helpers and data structures can be designed freely. Public behavior,
state semantics, failure effects and interaction must satisfy these files.
Passing the starter's empty test suite is not application completion.
