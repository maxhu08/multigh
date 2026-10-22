use super::platform;
use anyhow::{Context, Result, bail, ensure};
use jsonc_parser::{
    ParseOptions,
    cst::{CstInputValue, CstNode, CstObject, CstRootNode},
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Clone, Debug)]
pub(super) struct Identity {
    name: String,
    username: String,
    commit_name: String,
    email: String,
    accepted: BTreeSet<String>,
}

impl Identity {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn commit_name(&self) -> &str {
        &self.commit_name
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn accepts(&self, email: &str) -> bool {
        self.accepted.contains(&email.to_lowercase())
    }
}

#[derive(Clone, Debug)]
pub(super) struct IdentitySelector(String);
impl IdentitySelector {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
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
    email: String,
    name: Option<String>,
    #[serde(default)]
    additional_emails: Vec<String>,
}

pub(super) struct Catalog {
    path: PathBuf,
    identities: BTreeMap<String, Identity>,
}

impl Catalog {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn all(&self) -> impl Iterator<Item = &Identity> {
        self.identities.values()
    }

    pub fn is_empty(&self) -> bool {
        self.identities.is_empty()
    }

    pub fn resolve(&self, selector: &IdentitySelector) -> Result<&Identity> {
        self.identities
            .get(&selector.0.to_ascii_lowercase())
            .with_context(|| {
                format!(
                    "Unknown identity '{}'; choose {}",
                    selector.0,
                    self.identities
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
    }

    pub fn find(&self, name: &str) -> Option<&Identity> {
        self.identities.get(&name.to_ascii_lowercase())
    }

    pub fn load(path: PathBuf, missing_ok: bool) -> Result<Self> {
        match fs::read_to_string(&path) {
            Ok(text) => Self::parse(path, &text),
            Err(e) if missing_ok && e.kind() == io::ErrorKind::NotFound => Self::parse(path, "{}"),
            Err(e) => Err(e)
                .with_context(|| format!("Cannot read identity configuration {}", path.display())),
        }
    }

    fn parse(path: PathBuf, text: &str) -> Result<Self> {
        let result = (|| {
            let root = parse(text)?;
            let object = root.object_value().context("Root must be an object")?;
            let mut identities = BTreeMap::new();
            let mut usernames = BTreeSet::new();
            let mut emails = BTreeSet::new();
            for prop in object.properties() {
                let key = prop.decoded_name().context("Invalid identity key")?;
                validate_name(&key)?;
                let name = key.to_ascii_lowercase();
                ensure!(
                    !identities.contains_key(&name),
                    "Duplicate/case-colliding identity '{key}'"
                );
                let def: Definition = serde_json::from_value(
                    prop.to_serde_value().context("Missing identity value")?,
                )?;
                let username = def.username.trim().to_string();
                validate_username(&username)?;
                let commit_name = def
                    .commit
                    .name
                    .unwrap_or_else(|| username.clone())
                    .trim()
                    .to_string();
                validate_commit_name(&commit_name)?;
                let email = def.commit.email.trim().to_string();
                validate_email(&email)?;
                let mut accepted = BTreeSet::from([email.to_lowercase()]);
                for additional in def.commit.additional_emails {
                    let a = additional.trim();
                    validate_email(a)?;
                    accepted.insert(a.to_lowercase());
                }
                ensure!(
                    usernames.insert(username.to_lowercase()),
                    "Identities cannot share usernames or email addresses"
                );
                for a in &accepted {
                    ensure!(
                        emails.insert(a.clone()),
                        "Identities cannot share usernames or email addresses"
                    );
                }
                identities.insert(
                    name.clone(),
                    Identity {
                        name,
                        username,
                        commit_name,
                        email,
                        accepted,
                    },
                );
            }
            Ok(Self {
                path: path.clone(),
                identities,
            })
        })();
        result.with_context(|| format!("{}: invalid identity configuration", path.display()))
    }
}

fn parse(text: &str) -> Result<CstRootNode> {
    let root = CstRootNode::parse(
        text,
        &ParseOptions {
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
        },
    )?;
    let value = root.value().context("Configuration contains no value")?;
    duplicates(&value)?;
    Ok(root)
}

fn duplicates(node: &CstNode) -> Result<()> {
    if let Some(o) = node.as_object() {
        let mut keys = BTreeSet::new();
        for p in o.properties() {
            let key = p.decoded_name().context("Invalid property")?;
            ensure!(keys.insert(key.clone()), "Duplicate field '{key}'");
        }
    }
    for child in node.children() {
        duplicates(&child)?;
    }
    Ok(())
}

pub(super) fn config_path(selected: Option<&Path>) -> Result<PathBuf> {
    match selected {
        Some(p) if p.is_absolute() => Ok(p.to_path_buf()),
        Some(p) => Ok(std::env::current_dir()?.join(p)),
        None => Ok(platform::location("XDG_CONFIG_HOME", ".config")?.join("identities.jsonc")),
    }
}

pub(super) fn single_line(value: &str) -> Result<()> {
    ensure!(
        !value.chars().any(char::is_control),
        "Identity fields must be single-line values"
    );
    Ok(())
}

fn validate_name(s: &str) -> Result<()> {
    ensure!(
        s.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
            && s.bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-'),
        "Invalid identity name '{s}'"
    );
    Ok(())
}

fn validate_username(s: &str) -> Result<()> {
    ensure!(
        s.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
            && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'),
        "Invalid username '{s}'"
    );
    Ok(())
}

fn validate_commit_name(s: &str) -> Result<()> {
    ensure!(
        !s.is_empty() && !s.chars().any(|c| c.is_control() || c == '<' || c == '>'),
        "Invalid commit name '{s}'"
    );
    Ok(())
}

fn validate_email(s: &str) -> Result<()> {
    let parts = s.split('@').collect::<Vec<_>>();
    ensure!(
        parts.len() == 2
            && parts.iter().all(|s| !s.is_empty())
            && !s.chars().any(|c| c.is_whitespace() || c == '<' || c == '>'),
        "Invalid email '{s}'"
    );
    Ok(())
}

#[derive(PartialEq)]
struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
    #[cfg(unix)]
    ino: u64,
    #[cfg(unix)]
    dev: u64,
}

fn stamp(m: fs::Metadata) -> Stamp {
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    Stamp {
        modified: m.modified().ok(),
        len: m.len(),
        #[cfg(unix)]
        ino: m.ino(),
        #[cfg(unix)]
        dev: m.dev(),
    }
}

pub(super) struct EditSession {
    path: PathBuf,
    original: Option<(Vec<u8>, Stamp)>,
    root: CstRootNode,
    catalog: Catalog,
}

pub(super) struct PreparedIdentityChange {
    session: EditSession,
    pending: tempfile::NamedTempFile,
    catalog: Catalog,
}

impl EditSession {
    pub fn open(path: PathBuf) -> Result<Self> {
        platform::reject_symlink(&path, "Identity configuration")?;
        let original = match fs::read(&path) {
            Ok(bytes) => Some((bytes, stamp(fs::metadata(&path)?))),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e).with_context(|| format!("Cannot read {}", path.display())),
        };
        let text = match &original {
            Some((bytes, _)) => std::str::from_utf8(bytes)?,
            None => "{}",
        };
        let catalog = Catalog::parse(path.clone(), text)?;
        let root = parse(text)?;
        Ok(Self {
            path,
            original,
            root,
            catalog,
        })
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    pub fn create(
        self,
        name: &str,
        username: &str,
        email: &str,
        commit_name: &str,
    ) -> Result<PreparedIdentityChange> {
        for value in [name, username, email, commit_name] {
            single_line(value)?;
        }
        validate_name(name)?;
        let key = name.to_ascii_lowercase();
        ensure!(
            self.catalog.find(&key).is_none(),
            "Identity '{name}' already exists"
        );
        self.root.object_value().unwrap().append(
            &key,
            CstInputValue::Object(vec![
                (
                    "username".into(),
                    CstInputValue::String(username.trim().into()),
                ),
                (
                    "commit".into(),
                    CstInputValue::Object(vec![
                        (
                            "name".into(),
                            CstInputValue::String(commit_name.trim().into()),
                        ),
                        ("email".into(), CstInputValue::String(email.trim().into())),
                    ]),
                ),
            ]),
        );
        self.pending()
    }

    pub fn edit(
        self,
        name: &str,
        username: Option<&str>,
        email: Option<&str>,
        commit_name: Option<&str>,
    ) -> Result<PreparedIdentityChange> {
        self.catalog.resolve(&IdentitySelector::new(name))?;
        for value in [username, email, commit_name].into_iter().flatten() {
            single_line(value)?;
        }
        let prop = self
            .root
            .object_value()
            .unwrap()
            .properties()
            .into_iter()
            .find(|p| p.decoded_name().unwrap().eq_ignore_ascii_case(name))
            .unwrap();
        let object = prop.value().unwrap().as_object().unwrap();
        if let Some(username) = username {
            update(&object, "username", username.trim());
        }
        let commit = object.object_value("commit").unwrap();
        if let Some(email) = email {
            update(&commit, "email", email.trim());
        }
        if let Some(name) = commit_name {
            update(&commit, "name", name.trim());
        }
        self.pending()
    }

    pub fn remove(self, name: &str) -> Result<PreparedIdentityChange> {
        self.catalog.resolve(&IdentitySelector::new(name))?;
        self.root
            .object_value()
            .unwrap()
            .properties()
            .into_iter()
            .find(|p| p.decoded_name().unwrap().eq_ignore_ascii_case(name))
            .unwrap()
            .remove();
        self.pending()
    }

    fn pending(self) -> Result<PreparedIdentityChange> {
        let text = self.root.to_string();
        let catalog = Catalog::parse(self.path.clone(), &text)?;
        let parent = self.path.parent().context("Identity file has no parent")?;
        platform::private_dir(parent)?;
        let mut pending = tempfile::Builder::new()
            .prefix(".mgh-pending-")
            .tempfile_in(parent)?;
        pending.write_all(text.as_bytes())?;
        pending.as_file().sync_all()?;
        Ok(PreparedIdentityChange {
            session: self,
            pending,
            catalog,
        })
    }
}

fn update(object: &CstObject, key: &str, value: &str) {
    if let Some(p) = object.get(key) {
        if p.value()
            .and_then(|v| v.as_string_lit())
            .and_then(|s| s.decoded_value().ok())
            .as_deref()
            != Some(value)
        {
            p.set_value(CstInputValue::String(value.into()));
        }
    } else {
        object.append(key, CstInputValue::String(value.into()));
    }
}

impl PreparedIdentityChange {
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    pub fn save(self, signing_in: bool) -> Result<Catalog> {
        let path = &self.session.path;
        platform::reject_symlink(path, "Identity configuration")?;
        let current = match fs::read(path) {
            Ok(bytes) => Some((bytes, stamp(fs::metadata(path)?))),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e).with_context(|| format!("Cannot recheck {}", path.display())),
        };
        if current != self.session.original {
            bail!(
                "Configuration changed {}: {}",
                if signing_in {
                    "while signing in"
                } else {
                    "during editing"
                },
                path.display()
            );
        }
        if current.is_none() {
            self.pending.persist_noclobber(path).map_err(|e| e.error)?;
        } else {
            self.pending.persist(path).map_err(|e| e.error)?;
        }
        Ok(self.catalog)
    }
}
