use crate::{
    config::{Config, JSONC},
    git, github, output, policy, process,
};
use anyhow::{Context, Result, ensure};
use jsonc_parser::{cst::CstRootNode, json};
use serde::Serialize;
use std::{fs, io::Write, path::PathBuf};
use tempfile::NamedTempFile;

pub fn run(
    path: PathBuf,
    identity_name: Option<String>,
    username: Option<String>,
    email: Option<String>,
    name: Option<String>,
    repo: bool,
) -> Result<()> {
    ensure!(
        !repo || policy::repository()?,
        "--repo must be run inside a Git repository"
    );
    ensure!(
        !fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()),
        "Configuration file is a symlink; use --config with its target: {}",
        path.display()
    );

    let original = match fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error).with_context(|| format!("Read {}", path.display())),
    };
    let existing = original
        .as_ref()
        .map(|_| Config::load(path.clone()))
        .transpose()?;

    if policy::interactive() {
        cliclack::intro("Add new identity")?;
    } else {
        output::section("Add new identity");
    }

    let identity_name = field(identity_name, "Identity name", None)?.to_ascii_lowercase();
    let username = field(username, "GitHub username", None)?;
    let name = field(name, "Commit name", Some(&username))?;
    let email = field(email, "Commit email", None)?;

    if policy::interactive() {
        cliclack::outro("Identity details entered")?;
    }

    for value in [&identity_name, &username, &name, &email] {
        ensure!(
            !value.chars().any(char::is_control),
            "Identity fields must be single-line values"
        );
    }

    ensure!(
        existing
            .as_ref()
            .is_none_or(|config| !config.identities.contains_key(&identity_name)),
        "Identity '{identity_name}' already exists"
    );

    let parent = path
        .parent()
        .context("Configuration file has no parent directory")?;
    let create_parent = !parent.exists();

    fs::create_dir_all(parent)?;

    if create_parent {
        git::private(parent, true)?;
    }

    let mut pending = NamedTempFile::new_in(parent)?;

    if let Some(original) = &original {
        let document = CstRootNode::parse(original, &JSONC)?;

        document
            .object_value()
            .context("Configuration must be an object")?
            .append(
                &identity_name,
                json!({
                    "username": (username.as_str()),
                    "commit": {"name": name, "email": email}
                }),
            );

        write!(pending, "{document}")?;
    } else {
        let document = serde_json::json!({
            identity_name.as_str(): {
                "username": username,
                "commit": {"name": name, "email": email}
            }
        });
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut serializer = serde_json::Serializer::with_formatter(&mut pending, formatter);

        document.serialize(&mut serializer)?;
        writeln!(pending)?;
    }

    let mut config = Config::load(pending.path().to_owned())?;

    github::login(&username)?;

    let current = match fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };

    ensure!(
        current == original
            && !fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()),
        "Configuration changed while signing in; run mgh new again"
    );

    git::private(pending.path(), false)?;
    pending.persist(&path)?;
    config.path = path;

    output::section(&format!("✓ Identity added · {}", identity_name));
    output::row(
        "Accounts",
        &config.path.to_string_lossy(),
        output::Color::Changed,
    );

    super::setup(&config)
        .and_then(|()| super::switch::run(&config, &identity_name, repo))
        .with_context(|| {
            let config_option = format!("--config {}", process::quote(&config.path.to_string_lossy()));

            format!(
                "Identity saved in {}. After fixing the problem, run mgh {config_option} setup and mgh {config_option} switch {identity_name}{}",
                config.path.display(),
                if repo { " --repo" } else { "" }
            )
        })
}

fn field(value: Option<String>, label: &str, default: Option<&str>) -> Result<String> {
    if let Some(value) = value {
        return Ok(value.trim().to_owned());
    }

    if !policy::interactive() {
        return default.map(str::to_owned).with_context(|| {
            format!("{label} requires a terminal; supply the identity name, --username and --email")
        });
    }

    let message = match default {
        Some(default) => format!("{label} (default: {default}): "),
        None => format!("{label}: "),
    };
    let mut prompt = cliclack::input(message).required(false);

    if let Some(default) = default {
        prompt = prompt.default_input(default);
    }

    let answer: String = prompt.interact()?;
    let answer = answer.trim();

    Ok(if answer.is_empty() {
        default.unwrap_or(answer)
    } else {
        answer
    }
    .to_owned())
}
