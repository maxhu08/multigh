use anyhow::{Context, Result, bail, ensure};
use ini::{Ini, ParseOption};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    path::PathBuf,
};

pub struct Account {
    pub username: String,
    pub name: String,
    pub email: String,
    pub emails: BTreeSet<String>,
}

pub struct Config {
    pub path: PathBuf,
    pub accounts: BTreeMap<String, Account>,
}

pub fn directory(variable: &str, fallback: &str) -> Result<PathBuf> {
    if let Some(value) = env::var_os(variable).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(value));
    }

    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .context("Cannot locate your home directory")?;

    Ok(PathBuf::from(home).join(fallback))
}

pub fn path(explicit: Option<PathBuf>) -> Result<PathBuf> {
    let path = match explicit {
        Some(path) => path,
        None => directory("XDG_CONFIG_HOME", ".config")?.join("multigh/accounts.conf"),
    };

    Ok(if path.is_absolute() {
        path
    } else {
        env::current_dir()?.join(path)
    })
}

impl Config {
    pub fn load(path: PathBuf) -> Result<Self> {
        let options = ParseOption {
            enabled_quote: false,
            enabled_escape: false,
            enabled_indented_mutiline_value: true,
            ..Default::default()
        };

        let parsed = Ini::load_from_file_opt(&path, options).with_context(|| {
            format!(
                "Read {} (create it using example/accounts.conf)",
                path.display()
            )
        })?;

        let mut accounts = BTreeMap::new();

        for (section, values) in &parsed {
            let Some(alias) = section else {
                ensure!(
                    values.is_empty(),
                    "Account fields must be inside a named section"
                );
                continue;
            };

            let alias = alias.to_ascii_lowercase();

            ensure!(
                alias.starts_with(|c: char| c.is_ascii_lowercase())
                    && alias
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_-".contains(c)),
                "Invalid account section: {alias}"
            );
            ensure!(
                !accounts.contains_key(&alias),
                "Duplicate account section: {alias}"
            );

            let mut keys = BTreeSet::new();

            for (key, _) in values {
                ensure!(
                    ["username", "name", "email", "allowed_emails"].contains(&key)
                        && keys.insert(key),
                    "Unknown or duplicate field in [{alias}]: {key}"
                );
            }

            let username = values.get("username").unwrap_or_default().trim().to_owned();

            ensure!(
                !username.is_empty()
                    && username.starts_with(|c: char| c.is_ascii_alphanumeric())
                    && username
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "Invalid username in [{alias}]"
            );

            let name = values.get("name").unwrap_or(&username).trim().to_owned();

            ensure!(
                !name.is_empty() && !name.chars().any(|c| c.is_control() || "<>".contains(c)),
                "Invalid commit name in [{alias}]"
            );

            let email = values.get("email").unwrap_or_default().trim().to_owned();
            let mut emails: BTreeSet<_> = values
                .get("allowed_emails")
                .unwrap_or_default()
                .lines()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_ascii_lowercase)
                .collect();

            emails.insert(email.to_ascii_lowercase());

            for email in &emails {
                let parts: Vec<_> = email.split('@').collect();

                ensure!(
                    parts.len() == 2
                        && parts.iter().all(|part| !part.is_empty())
                        && !email.chars().any(|c| c.is_whitespace() || "<>".contains(c)),
                    "Invalid email in [{alias}]"
                );
            }

            ensure!(
                !accounts.values().any(|other: &Account| other
                    .username
                    .eq_ignore_ascii_case(&username)
                    || !other.emails.is_disjoint(&emails)),
                "Accounts cannot share usernames or email addresses"
            );

            accounts.insert(
                alias,
                Account {
                    username,
                    name,
                    email,
                    emails,
                },
            );
        }

        ensure!(!accounts.is_empty(), "Add an account to {}", path.display());

        Ok(Self { path, accounts })
    }

    pub fn account(&self, alias: &str) -> Result<&Account> {
        match self.accounts.get(&alias.to_ascii_lowercase()) {
            Some(account) => Ok(account),
            None => bail!(
                "Unknown account '{alias}'; choose {}",
                self.accounts
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}
