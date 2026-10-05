use crate::{
    config::{Config, Identity},
    github,
    repository::Repository,
};
use anyhow::{Result, bail, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub const ALLOWED: &str = "mgh.allowed-identity";
pub const CURRENT: &str = "mgh.current-identity";

pub fn stored(repository: &Repository) -> Result<BTreeSet<String>> {
    let values = repository.entries(
        "^(mgh\\.(allowed-identity|current-identity|allowedaccount|account)|ghguard\\.account)$",
    )?;
    decode(&values)
}

pub fn decode(values: &BTreeMap<String, Vec<String>>) -> Result<BTreeSet<String>> {
    let explicit = values
        .get(ALLOWED)
        .or_else(|| values.get("mgh.allowedaccount"));
    let mut allowed = BTreeSet::new();

    if let Some(values) = explicit {
        for name in values.iter().filter(|value| !value.is_empty()) {
            allowed.insert(name.to_ascii_lowercase());
        }
    } else {
        let pinned = values
            .get(CURRENT)
            .or_else(|| values.get("mgh.account"))
            .and_then(|values| values.last())
            .map(|name| name.to_ascii_lowercase());
        let legacy = values
            .get("ghguard.account")
            .and_then(|values| values.last())
            .map(|name| name.to_ascii_lowercase());
        ensure!(
            pinned.is_none() || legacy.is_none() || pinned == legacy,
            "Repository identity settings conflict"
        );
        if let Some(name) = pinned.or(legacy) {
            allowed.insert(name);
        }
    }
    Ok(allowed)
}

pub fn migrate(repository: &Repository) -> Result<()> {
    let legacy = repository.entries("^mgh\\.(allowedaccount|account)$")?;

    for (old, new) in [("mgh.allowedaccount", ALLOWED), ("mgh.account", CURRENT)] {
        if let Some(values) = legacy.get(old) {
            if repository.local(new)?.is_none() {
                for value in values {
                    repository.run(&["config", "--local", "--add", new, value])?;
                }
            }

            repository.run(&["config", "--local", "--unset-all", old])?;
        }
    }

    Ok(())
}

pub fn allowed(repository: &Repository, config: &Config) -> Result<BTreeSet<String>> {
    let allowed = stored(repository)?;

    for name in &allowed {
        config.identity(name)?;
    }

    Ok(allowed)
}

pub fn active<'a>(
    config: &'a Config,
    allowed: &'a BTreeSet<String>,
    login: &str,
) -> Result<(&'a str, &'a Identity)> {
    ensure!(
        !allowed.is_empty(),
        "No identities are authorized for this repository.\nRun: mgh repo allowed update"
    );

    let identity_name = allowed
        .iter()
        .find(|identity_name| config.identities[*identity_name].matches_username(login));

    let Some(identity_name) = identity_name else {
        bail!(
            "GitHub is using {login}, which is not allowed in this repository.\nAllowed identities: {}\nRun: mgh switch <allowed-identity>",
            allowed
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        );
    };

    Ok((identity_name, &config.identities[identity_name]))
}

pub fn apply_identity(
    repository: &Repository,
    identity_name: &str,
    identity: &Identity,
) -> Result<()> {
    migrate(repository)?;
    repository.set("--local", "user.name", &identity.commit_name)?;
    repository.set("--local", "user.email", &identity.commit_email)?;
    repository.set("--local", CURRENT, &identity_name.to_ascii_lowercase())?;

    Ok(())
}

pub fn authorize(
    repository: &Repository,
    config: &Config,
    identity_names: &BTreeSet<String>,
) -> Result<()> {
    for identity_name in identity_names {
        config.identity(identity_name)?;
    }

    let selected = github::selected()?.unwrap_or_default();
    let preferred = identity_names
        .iter()
        .find(|identity_name| config.identities[*identity_name].matches_username(&selected))
        .unwrap_or_else(|| {
            identity_names
                .first()
                .expect("authorization requires a selection")
        });

    save(repository, identity_names)?;
    apply_identity(repository, preferred, &config.identities[preferred])?;

    if repository.local("ghguard.account")?.is_some() {
        repository.run(&["config", "--local", "--unset-all", "ghguard.account"])?;
    }

    Ok(())
}

pub fn save(repository: &Repository, identity_names: &BTreeSet<String>) -> Result<()> {
    migrate(repository)?;

    repository.run(&[
        "config",
        "--local",
        "--replace-all",
        ALLOWED,
        identity_names.first().map(String::as_str).unwrap_or(""),
    ])?;

    for name in identity_names.iter().skip(1) {
        repository.run(&["config", "--local", "--add", ALLOWED, name])?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ALLOWED, CURRENT, decode};
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn explicit_empty_permissions_override_every_legacy_fallback() {
        let values = BTreeMap::from([
            (ALLOWED.into(), vec![String::new()]),
            (CURRENT.into(), vec!["personal".into()]),
            ("mgh.allowedaccount".into(), vec!["work".into()]),
            ("ghguard.account".into(), vec!["school".into()]),
        ]);

        assert!(decode(&values).unwrap().is_empty());
    }

    #[test]
    fn explicit_permissions_normalize_names_and_take_precedence_over_pins() {
        let values = BTreeMap::from([
            (
                ALLOWED.into(),
                vec!["WORK".into(), "personal".into(), "Work".into()],
            ),
            (CURRENT.into(), vec!["school".into()]),
            ("ghguard.account".into(), vec!["other".into()]),
        ]);

        assert_eq!(
            decode(&values).unwrap(),
            BTreeSet::from(["personal".into(), "work".into()])
        );
    }

    #[test]
    fn legacy_pins_must_agree_when_explicit_permissions_are_absent() {
        let mut values = BTreeMap::from([
            (CURRENT.into(), vec!["Personal".into()]),
            ("ghguard.account".into(), vec!["personal".into()]),
        ]);
        assert_eq!(
            decode(&values).unwrap(),
            BTreeSet::from(["personal".into()])
        );

        values.insert("ghguard.account".into(), vec!["work".into()]);
        assert!(
            decode(&values)
                .unwrap_err()
                .to_string()
                .contains("settings conflict")
        );
    }
}
