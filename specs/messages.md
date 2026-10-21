# Messages, prompts and rendering

This contract defines multigh-owned user-facing text and presentation. Values in
angle brackets below are substituted with actual identity names, paths, account
names or error details. Native Git, GitHub CLI and project-hook output retains
its own text and streams.

## Interactive prompts

M-01. Identity fields use these labels followed by a colon and space:
`Identity name`, `GitHub username`, `Commit name`, `Commit email`.
When a default is shown, format the prompt as
`<label> (default: <value>): `. Creation asks identity, username, commit name,
email; edit asks username, email, commit name. Enter accepts a displayed default.

M-02. The permission checklist asks
`Which identities may use this repository?`. The account-selection form asks
`Which allowed identity should be active?`. Choices show configured identity
names and corresponding GitHub usernames. Existing permitted choices are checked;
the selected account is the initial selection in the account form when applicable.
An empty checklist selection displays `Input required` and keeps the form open.
Escape cancels with `Operation cancelled.`; Ctrl+C cancels without saving.

## Diagnostic text and actionable guidance

M-03. Include the following diagnostic phrases and associated context. Compose
path/value lists from actual state; do not substitute presentation placeholders
for valid application values.

| Condition | Diagnostic and guidance |
| --- | --- |
| Unknown configured identity | `Unknown identity '<identity>'; choose <sorted configured names>` |
| Duplicate identity on creation | `Identity '<identity>' already exists` |
| No repository for a repo command | `No Git repository found here. Enter a repository before running mgh repo.` |
| No allowed identities | `No identities are authorized for this repository.` followed by `Run: mgh repo allowed update` |
| Disallowed live account | `GitHub is using <login>, which is not allowed in this repository.` followed by allowed names and `Run: mgh switch <allowed-identity>` |
| Author/committer mismatch | `<AUTHOR or COMMITTER> commit details do not match identity [<identity>]: <actual Git identity>`; include `Run: mgh switch <identity>` and amend guidance using `--reset-author` |
| Invalid push fields | `Invalid push-hook input` |
| Invalid push object IDs | `Invalid commit ID in push-hook input` |
| Unavailable remote commit | `Remote baseline is missing locally. Fetch the remote and try again.` |
| Disallowed outgoing identity | `Push blocked: commit <12-character ID> contains your <disallowed identity> identity in a <active identity> repository.` and `Correct the affected commit before pushing.` |
| Unknown hook name | `Unknown Git hook: <name>` |
| Project hook failure | `Existing <name> hook rejected this operation` |
| Missing active account | `No active GitHub account` |
| Failed full authentication report | `GitHub authentication check failed` |
| Incorrect post-login account | `GitHub account '<username>' is not signed in`; direct the user to authorize that account and retry |
| Concurrent config save | `Configuration changed while signing in` or `Configuration changed during editing`, as applicable; preserve concurrent content |
| Invalid config | Name the selected path and `invalid identity configuration`; retain parsing/validation details |
| Invalid identity field | Identify `Invalid identity name`, `Invalid username`, `Invalid commit name` or `Invalid email`, with the affected name/value |
| Duplicate account/email ownership | `Identities cannot share usernames or email addresses` |
| Multiline command input | `Identity fields must be single-line values` |
| Missing home fallback | `Cannot locate your home directory` |

M-04. Preserve causes for external command, filesystem and malformed data errors.
Identify the failed program or path and its diagnostic. Do not relabel unexpected
errors as no repository, missing account or empty configuration. Invalid UTF-8
push input is a rejection with encoding context.

M-05. Noninteractive onboarding explains that setup needs a first identity,
shows the selected config path and supplies
`mgh identity new --username <username> --email <email> <identity>`.
Missing noninteractive fields identify the required field and available flags.
A permission picker without usable terminal streams directs users to
`mgh repo allowed update` interactively or `mgh repo allowed add <identity>`
for automation. Multiple-choice autoswitch without a terminal advises
`mgh switch <allowed-identity>`.

M-06. Config-edit symlink errors identify the path and advise using `--config`
with its target. Generated identity-file symlinks, unrecognized managed hook
destinations and custom global hook paths identify the protected destination
or setting and explain that it was left intact. Hook-health diagnostics distinguish missing, nonexecutable,
changed/unreadable launchers and effective-path overrides with their scopes/paths.

M-07. If identity saving succeeds before integration/selection fails, report
`Identity saved in <path>` and usable setup/switch recovery commands. Include the
actual custom `--config` option with shell-safe quoting. If Git initialization
succeeds but entry fails, state that initialization succeeded and advise
`mgh repo allowed update` in the new repo.

M-08. An unconfigured protected repo with working integration reports
`No identities selected; commits and pushes remain blocked.`. If integration is
also unavailable, require both repairing hooks and choosing identities before
committing/pushing. Explain that protections are not active and direct users to
setup; unsupported global/command overrides need manual integration.
Canceling entry selection retains the relevant blocked-state guidance.

## Report structure

M-09. Identity reports distinguish configured identity names from GitHub
usernames and commit names. Use labels such as `GitHub username`, `Commit name`,
`Commit email`, `Identity file` and `Accounts`. Mark the active identity
`(Active)` and an unsigned configured account `(Not signed in)`.
Unconfigured account entries are listed separately.

M-10. Repo reports include `Current repository`, `Repo config`, `Active identity`,
`Allowed identities`, `Commit name`, `Commit email`, `Protection` and
`Autoswitch (global)`. Empty permissions display `none selected`; unresolved
permission entries advise removal or updating permissions. A successful identity
match reports `Identity and commit details match`. A healthy enabled integration
reports `Commit + push checks enabled`; off and unavailable/overridden states are
distinct. Preferences display `ON` or `OFF`.

M-11. Global changes report `Global Git config updated by <command>` once;
a successful no-op reports `Global Git config unchanged`. A failing no-op does
not emit a successful unchanged report. A changed before/after value uses `→`;
an unchanged value is shown once. Include completed writes even when a later
operation fails. Entry selection suppresses redundant change/permission reports.

M-12. Welcome displays identity, GitHub username and effective commit email.
Missing state is presented as `not configured` or `unavailable` where appropriate.
Commit mismatch guidance uses `Commit details need: mgh switch <identity>`;
an account outside a nonempty permitted set uses
`This repository needs: mgh switch <allowed identity>` with the available choices.

M-13. Setup reports real config/identity-file paths, global Git config,
protection/preference state and Fish/Bash/Zsh initialization examples. Include
`Add more identities: mgh identity new` and
`Choose permissions in a repo: mgh repo allowed update`.
Doctor reports problems under `multigh diagnostics`, fails with repair guidance
when unhealthy, and reports `Installation checks passed` when healthy.

## Streams, spacing and colors

M-14. Human reports and warnings use stdout; application/parser errors use
stderr. Requested help/version uses stdout. Native/full-authentication output
keeps the documented stdout/stderr distinction. Shell/completion output and
inherited child I/O are not reformatted as human reports.

M-15. Human content has one leading and one trailing empty line. Adjacent human
sections/forms/warnings/errors share padding without repeated blank lines.
Report rows are indented two spaces; nested identity fields four. Report values
start at column 31 (one-based). Long labels wrap the value to that column on the
next line. Silent commands emit no padding.

| Meaning | Color | Weight |
| --- | --- | --- |
| Heading | `#F472B6` | Bold |
| Identity name | `#67E8F9` | Bold |
| GitHub username | `#C084FC` | Normal |
| Commit name/email | `#FB923C` | Normal |
| Path/muted text | `#94A3B8` | Normal |
| Success | `#86EFAC` | Bold |
| Warning | `#FBBF24` | Bold |
| Error | `#FB7185` | Bold |

M-16. Apply value colors consistently in forms, status, welcome and changes.
Disable styling for a redirected stream, `NO_COLOR` or `TERM=dumb`. Full native
GitHub reports also honor `CLICOLOR=0`. Keep authentication text from the child
intact within the surrounding report spacing.

## Help

M-17. Root help lists the public commands in [the CLI contract](cli.md), describes
global identity/preferences versus repository permissions and includes first
setup, identity creation and permission-selection examples. Describe `--config`,
default configuration/state paths, protections-on and autoswitch-off defaults.
Each command's help describes its flags, defaults, scope, interaction needs,
success/failure behavior and recovery commands from these specifications.
Hidden integration entry points do not appear in the public command list.
