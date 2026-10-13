use anyhow::{Context, Result, ensure};
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

pub struct Config {
    pub identities: BTreeMap<String, Identity>,
}

pub struct Identity {
    pub username: String,
    pub commit: CommitDetails,
    accepted_emails: BTreeSet<String>,
}

impl Identity {
    pub fn matches_username(&self, username: &str) -> bool {
        self.username.eq_ignore_ascii_case(username)
    }
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

                while let Some(name) = map.next_key::<String>()? {
                    let name = name.to_ascii_lowercase();

                    if !name.starts_with(|c: char| c.is_ascii_lowercase())
                        || !name.chars().all(|c| {
                            c.is_ascii_lowercase() || c.is_ascii_digit() || "_-".contains(c)
                        })
                    {
                        return Err(de::Error::custom(format!("Invalid identity name: {name}")));
                    }

                    if identities.contains_key(&name) {
                        return Err(de::Error::custom(format!(
                            "Duplicate identity name: {name}"
                        )));
                    }

                    identities.insert(name, map.next_value()?);
                }

                Ok(Definitions(identities))
            }
        }

        deserializer.deserialize_map(DefinitionsVisitor)
    }
}

const JSONC: ParseOptions = ParseOptions {
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

fn path() -> Result<PathBuf> {
    let directory = match env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        Some(directory) => PathBuf::from(directory),
        None => {
            let home = env::var_os("HOME")
                .or_else(|| env::var_os("USERPROFILE"))
                .context("Cannot locate your home directory")?;

            PathBuf::from(home).join(".config")
        }
    };
    let path = directory
        .join(env!("CARGO_PKG_NAME"))
        .join("identities.jsonc");

    Ok(if path.is_absolute() {
        path
    } else {
        env::current_dir()?.join(path)
    })
}

impl Config {
    pub fn identity_for_username(&self, username: &str) -> Option<(&str, &Identity)> {
        self.identities
            .iter()
            .find(|(_, identity)| identity.matches_username(username))
            .map(|(name, identity)| (name.as_str(), identity))
    }

    pub fn load() -> Result<Self> {
        let path = path()?;
        let text = fs::read_to_string(&path).with_context(|| {
            format!(
                "Could not read identity configuration at {}",
                path.display()
            )
        })?;

        Self::parse(&text)
            .with_context(|| format!("Invalid identity configuration at {}", path.display()))
    }

    fn parse(text: &str) -> Result<Self> {
        let Definitions(definitions) = jsonc_parser::parse_to_serde_value(text, &JSONC)?;
        let mut identities = BTreeMap::new();

        for (identity_name, definition) in definitions {
            let username = definition.username.trim().to_owned();

            ensure!(
                username.starts_with(|c: char| c.is_ascii_alphanumeric())
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
            let mut accepted_emails: BTreeSet<_> = definition
                .commit
                .additional_emails
                .iter()
                .map(|email| email.trim().to_ascii_lowercase())
                .collect();
            accepted_emails.insert(email.to_ascii_lowercase());

            for email in &accepted_emails {
                let parts: Vec<_> = email.split('@').collect();

                ensure!(
                    parts.len() == 2
                        && parts.iter().all(|part| !part.is_empty())
                        && !email.chars().any(|c| c.is_whitespace() || "<>".contains(c)),
                    "Invalid email in [{identity_name}]"
                );
            }

            ensure!(
                !identities.values().any(|other: &Identity| {
                    other.matches_username(&username)
                        || !other.accepted_emails.is_disjoint(&accepted_emails)
                }),
                "Identities cannot share usernames or email addresses"
            );

            identities.insert(
                identity_name,
                Identity {
                    username,
                    commit: CommitDetails { name, email },
                    accepted_emails,
                },
            );
        }

        Ok(Self { identities })
    }
}
