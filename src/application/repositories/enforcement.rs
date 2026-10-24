use super::{AccountQueries, Catalog, Identity, Repository, platform};
use anyhow::{Context, Result, bail, ensure};

impl Repository {
    fn active_identity<'a>(
        &self,
        allowed: &[&'a Identity],
        accounts: &AccountQueries,
    ) -> Result<&'a Identity> {
        ensure!(
            !allowed.is_empty(),
            "No identities are authorized for this repository.\nRun: mgh repo allowed update"
        );
        let active = accounts.active()?;
        allowed
            .iter()
            .copied()
            .find(|identity| identity.username().eq_ignore_ascii_case(&active.login))
            .with_context(|| {
                let permitted = self
                    .stored()
                    .map(|identities| {
                        identities
                            .iter()
                            .map(|identity| identity.name())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();

                format!(
                    "GitHub is using {}, which is not allowed in this repository.\n\
                     Allowed identities: {permitted}\n\
                     Run: mgh switch <allowed-identity>",
                    active.login,
                )
            })
    }

    pub(super) fn enforce_commit(
        &self,
        catalog: &Catalog,
        accounts: &AccountQueries,
    ) -> Result<()> {
        let allowed = self.allowed(catalog)?;
        let active = self.active_identity(&allowed, accounts)?;
        for role in ["AUTHOR", "COMMITTER"] {
            let value =
                platform::text(self.git.run(["var", &format!("GIT_{role}_IDENT")])?.stdout)?;
            let (name, email) = parse_ident(value.trim())?;
            if name != active.commit_name() || !active.accepts(email) {
                bail!(
                    "{role} commit details do not match identity [{}]: {}\n\
                     Run: mgh switch {}\n\
                     When amending, use git commit --amend --reset-author",
                    active.name(),
                    value.trim(),
                    active.name()
                );
            }
        }
        Ok(())
    }

    pub(super) fn enforce_push(
        &self,
        catalog: &Catalog,
        input: &[u8],
        accounts: &AccountQueries,
    ) -> Result<()> {
        let allowed = self.allowed(catalog)?;
        let active = self.active_identity(&allowed, accounts)?;
        let disallowed = catalog
            .all()
            .filter(|identity| !allowed.iter().any(|a| a.name() == identity.name()))
            .collect::<Vec<_>>();
        let text = std::str::from_utf8(input)
            .context("Invalid push-hook input: invalid UTF-8 encoding")?;
        let mut checked = std::collections::BTreeSet::new();
        for line in text.lines() {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            ensure!(
                fields.len() == 4,
                "Invalid push-hook input: expected four fields"
            );
            let local = fields[1];
            let remote = fields[3];
            for id in [local, remote] {
                ensure!(
                    (id.len() == 40 || id.len() == 64) && id.bytes().all(|c| c.is_ascii_hexdigit()),
                    "Invalid commit ID in push-hook input: {id}"
                );
            }
            if local.bytes().all(|c| c == b'0') {
                continue;
            }
            let peel = |id: &str| -> Result<Option<String>> {
                let object = platform::text(
                    self.git
                        .run(["rev-parse", "--verify", &format!("{id}^{{}}")])?
                        .stdout,
                )?
                .trim()
                .to_string();
                let kind = platform::text(self.git.run(["cat-file", "-t", &object])?.stdout)?;
                Ok(if kind.trim() == "commit" {
                    Some(object)
                } else {
                    None
                })
            };
            let Some(local) = peel(local)? else {
                continue;
            };
            let mut args = vec!["rev-list".to_string(), local];
            if !remote.bytes().all(|c| c == b'0')
                && let Some(baseline) = peel(remote).context(
                    "Remote baseline is missing locally. Fetch the remote and try again.",
                )?
            {
                args.push(format!("^{baseline}"));
            }
            let commits = platform::text(self.git.run(&args)?.stdout)?;
            let pending = commits
                .lines()
                .filter(|id| checked.insert((*id).to_string()))
                .collect::<Vec<_>>();
            // Bound argument size while reading many commits in each Git process.
            for batch in pending.chunks(128) {
                let mut args = vec![
                    "show",
                    "-s",
                    "--no-walk=unsorted",
                    "--format=%H%x00%an%x00%ae%x00%cn%x00%ce%x00",
                ];
                args.extend(batch.iter().copied());
                let output = self
                    .git
                    .run(args)
                    .and_then(|out| platform::text(out.stdout));
                let records = output
                    .as_ref()
                    .ok()
                    .map(|text| text.split_terminator("\0\n").collect::<Vec<_>>());
                let records = records.filter(|records| {
                    records.len() == batch.len()
                        && records.iter().zip(batch).all(|(record, id)| {
                            record.split_once('\0').is_some_and(|(actual, fields)| {
                                actual == *id && fields.split('\0').count() == 4
                            })
                        })
                });
                if let Some(records) = records {
                    for (id, record) in batch.iter().zip(records) {
                        let (_, fields) = record.split_once('\0').unwrap();
                        check_commit(id, fields.trim_end_matches('\n'), active, &disallowed)?;
                    }
                } else {
                    // Preserve individual diagnostics for unusual output or a failed batch.
                    for id in batch {
                        let output = self.git.run([
                            "show",
                            "-s",
                            "--format=%an%x00%ae%x00%cn%x00%ce",
                            id,
                        ])?;
                        let details = platform::text(output.stdout)?;
                        check_commit(id, details.trim_end_matches('\n'), active, &disallowed)?;
                    }
                }
            }
        }
        Ok(())
    }
}

fn check_commit(
    id: &str,
    details: &str,
    active: &Identity,
    disallowed: &[&Identity],
) -> Result<()> {
    let mut fields = details.split('\0');
    let parsed = [fields.next(), fields.next(), fields.next(), fields.next()];
    ensure!(
        parsed.iter().all(Option::is_some) && fields.next().is_none(),
        "Invalid commit identity response for {id}"
    );
    if disallowed.is_empty() {
        return Ok(());
    }
    let [author, author_email, committer, committer_email] = parsed.map(Option::unwrap);
    // Normalize each address once, regardless of the number of configured identities.
    let roles = [(author, author_email), (committer, committer_email)].map(|(name, email)| {
        (
            name,
            email.to_lowercase(),
            name.eq_ignore_ascii_case(active.username())
                || name.eq_ignore_ascii_case(active.commit_name()),
        )
    });
    for identity in disallowed {
        for (name, email, active_name) in &roles {
            let other_name = name.eq_ignore_ascii_case(identity.username())
                || name.eq_ignore_ascii_case(identity.commit_name());
            if identity.accepts_normalized(email) || (other_name && !active_name) {
                bail!(
                    "Push blocked: commit {} contains your {} identity in a {} repository.\n\
                     Correct the affected commit before pushing.",
                    &id[..12],
                    identity.name(),
                    active.name()
                );
            }
        }
    }
    Ok(())
}

fn parse_ident(value: &str) -> Result<(&str, &str)> {
    let (name, rest) = value.rsplit_once(" <").context("Invalid Git identity")?;
    let (email, _) = rest.split_once('>').context("Invalid Git identity")?;
    Ok((name, email))
}
