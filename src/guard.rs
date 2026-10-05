use crate::{
    config::{Config, Identity},
    github, policy,
    repository::Repository,
};
use anyhow::{Result, bail, ensure};
use std::collections::BTreeSet;

pub fn commit_details(
    repository: &Repository,
    identity_name: &str,
    identity: &Identity,
) -> Result<()> {
    for kind in ["AUTHOR", "COMMITTER"] {
        let value = repository.run(&["var", &format!("GIT_{kind}_IDENT")])?;
        let parsed = value
            .rsplit_once(" <")
            .and_then(|(name, rest)| rest.split_once('>').map(|(email, _)| (name, email)));

        ensure!(
            parsed.is_some_and(|(name, email)| identity.matches_commit(name, email)),
            "{kind} commit details do not match identity [{identity_name}]: {value}\nRun: mgh switch {identity_name}\nWhen amending an old commit, also use --reset-author."
        );
    }

    Ok(())
}

pub fn check(repository: &Repository, config: &Config) -> Result<()> {
    let allowed = policy::allowed(repository, config)?;
    let (identity_name, identity) = policy::active(config, &allowed, &github::active()?)?;

    commit_details(repository, identity_name, identity)
}

pub fn push(repository: &Repository, config: &Config, updates: &str) -> Result<()> {
    let allowed = policy::allowed(repository, config)?;
    let (identity_name, identity) = policy::active(config, &allowed, &github::active()?)?;

    let mut checked = BTreeSet::new();

    for update in updates.lines() {
        let update = PushUpdate::parse(update)?;

        if update.local_oid.bytes().all(|c| c == b'0') {
            continue;
        }

        let exclude = format!("^{}", update.remote_oid);
        let mut args = vec![
            "log",
            "--format=%H%x00%an%x00%ae%x00%cn%x00%ce%x00",
            update.local_oid,
        ];

        if !update.remote_oid.bytes().all(|c| c == b'0') {
            ensure!(
                repository.commit_exists(update.remote_oid)?,
                "Remote baseline is missing locally. Fetch the remote and try again."
            );

            args.push(&exclude);
        }

        args.push("--");

        let log = repository.run(&args)?;

        for commit in OutgoingCommit::parse_log(&log)? {
            if !checked.insert(commit.oid.to_owned()) {
                continue;
            }

            for (other_identity_name, other) in &config.identities {
                if allowed.contains(other_identity_name) {
                    continue;
                }

                let wrong_name = [commit.author_name, commit.committer_name]
                    .iter()
                    .any(|name| {
                        (other.matches_username(name)
                            || name.eq_ignore_ascii_case(&other.commit_name))
                            && !name.eq_ignore_ascii_case(&identity.commit_name)
                            && !identity.matches_username(name)
                    });
                let wrong_email = [commit.author_email, commit.committer_email]
                    .iter()
                    .any(|email| other.accepts_email(email));

                ensure!(
                    !wrong_name && !wrong_email,
                    "Push blocked: commit {} contains your {} identity in a {} repository.\nCorrect the affected commit before pushing.",
                    &commit.oid[..12],
                    other_identity_name,
                    identity_name
                );
            }
        }
    }

    Ok(())
}

struct PushUpdate<'a> {
    local_oid: &'a str,
    remote_oid: &'a str,
}

impl<'a> PushUpdate<'a> {
    fn parse(line: &'a str) -> Result<Self> {
        let fields: Vec<_> = line.split_whitespace().collect();
        let [_, local_oid, _, remote_oid] = fields.as_slice() else {
            bail!("Invalid push-hook input");
        };

        ensure!(
            [local_oid, remote_oid]
                .iter()
                .all(|oid| [40, 64].contains(&oid.len())
                    && oid.bytes().all(|c| c.is_ascii_hexdigit())),
            "Invalid commit ID in push-hook input"
        );

        Ok(Self {
            local_oid,
            remote_oid,
        })
    }
}

struct OutgoingCommit<'a> {
    oid: &'a str,
    author_name: &'a str,
    author_email: &'a str,
    committer_name: &'a str,
    committer_email: &'a str,
}

impl<'a> OutgoingCommit<'a> {
    fn parse_log(log: &'a str) -> Result<Vec<Self>> {
        let fields: Vec<_> = log.split_terminator('\0').collect();
        let mut commits = Vec::new();

        for fields in fields.chunks(5) {
            let [
                oid,
                author_name,
                author_email,
                committer_name,
                committer_email,
            ] = fields
            else {
                bail!("Cannot parse outgoing commit details");
            };

            commits.push(Self {
                oid: oid.trim(),
                author_name,
                author_email,
                committer_name,
                committer_email,
            });
        }

        Ok(commits)
    }
}
