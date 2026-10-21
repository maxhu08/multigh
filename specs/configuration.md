# Configuration, persistence and permissions

## Identity configuration

F-01. Default identity definitions are
`$XDG_CONFIG_HOME/multigh/identities.jsonc`, falling back to
`$HOME/.config/multigh/identities.jsonc`. An empty XDG variable uses the fallback.
If HOME is absent, support USERPROFILE as the fallback home. Explicit absolute
or invocation-relative `--config` overrides the default without needing a default
home/config directory. Report missing prerequisites usefully.

F-02. Accept JSON and JSONC with comments and trailing commas. The root is an
object keyed by identity names. Reject malformed syntax, nonobject roots,
comment-only input, unknown fields, duplicate fields and case-colliding identity
keys. An empty object is valid.

F-03. Each definition has required `username` and required `commit` object.
Commit has required `email`, optional `name` and optional array
`additional_emails`. Preserve this externally editable schema:

```jsonc
{
  "personal": {
    "username": "alice",
    "commit": {
      "name": "Alice Example",
      "email": "alice@example.com",
      "additional_emails": ["123+alice@users.noreply.github.com"]
    }
  }
}
```

F-04. Names are case-insensitive and interpreted in lowercase. Valid configured
names start with an ASCII letter and otherwise contain ASCII letters/digits,
underscore or hyphen. Names identify configured identities, not GitHub usernames
or commit names. Preserve canonical sorting and deduplication in listings/selections.

F-05. Trim username and commit fields. Username starts with an ASCII alphanumeric
character and otherwise contains ASCII alphanumerics or hyphens. An omitted
commit name defaults to the normalized username. Commit names must be nonempty
and contain no control characters or angle brackets.

F-06. Emails must contain exactly one @ with nonempty sides and no whitespace
or angle brackets. Trim primary/additional addresses; email matching is
case-insensitive. Primary email retains its configured display spelling while
the accepted set includes its normalized value plus additional addresses.
Do not add GitHub/RFC restrictions beyond the validation rules specified here.
Interactive/flag identity writes also reject multiline/control input.

F-07. Identities cannot share usernames or accepted email addresses, using
case-insensitive comparison. Duplicate accepted emails within one identity are
deduplicated. Commit names may be shared; do not infer identity uniqueness from them.

F-08. Editing preserves other identities, additional emails, JSONC comments,
indentation, formatting, escaped Unicode/text and CRLF line endings. Validate the
whole proposed configuration before authentication and persistence.

F-09. Do not replace the original during canceled/invalid input, login failure,
wrong-account authentication or concurrent modification/creation/deletion.
Capture the original before prompts/login; compare before replacing. Reject
symlink targets for command edits and preserve the target. Read-only loading is
not subject to the editing-only symlink rejection. Clean abandoned pending files.
New private files are 0600 and newly created private directories 0700 on Unix.

## Global state and Git identity rules

F-10. State lives at `$XDG_STATE_HOME/multigh`, falling back to
`$HOME/.local/state/multigh` with the same home fallback. Preserve existing
`autoswitch-enabled`, `verbose-enabled`, `welcome-enabled` and
`setup-complete-enabled` markers and their meanings.
All three user preferences are initially off; first completed setup enables
verbose once. Setup never resets later preference choices.

F-11. Write generated identity files at
`<state>/identities/git-<lowercase-identity>.conf` and shared launchers at
`<state>/hooks/<hook-name>`. Report real selected paths, not assumed defaults.
Default private storage stays outside the project; explicit user-selected
configuration paths remain supported. GitHub tokens stay with GitHub CLI.

F-12. Conditional identity rules apply name/email by GitHub remote owner for:
`https://github.com/<username>/...`, `git@github.com:<username>/...` and
`ssh://git@github.com/<username>/...`. Preserve Git's
`includeif.hasconfig:remote.*.url:<prefix><username>/**.path` integration.
An unconfigured remote keeps ordinary global defaults. Explicit permitted local
identity settings take their normal precedence over conditional/global defaults.

F-13. Refresh changed identity contents and remove obsolete owned include rules.
Repeated equivalent setup does not rewrite unchanged global entries. Preserve
unrelated global config and foreign include rules. A desired rule already
pointing outside the managed identity directory is a conflict; reject without
changing the conflicting rules. Refuse generated identity-file symlinks and
leave their targets intact. Do not sweep unrelated files.

F-14. Git selects global config locations, including HOME/XDG, environment
overrides and symlinks. Preserve those locations and symlinks. Changed generated
global settings and generated identity-file entries carry
`# Generated by multigh (<originating command>)` provenance using the original
command spelling. Removal leaves no stray provenance-only comment.

## Repository scope and stored policy

F-15. Preserve canonical local keys `mgh.allowed-identity` (repeated values),
`mgh.current-identity` and `mgh.protections`. Permissions/protections are shared
through the common repo config in linked worktrees. Report that config's real path.
Honor the original invocation directory, including relative GIT_DIR/GIT_WORK_TREE.
Bare repositories and normal subdirectories remain supported.

F-16. Repository permissions come only from repeated local
`mgh.allowed-identity` values. Missing entries or an explicit empty value mean
no allowed identities. Normalize names case-insensitively and remove duplicates.
`mgh.current-identity` records the selection for reporting; it never grants access.

F-17. Permission reads are read-only. Only explicit add, revoke or saved-checklist
operations write the allowed set. Setup, entry, switching and inspection preserve
that set unless the user saves a permission selection.

F-18. Configured resolution for checks/switching validates every allowed name,
not only the matching active one. Stale names block protected checks and manual
switching. Listing and revoking stored names must still work with stale references.
Removing all entries writes an explicit empty list. A current identity selection
must not restore access.

F-19. Successful authorization chooses local commit details from the allowed
identity matching the local selected GitHub login, otherwise the first sorted
allowed identity. Save local name, email and canonical current identity.
Do not sign in or switch accounts for this permission operation.

F-20. Protections default on unless explicitly disabled locally. A global value
does not replace the repo-local default. Invalid stored booleans fail closed.
Autoswitch is a global preference.

F-21. Preserve hook override scope using effective `core.hooksPath` and saved
`mgh.originalHooksPath`. Repository-local/worktree overrides are saved and repaired
in their original scope; do not turn a worktree exception into a common local one.
Global/command overrides remain untouched and are reported when they prevent
effective readiness.

F-22. Missing repository detection must distinguish confirmed no-repository from
unexpected Git/config errors. Missing optional config values are different from
failed reads. Present valid values must never be used as absence sentinels.

## Failure effects

F-23. Invalid repository permissions reject switching before identity
files, authentication or config changes. Invalid identity input rejects before
login or Git mutation. No-op writes preserve existing global bytes/provenance.

F-24. Failure after successful work can leave partial effects: a saved identity
survives failed integration/selection; refreshed identity rules can survive a
failed account switch; completed global writes remain attributed/reported.
Preserve user data and accurate recovery guidance rather than claiming rollback.
