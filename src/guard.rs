use crate::{
    config::{Account, Config},
    git, github, policy,
};
use anyhow::{Result, ensure};
use std::collections::BTreeSet;

pub fn identity(alias: &str, account: &Account) -> Result<()> {
    for kind in ["AUTHOR", "COMMITTER"] {
        let value = git::run(&["var", &format!("GIT_{kind}_IDENT")])?;
        let parsed = value
            .rsplit_once(" <")
            .and_then(|(name, rest)| rest.split_once('>').map(|(email, _)| (name, email)));

        ensure!(
            parsed.is_some_and(|(name, email)| name == account.name
                && account.emails.contains(&email.to_ascii_lowercase())),
            "{kind} identity does not match [{alias}]: {value}\nRun: mgh switch {alias} --repo\nWhen amending an old commit, also use --reset-author."
        );
    }

    Ok(())
}

pub fn check(config: &Config) -> Result<()> {
    if !policy::repository()? {
        return Ok(());
    }

    let allowed = policy::allowed(config)?;
    let alias = policy::active(config, &allowed, &github::active()?)?;

    identity(&alias, config.account(&alias)?)
}

pub fn push(config: &Config, updates: &str) -> Result<()> {
    let allowed = policy::allowed(config)?;
    let alias = policy::active(config, &allowed, &github::active()?)?;
    let account = config.account(&alias)?;

    let mut checked = BTreeSet::new();

    for update in updates.lines() {
        let fields: Vec<_> = update.split_whitespace().collect();

        ensure!(fields.len() == 4, "Invalid push-hook input");

        let (local, remote) = (fields[1], fields[3]);

        ensure!(
            [local, remote]
                .iter()
                .all(|oid| [40, 64].contains(&oid.len())
                    && oid.bytes().all(|c| c.is_ascii_hexdigit())),
            "Invalid commit ID in push-hook input"
        );

        if local.bytes().all(|c| c == b'0') {
            continue;
        }

        let exclude = format!("^{remote}");
        let mut args = vec!["log", "--format=%H%x00%an%x00%ae%x00%cn%x00%ce%x00", local];

        if !remote.bytes().all(|c| c == b'0') {
            ensure!(
                git::optional(&["cat-file", "-e", &format!("{remote}^{{commit}}")])?.is_some(),
                "Remote baseline is missing locally. Fetch the remote and try again."
            );

            args.push(&exclude);
        }

        args.push("--");

        let log = git::run(&args)?;
        let fields: Vec<_> = log.split('\0').collect();

        for commit in fields.chunks(5) {
            if commit.len() == 1 && commit[0].trim().is_empty() {
                continue;
            }

            ensure!(commit.len() == 5, "Cannot parse outgoing commit identities");

            let oid = commit[0].trim();

            if !checked.insert(oid.to_owned()) {
                continue;
            }

            for (other_alias, other) in &config.accounts {
                if allowed.contains(other_alias) {
                    continue;
                }

                let wrong_name = [commit[1], commit[3]].iter().any(|name| {
                    (name.eq_ignore_ascii_case(&other.username)
                        || name.eq_ignore_ascii_case(&other.name))
                        && !name.eq_ignore_ascii_case(&account.name)
                        && !name.eq_ignore_ascii_case(&account.username)
                });
                let wrong_email = [commit[2], commit[4]]
                    .iter()
                    .any(|email| other.emails.contains(&email.to_ascii_lowercase()));

                ensure!(
                    !wrong_name && !wrong_email,
                    "Push blocked: commit {} contains your {} identity in a {} repository.\nCorrect the affected commit before pushing.",
                    &oid[..12],
                    other.username,
                    account.username
                );
            }
        }
    }

    Ok(())
}
