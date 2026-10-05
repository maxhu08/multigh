use super::{entries, global, run};
use crate::{config::Config, utils::storage};
use anyhow::{Result, ensure};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

pub fn directory() -> Result<PathBuf> {
    Ok(storage::directory("XDG_STATE_HOME", ".local/state")?.join("multigh/identities"))
}

pub fn refresh(config: &Config, global: &mut global::Writer<'_>) -> Result<PathBuf> {
    let directory = directory()?;
    let previous = entries(Some("--global"), "^[iI]nclude[iI]f\\.")?;
    let mut desired = BTreeMap::new();

    for (identity_name, identity) in &config.identities {
        let identity_file = directory.join(format!("git-{identity_name}.conf"));

        ensure!(
            !fs::symlink_metadata(&identity_file)
                .is_ok_and(|metadata| metadata.file_type().is_symlink()),
            "Identity file is a symlink: {}",
            identity_file.display()
        );

        for prefix in [
            "https://github.com/",
            "git@github.com:",
            "ssh://git@github.com/",
        ] {
            let key = format!(
                "includeif.hasconfig:remote.*.url:{prefix}{}/**.path",
                identity.username
            );

            ensure!(
                previous.get(&key).is_none_or(|values| values
                    .iter()
                    .all(|value| Path::new(value).parent() == Some(directory.as_path()))),
                "Existing Git identity rule conflicts with [{identity_name}]; no rules changed"
            );

            desired.insert(key, identity_file.clone());
        }
    }

    fs::create_dir_all(&directory)?;
    storage::private(&directory, true)?;

    for (identity_name, identity) in &config.identities {
        let temporary = NamedTempFile::new_in(&directory)?;
        let path = temporary.path().to_string_lossy();

        let comment = global.comment();

        for (key, value) in [
            ("user.name", &identity.commit_name),
            ("user.email", &identity.commit_email),
        ] {
            run(&["config", "--file", &path, "--comment", &comment, key, value])?;
        }

        temporary.persist(directory.join(format!("git-{identity_name}.conf")))?;
    }

    for (key, values) in &previous {
        if desired.get(key).is_some_and(|identity_file| {
            values.as_slice() == [identity_file.to_string_lossy().as_ref()]
        }) {
            continue;
        }

        for value in values.iter().collect::<std::collections::BTreeSet<_>>() {
            if Path::new(&value).parent() == Some(directory.as_path()) {
                global.edit(&["--fixed-value", "--unset-all", key, value])?;
            }
        }
    }

    for (key, identity_file) in desired {
        if previous
            .get(&key)
            .is_some_and(|values| values.as_slice() == [identity_file.to_string_lossy().as_ref()])
        {
            continue;
        }

        global.edit(&["--add", &key, &identity_file.to_string_lossy()])?;
    }

    Ok(directory)
}
