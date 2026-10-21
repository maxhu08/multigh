# CLI and user experience

## Command surface

C-01. Preserve `mgh [--config <path>] <command>`. The config option is global
and works before or after subcommands. Standard help/version behavior, nested
help, invalid-argument diagnostics and exit behavior follow these contracts.
Help, shell initialization and completions work without identity configuration,
GitHub authentication or a default home/config directory.

| Command | Required behavior |
| --- | --- |
| `setup` | Guide onboarding when no identities exist; otherwise refresh integration. |
| `status [--full]` | Report global identities/authentication and optional current-repo state. |
| `switch <identity>` | Select global authentication/defaults and conditionally local details. |
| `doctor` | Diagnose installation, configuration, tools, authentication and hooks without repairs. |
| `identity [list]` | List global identities; omitted subcommand means list. |
| `identity new [identity] [--username <username>] [--email <email>] [--name <name>]` | Add, authenticate, install integration and select the identity. |
| `identity edit <identity> [--username <username>] [--email <email>] [--name <name>]` | Edit selected fields or prompt with defaults. |
| `identity remove <identity>` | Remove a configured identity without signing out or editing permissions. |
| `repo [status]` | Report this repository; omitted subcommand means status. |
| `repo check` | Explicit live-account and actual commit-identity check, silent on success. |
| `repo protections [on\|off]` | Show or change this repository's protection preference. |
| `repo allowed [list]` | List stored permissions, including unresolved references. |
| `repo allowed add <identity>` | Add a configured identity without duplicates. |
| `repo allowed remove <identity>` | Revoke an identity, including stale references. |
| `repo allowed update` | Replace permissions using a checklist; accepts no identity arguments. |
| `settings` | Show autoswitch, verbose and welcome preferences. |
| `settings autoswitch [on\|off]` | Show/change global autoswitch; enabling also enters the current repo. |
| `settings verbose [on\|off]` | Show/change global entry reports. |
| `settings welcome [on\|off]` | Show/change the optional global greeting. |
| `shell init <fish\|bash\|zsh>` | Emit shell integration to stdout. |
| `shell completions <shell>` | Emit command/option completions. |

C-02. Provide hidden integration entry points:
`internal enter`, `internal welcome`, `internal hook <name> [args...]` and
`internal git [args...]`. Preserve native argument boundaries, empty arguments
and option-like arguments in forwarded input. Internal commands are hidden from
the public command list.

C-03. Do not provide unsupported top-level aliases such as `new`, `protect`,
`protections`, `verbose`, `welcome`, `init`, `check`, `completions`,
`enter` or `hook`. Reject unsupported states, arguments, missing required
identities, `--repo` on identity/switch commands, `repo autoswitch` and extra
arguments to the permission checklist.

## Setup and identities

C-04. Setup with missing or empty identity configuration prompts for a first
identity only in an interactive terminal. Noninteractive setup fails with useful
`identity new` guidance and the selected configuration path. Empty `{}` is
valid configuration, not malformed input.

C-05. A populated setup refreshes identity rules and shared hooks without
rewriting identity definitions, resetting saved preferences or changing explicit
local protection exceptions. First completed setup enables verbose. Subsequent
setup preserves user choices. Report configuration/identity-file paths, shell
initialization instructions and next steps.

C-06. Identity creation prompts for missing identity name, GitHub username,
commit name and email, in that order. Name defaults to username. Noninteractive
creation needs identity name, username and email; an omitted commit name uses
the default. Normalize/validate before authentication. Reuse a successful existing
login; otherwise start browser login and verify the requested account.

C-07. Cancellation, invalid input, failed/wrong-account login, unexpected account
query failure or concurrent config edits do not save the proposed identity or
change Git defaults. If setup or selection fails after saving, retain the identity
and report recovery instructions including a custom `--config` path.

C-08. Editing without field flags prompts using existing values, in username,
email, commit-name order. With flags, change only supplied fields. Preserve
additional emails, other identities, comments and formatting. Verify the trimmed,
validated username's login before saving. Do not switch the account, apply local
commit details or expand permissions. Refresh generated identity rules afterward.

C-09. Removing an unknown identity fails. Removing the final configured identity
is valid. Preserve GitHub login and all repository permission entries; warn that
stale permissions remain blocked until explicitly updated. Refresh generated
include rules after saving removal.

## Switching and repository operations

C-10. Manual switching is case-insensitive and works inside or outside Git.
Validate a repository's complete stored policy before authentication or settings
changes. Unknown permitted identities reject the switch before generated
identity files, account selection or commit defaults change.

C-11. A successful switch changes the active GitHub account and global commit
name/email. Apply local name/email/current identity only if this repository
already allows the requested identity. A disallowed target can still be selected
globally; it must not acquire permission or overwrite this repo's local identity.
Reports show meaningful before/after values without spurious arrows for equal values.

C-12. Every `repo` command requires a repository before identity loading or
settings mutation. Treat malformed Git config as an error rather than absence.
Worktrees, subdirectories, bare repositories and relative Git environment paths
must target the intended repository.

C-13. Adding permission is case-insensitive/idempotent. Permission changes do not
switch GitHub authentication. A valid nonempty saved selection also selects local
commit details: prefer an allowed identity matching the local selected account,
otherwise the first sorted allowed name. Revoke can remove stale names while
other stale names remain. Removing the last permission leaves an explicit empty
policy and blocks protected operations.

C-14. Turning protections off is local, works without identity configuration
and preserves project hooks and permission entries. Turning on installs/repairs
integration before saving the enabled preference; a failed enable must not erase
an existing off exception. Explicit `repo check` runs even when protections are off.

## Settings, entry and inspection

C-15. Preferences are global, independent and persist across setup and restart.
Changing verbose/welcome does not require valid identity configuration or mutate
repository settings/authentication. Autoswitch preferences also work outside Git
without identity configuration. Enabling autoswitch inside Git runs entry; its
saved on state can remain if that later entry fails.

C-16. Entry outside Git is silent. In a repo, all modes off permits a silent
early exit without loading identities. When protections are enabled, check/repair
effective integration before selection. No permissions triggers a checklist in
an interactive terminal, or guidance otherwise, including when integration is
unavailable. Canceling leaves protected operations blocked.

C-17. Autoswitch considers only allowed identities. No allowed identities means
no switch. With one, select it unless its account is already locally selected;
that already-selected case does not switch or rewrite commit settings. With
several, always prompt, preselecting the current one when applicable. Noninteractive
multiple-choice entry prints guidance. Picker/autoswitch errors warn without
breaking shell startup; earlier discovery/config/policy failures retain failure.

C-18. Verbose off suppresses informational permissions/detected-hooks reports,
not protection warnings, permission prompts or autoswitch. Successful selection
avoids redundant entry/global-change reports. Existing inactive hooks are shown
individually and marked as skipped.

C-19. Status lists configured identity names, GitHub usernames, commit details,
generated identity-file paths and active/not-signed-in states; distinguish
unconfigured GitHub accounts. Repo status includes repo config path, permissions,
effective commit details, actual hook readiness and global autoswitch state.
Mismatches are useful warnings. Do not report protections active merely because
the preference is on.

C-20. Status uses live account information. Missing/invalid identity config can
be reported while available account information remains visible. Account-query
failure or no active account fails; an active account with a reported expired
state remains visible with its warning. `--full` also displays native stdout
and stderr authentication details and fails if that full check fails.
Identity listing warns and continues if the account query is unavailable.

C-21. Welcome is opt-in and silent when off. It uses local selected-account
information without a live-auth query, displays identity/username/effective
email, warns about bad configuration or permission/commit mismatches and remains
read-only. An unexpected selected-account process error still fails. A legitimate
login literally named `unavailable` remains a valid login.

C-22. Doctor checks Git/gh availability, Git version, nonempty valid identity
configuration, live authentication and effective/shared hook readiness. Report
independent tool, version, identity-config, authentication and hook-health problems
together, then fail if unhealthy. A failed repository/configuration read retains
its cause. Never repair configuration or hooks from this command.

## Interaction, output and failures

C-23. Forms need terminal stdin, stdout and stderr. Preserve defaults and saved
checklist selections, arrow navigation, Space selection, Enter confirmation and
Escape/Ctrl+C cancellation. The permission checklist requires at least one item
and scrolls beyond seven visible choices. Cancellation leaves affected state intact.

C-24. Preserve semantic colors: headings pink, identities bold cyan, GitHub
usernames purple, names/emails orange, paths slate, success green, warnings amber
and errors rose. Values retain their colors in forms and before/after reports.
Disable color for redirected streams, `NO_COLOR` or `TERM=dumb`.

C-25. Human reports, help, warnings, forms and errors have exactly one blank line
before/after content and no repeated empty padding between adjacent sections.
Preserve shared top-level/nested value alignment. Successful check/quiet entry
remains silent. Shell scripts, completions and inherited native child output keep
their own byte stream formatting.

C-26. Report actual global writes once, with command provenance. A successful
no-op reports unchanged; a failed operation with no global writes does not claim
successful unchanged state. Completed writes remain reported if a later stage
fails. Human warnings go to stdout and application errors go to stderr.
Native full-authentication output preserves its stdout/stderr separation and
color preferences, including `CLICOLOR=0`.

C-27. Preserve useful diagnostic distinctions and recovery guidance for unknown
identity, empty authorization, disallowed account, author/committer mismatch,
bad config, hook overrides and authentication/process failures.
Use the local [message contract](messages.md) for diagnostic text, prompts,
reporting and help. Successful commands return zero, application failures return
one, argument parsing failures return two, and requested help/version returns
zero. Native Git-wrapper failures keep their native exit status; a child failure
without a numeric exit status returns one.
