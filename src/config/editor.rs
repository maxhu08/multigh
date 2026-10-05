use super::{Config, JSONC};
use crate::utils::storage;
use anyhow::{Context, Result, ensure};
use jsonc_parser::{
    cst::{CstObject, CstObjectProp, CstRootNode},
    json,
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

pub struct Editor {
    path: PathBuf,
    original: Option<String>,
    config: Option<Config>,
}

pub struct PendingUpdate {
    editor: Editor,
    file: NamedTempFile,
    config: Config,
}

fn read(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("Read {}", path.display())),
    }
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
}

impl Editor {
    pub fn open(path: PathBuf) -> Result<Self> {
        let path = super::path(Some(path))?;
        ensure!(
            !is_symlink(&path),
            "Configuration file is a symlink; use --config with its target: {}",
            path.display()
        );
        let original = read(&path)?;
        let config = original
            .as_ref()
            .map(|text| Config::parse(path.clone(), text))
            .transpose()?;

        Ok(Self {
            path,
            original,
            config,
        })
    }

    pub fn config(&self) -> Option<&Config> {
        self.config.as_ref()
    }

    pub fn add(
        self,
        identity_name: &str,
        username: &str,
        commit_name: &str,
        email: &str,
    ) -> Result<PendingUpdate> {
        let text = if let Some(original) = &self.original {
            let document = CstRootNode::parse(original, &JSONC)
                .expect("the original snapshot was validated on open");
            let object = document
                .object_value()
                .expect("validated configuration is an object");
            object.append(
                identity_name,
                json!({
                    "username": username,
                    "commit": {"name": commit_name, "email": email}
                }),
            );
            document.to_string()
        } else {
            let document = serde_json::json!({identity_name: {
                "username": username, "commit": {"name": commit_name, "email": email}
            }});
            let text = serde_json::to_string_pretty(&document)
                .expect("string-only identity JSON is serializable");
            format!("{text}\n")
        };

        self.prepare(&text)
    }

    pub fn edit(
        self,
        identity_name: &str,
        username: &str,
        commit_name: &str,
        email: &str,
    ) -> Result<PendingUpdate> {
        let document = self.document();
        let property = find_identity(&document, identity_name);
        let object = property
            .object_value()
            .expect("validated identity is an object");
        let commit = object
            .object_value("commit")
            .expect("validated commit details are an object");

        set(&object, "username", username);
        set(&commit, "name", commit_name);
        set(&commit, "email", email);

        self.prepare(&document.to_string())
    }

    pub fn remove(self, identity_name: &str) -> Result<PendingUpdate> {
        let document = self.document();
        find_identity(&document, identity_name).remove();
        self.prepare(&document.to_string())
    }

    fn document(&self) -> CstRootNode {
        let original = self
            .original
            .as_deref()
            .expect("edit and remove require an existing identity");
        CstRootNode::parse(original, &JSONC).expect("the original snapshot was validated on open")
    }

    fn prepare(self, text: &str) -> Result<PendingUpdate> {
        let config = Config::parse(self.path.clone(), text)?;
        let parent = self
            .path
            .parent()
            .expect("absolute configuration filepath has a parent directory");
        let create_parent = !parent.exists();
        fs::create_dir_all(parent)?;
        if create_parent {
            storage::private(parent, true)?;
        }

        let mut file = NamedTempFile::new_in(parent)?;
        file.write_all(text.as_bytes())?;
        storage::private(file.path(), false)?;

        Ok(PendingUpdate {
            editor: self,
            file,
            config,
        })
    }
}

impl PendingUpdate {
    pub fn save(self, conflict_message: &str) -> Result<Config> {
        ensure!(
            !is_symlink(&self.editor.path) && read(&self.editor.path)? == self.editor.original,
            "{conflict_message}"
        );
        self.file.persist(&self.editor.path)?;
        Ok(self.config)
    }
}

fn find_identity(document: &CstRootNode, identity_name: &str) -> CstObjectProp {
    document
        .object_value()
        .expect("validated configuration is an object")
        .properties()
        .into_iter()
        .find(|property| {
            property
                .decoded_name()
                .is_some_and(|key| key.eq_ignore_ascii_case(identity_name))
        })
        .expect("the command resolved this identity before editing")
}

fn set(object: &CstObject, key: &str, value: &str) {
    if let Some(property) = object.get(key) {
        property.set_value(json!(value));
    } else {
        object.append(key, json!(value));
    }
}

#[cfg(test)]
mod tests {
    use super::Editor;
    use std::fs;

    #[test]
    fn prepared_identity_is_not_visible_until_saved_and_dropping_it_cleans_up() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("identities.jsonc");
        let pending = Editor::open(path.clone())
            .unwrap()
            .add("personal", "alice", "Alice Example", "alice@example.com")
            .unwrap();

        assert!(!path.exists());
        drop(pending);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);

        let config = Editor::open(path.clone())
            .unwrap()
            .add("personal", "alice", "Alice Example", "alice@example.com")
            .unwrap()
            .save("Configuration changed")
            .unwrap();
        assert_eq!(config.path, path);
        assert_eq!(config.identity("PERSONAL").unwrap().username, "alice");
        assert!(path.is_file());
    }

    #[test]
    fn concurrent_creation_and_deletion_are_preserved() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("identities.jsonc");
        let pending = Editor::open(path.clone())
            .unwrap()
            .add("personal", "alice", "Alice Example", "alice@example.com")
            .unwrap();
        fs::write(&path, "// Created by another process\n{}\n").unwrap();

        assert!(pending.save("Configuration changed").is_err());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "// Created by another process\n{}\n"
        );

        let pending = Editor::open(path.clone())
            .unwrap()
            .add("personal", "alice", "Alice Example", "alice@example.com")
            .unwrap();
        fs::remove_file(&path).unwrap();

        assert!(pending.save("Configuration changed").is_err());
        assert!(!path.exists());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}
