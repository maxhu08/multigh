pub mod editor;

use anyhow::{Context, Result, bail, ensure};
use jsonc_parser::ParseOptions;
use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, Visitor},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fmt, fs,
    path::PathBuf,
};

pub struct Identity {
    pub username: String,
    pub commit_name: String,
    pub commit_email: String,
    pub allowed_emails: BTreeSet<String>,
}

impl Identity {
    pub fn matches_username(&self, username: &str) -> bool {
        self.username.eq_ignore_ascii_case(username)
    }

    pub fn accepts_email(&self, email: &str) -> bool {
        self.allowed_emails.contains(&email.to_ascii_lowercase())
    }

    pub fn matches_commit(&self, name: &str, email: &str) -> bool {
        name == self.commit_name && self.accepts_email(email)
    }
}

pub struct Config {
    pub path: PathBuf,
    pub identities: BTreeMap<String, Identity>,
}

pub struct CommitDetails {
    pub name: String,
    pub email: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    username: String,
    commit: Commit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Commit {
    name: Option<String>,
    email: String,
    #[serde(default)]
    additional_emails: Vec<String>,
}

struct Definitions(BTreeMap<String, Definition>);

impl<'de> Deserialize<'de> for Definitions {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct DefinitionsVisitor;

        impl<'de> Visitor<'de> for DefinitionsVisitor {
            type Value = Definitions;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an object keyed by identity names")
            }

            fn visit_map<M: MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<Self::Value, M::Error> {
                let mut identities = BTreeMap::new();

                while let Some(key) = map.next_key::<String>()? {
                    let key = key.to_ascii_lowercase();

                    if !key.starts_with(|c: char| c.is_ascii_lowercase())
                        || !key.chars().all(|c| {
                            c.is_ascii_lowercase() || c.is_ascii_digit() || "_-".contains(c)
                        })
                    {
                        return Err(de::Error::custom(format!("Invalid identity name: {key}")));
                    }

                    if identities.contains_key(&key) {
                        return Err(de::Error::custom(format!("Duplicate identity name: {key}")));
                    }

                    identities.insert(key, map.next_value()?);
                }

                Ok(Definitions(identities))
            }
        }

        deserializer.deserialize_map(DefinitionsVisitor)
    }
}

pub const JSONC: ParseOptions = ParseOptions {
    allow_comments: true,
    allow_trailing_commas: true,
    allow_loose_object_property_names: false,
    allow_missing_commas: false,
    allow_single_quoted_strings: false,
    allow_hexadecimal_numbers: false,
    allow_unary_plus_numbers: false,
    allow_bare_decimal_point_numbers: false,
    allow_non_finite_numbers: false,
    allow_extended_string_escapes: false,
};

pub fn path(explicit: Option<PathBuf>) -> Result<PathBuf> {
    let path = match explicit {
        Some(path) => path,
        None => crate::storage::directory("XDG_CONFIG_HOME", ".config")?
            .join("multigh/identities.jsonc"),
    };

    Ok(if path.is_absolute() {
        path
    } else {
        env::current_dir()?.join(path)
    })
}

impl Config {
    pub fn load(path: PathBuf) -> Result<Self> {
        let text = fs::read_to_string(&path).with_context(|| {
            format!(
                "Read {} (create it using examples/identities.jsonc)",
                path.display()
            )
        })?;

        Self::parse(path, &text)
    }

    pub fn parse(path: PathBuf, text: &str) -> Result<Self> {
        let Definitions(parsed) = jsonc_parser::parse_to_serde_value(text, &JSONC)
            .with_context(|| format!("Read {}: invalid identity configuration", path.display()))?;
        let mut identities = BTreeMap::new();

        for (identity_name, definition) in parsed {
            let username = definition.username.trim().to_owned();

            ensure!(
                !username.is_empty()
                    && username.starts_with(|c: char| c.is_ascii_alphanumeric())
                    && username
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "Invalid username in [{identity_name}]"
            );

            let name = definition
                .commit
                .name
                .as_deref()
                .unwrap_or(&username)
                .trim()
                .to_owned();

            ensure!(
                !name.is_empty() && !name.chars().any(|c| c.is_control() || "<>".contains(c)),
                "Invalid commit name in [{identity_name}]"
            );

            let email = definition.commit.email.trim().to_owned();
            let mut emails: BTreeSet<_> = definition
                .commit
                .additional_emails
                .iter()
                .map(|email| email.trim().to_ascii_lowercase())
                .collect();

            emails.insert(email.to_ascii_lowercase());

            for email in &emails {
                let parts: Vec<_> = email.split('@').collect();

                ensure!(
                    parts.len() == 2
                        && parts.iter().all(|part| !part.is_empty())
                        && !email.chars().any(|c| c.is_whitespace() || "<>".contains(c)),
                    "Invalid email in [{identity_name}]"
                );
            }

            ensure!(
                !identities
                    .values()
                    .any(|other: &Identity| other.matches_username(&username)
                        || !other.allowed_emails.is_disjoint(&emails)),
                "Identities cannot share usernames or email addresses"
            );

            identities.insert(
                identity_name,
                Identity {
                    username,
                    commit_name: name,
                    commit_email: email,
                    allowed_emails: emails,
                },
            );
        }

        Ok(Self { path, identities })
    }

    pub fn identity(&self, identity_name: &str) -> Result<&Identity> {
        match self.identities.get(&identity_name.to_ascii_lowercase()) {
            Some(identity) => Ok(identity),
            None => bail!(
                "Unknown identity '{identity_name}'; choose {}",
                self.identities
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    pub fn identity_for_username(&self, username: &str) -> Option<(&str, &Identity)> {
        self.identities
            .iter()
            .find(|(_, identity)| identity.matches_username(username))
            .map(|(name, identity)| (name.as_str(), identity))
    }
}
