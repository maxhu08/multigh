# Authentication, enforcement, hooks and shells

## GitHub CLI behavior

I-01. Use GitHub CLI's existing github.com account storage. Preserve browser
login for missing/unsuccessful account login and recheck that the requested,
validated username has successful login afterward. Compare usernames
case-insensitively. A successful existing login avoids an unnecessary browser.

I-02. Preserve the distinction between local selected-account reads, live active
authentication and account listings. A missing selected value is optional; an
unexpected query/JSON/process failure retains its cause. A missing GitHub host
in account-list responses behaves as an empty list. Do not initiate login merely
because account-query parsing or execution failed.

I-03. Enforcement uses live active-account verification every operation and never
trusts cached local selection. Commit/push enforcement does not prompt or switch
accounts. Initial-clone checkout can invoke the separate entry workflow, which
retains its normal interactive/autoswitch behavior.

## Commit and explicit repository checks

I-04. Pre-commit and pre-merge-commit require a valid catalog, valid nonempty
repository permissions and an active GitHub account matching an allowed identity.
Invalid/unknown permissions and failed live authentication reject.

I-05. Check actual Git author and committer identities, including command-scoped
config and GIT_AUTHOR/GIT_COMMITTER environment overrides. Names must exactly
match the active identity's configured commit name; emails may match its primary
or additional accepted addresses case-insensitively. Checking config values alone
is insufficient.

I-06. A mismatch identifies AUTHOR or COMMITTER, the identity and relevant
details, with switch guidance and reset-author guidance for amending. Rejection
occurs before existing hooks execute. Explicit `repo check` performs these checks
even when protections are off and is silent on success.

## Push enforcement

I-07. Pre-push validates repository permissions and live active authentication,
then checks outgoing commits. It does not additionally perform the current
author/committer-environment check used by pre-commit and `repo check`.

I-08. Read pre-push input once. Protected input is UTF-8 with four whitespace-
separated fields per update: local ref, local object ID, remote ref, remote object
ID. Each ID is 40 or 64 hexadecimal characters. Reject malformed fields, IDs and
encoding before native hook forwarding. Replay the original bytes afterward;
do not reconstruct or normalize the payload.

I-09. A local all-zero ID is a deletion: still require allowed live authentication,
but do not inspect deleted history. A new remote ref (zero remote ID) checks all
reachable history from the local commit. An existing remote ref excludes its
remote commit and ancestors. If that nonzero remote commit is unavailable locally,
reject with fetch-and-retry guidance.

I-10. Check all new refs in one push; deduplicating already-checked commits must
not omit an unsafe new ref. Support SHA-1 and SHA-256 with full object IDs.
Refs resolving to commits, including annotated tags, use the same reachability
rules and exclude the supplied remote commit and its ancestors.

I-11. Reject an outgoing author or committer attributed to another configured
identity that this repository does not allow. For each disallowed identity:

- A name matches its GitHub username or configured commit name case-insensitively,
  unless it also matches the active identity's username or commit name.
- An email matching its primary/additional accepted addresses case-insensitively
  is disallowed regardless of the name exemption.

Allowed identities are exempt. Collaborators not matching a disallowed configured
identity are accepted. Preserve this rule rather than substituting current-commit
exact-name matching. Reports identify the affected commit, disallowed identity
and active repository identity. A rejected push must not create the rejected refs.

## Hook installation and preservation

I-12. Preserve shared launchers for all 23 names:

```text
applypatch-msg       pre-applypatch       post-applypatch
pre-commit           pre-merge-commit     prepare-commit-msg
commit-msg           post-commit          pre-rebase
pre-push             post-checkout        post-merge
pre-receive          update               proc-receive
post-receive         post-update          reference-transaction
push-to-checkout     pre-auto-gc          post-rewrite
sendemail-validate   fsmonitor-watchman
```

I-13. Launchers invoke the selected executable/configuration via hidden
`internal hook` entry points and preserve native arguments, quoting, empty args
and option-like args. Existing launcher/config paths with spaces/apostrophes work.
The managed format marker is `# Managed by mgh; existing hooks are preserved.`
Launchers are executable; private state directories remain private.

I-14. Reject unrecognized files and symlinks at managed launcher destinations
without overwriting their contents/targets. Refuse unsupported custom global
hook installation paths rather than replacing them. Preserve unrelated Git config
and original project hook files.

I-15. Actual readiness requires every managed launcher's existence, executable
status and correct marker, plus effective hook-path selection. Missing, modified
or nonexecutable launchers and global/command/include overrides produce concrete
problems, not an active-protections claim. Re-evaluate effective settings after a
repair; an overriding include can make successful local writes ineffective.

I-16. Preserve local/worktree hook overrides in their original scopes and repair
them to use the managed launchers. Resolve project-relative hook paths from the
correct repository root, including subdirectory entry and linked worktrees.
Do not rewrite global or command overrides. Recognize canonical aliases back to
the shared directory and reject preserved-hook recursion.

I-17. Existing executable project hooks retain their arguments, working directory,
stdout/stderr, stdin and rejection behavior. Nonexecutable hooks are skipped and
reported as such. Forward ordinary Git hooks and Husky wrappers from their
resolved project paths; reject paths that recurse into managed hooks. Display
meaningful Husky project paths rather than only wrapper paths.

I-18. Enforcement runs before native forwarding; a rejected guard must not invoke
the existing hook. A native rejection still fails after mgh checks pass.
Disabling mgh protections bypasses its checks, not project checks. All installed
hook kinds still forward when disabled, even without identity configuration.

I-19. Preserve binary stdin when enforcement is bypassed. Pre-push input is
buffered and replayed exactly once; other hook input remains inherited. Preserve
successful early-closing hooks without incorrectly failing on a broken input
pipe, while retaining genuine hook failures and other write errors.

I-20. The initial-clone post-checkout signature (three args, all-zero old ID and
checkout flag 1) invokes entry when appropriate. Ordinary checkout does not open
the initial permission picker. Initial/empty/bare/no-checkout clones never inherit
implicit repository authorization.

I-21. During initialization, hooks may run before normal repository discovery
succeeds. Protections still default on and original hooks can be located using
Git's environment. Commit hooks load identity config
but skip repo checks when no repo is discoverable; protected pre-push requires a
repository. Do not make every hook require a repository/configuration.

I-22. Continue to cooperate with Cocogitto commit-message hooks: do not replace
managed shared hooks, do not suppress native message rejection, and keep native
message checks active when identity protections are off.

## Interactive shells and native Git forwarding

I-23. Fish, Bash and Zsh initialization is emitted by `shell init` and loads
command/option completions only interactively. Integrate startup and directory-
change entry without destroying existing Git functions/aliases or prompt handlers.
Repeated initialization does not duplicate existing prompt handlers or reports.

I-24. Bash supports both string and array PROMPT_COMMAND handlers; avoid repeated
entry reports when PWD is unchanged. Fish handles piped initialization with usable
terminal input. Zsh initializes completion when needed without writing a completion
dump, preserves existing completion support and hooks directory changes.

I-25. Noninteractive shells do not run entry checks or create preferences/query
authentication merely because initialization is sourced. Standalone shell output
works without identity config. Actual Tab completion for commands/options remains
functional in all three supported interactive shells.

I-26. Interactive wrapped `git init` runs Git first, then offers permissions in
the initialized repository if protections are on and no identities are allowed.
Support these options and combinations: repeated -C, -c, attached/separate
branch options, --initial-branch abbreviations, --bare, --git-dir, --work-tree,
--separate-git-dir, --object-format, --template, --shared, -- separators and paths
with spaces/apostrophes. Preserve applicable command-scoped configuration.

I-27. Successful reinitialization preserves existing permissions without another
picker. Canceling selection retains Git success and leaves the new repo blocked.
If entry fails after successful initialization, warn with manual recovery guidance
without pretending initialization failed. Native Git failures preserve status and
native output, with no permission prompt.

I-28. Noninteractive forwarding, non-init commands, information flags and aliases
that merely include the word init do not authorize a repository or trigger a picker.
Never persist a temporary forwarding alias. Preserve existing custom Git aliases/
functions. Direct `command git`/executable invocations can bypass the wrapper;
shared hooks still supply protection.
