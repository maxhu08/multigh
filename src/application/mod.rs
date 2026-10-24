mod accounts;
mod identities;
mod platform;
mod preferences;
mod repositories;
mod shell;
mod workflows;

use anyhow::Result;
pub use preferences::Preference;
pub enum ShellKind {
    Fish,
    Bash,
    Zsh,
}
use std::{
    ffi::OsString,
    fmt,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Cancelled;
impl fmt::Display for Cancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Operation cancelled.")
    }
}

impl std::error::Error for Cancelled {}
#[derive(Clone, Copy)]
pub enum Meaning {
    Heading,
    HeadingArrow,
    Identity,
    Username,
    Commit,
    Path,
    Success,
    Warning,
    Error,
    Plain,
}

pub struct Choice {
    pub name: String,
    pub username: String,
}

pub trait Prompts {
    fn interactive(&self) -> bool;
    fn field(&mut self, label: &str, default: Option<&str>, meaning: Meaning) -> Result<String>;
    fn permissions(&mut self, choices: &[Choice], checked: &[String]) -> Result<Vec<String>>;
    fn active(&mut self, choices: &[Choice], selected: Option<&str>) -> Result<String>;
}

pub trait Reports {
    fn separate(&mut self);
    fn heading(&mut self, text: &str) {
        self.heading_parts(&[(text, Meaning::Heading)]);
    }

    fn heading_parts(&mut self, parts: &[(&str, Meaning)]);
    fn value_heading(&mut self, label: &str, value: &str, meaning: Meaning) {
        self.heading_parts(&[
            (label, Meaning::Heading),
            (" [", Meaning::Heading),
            (value, meaning),
            ("]", Meaning::Heading),
        ]);
    }

    fn identity_heading(&mut self, text: &str);
    fn row(&mut self, label: &str, value: &str, meaning: Meaning, nested: bool);
    fn on_off(&mut self, label: &str, enabled: bool) {
        let (value, meaning) = if enabled {
            ("ON", Meaning::Success)
        } else {
            ("OFF", Meaning::Error)
        };
        self.row(label, value, meaning, false);
    }

    fn warning(&mut self, text: &str) {
        self.message(text, Meaning::Warning);
    }

    fn message(&mut self, text: &str, meaning: Meaning) {
        self.message_parts(&[(text, meaning)]);
    }

    fn message_parts(&mut self, parts: &[(&str, Meaning)]);
    fn progress(&mut self, text: &str, finished: bool);
    fn before_native(&mut self);
    fn native_report(&mut self, stdout: &[u8], stderr: &[u8]) -> Result<()>;
}

pub trait Interaction: Prompts + Reports {}
impl<T: Prompts + Reports> Interaction for T {}
#[derive(Default)]
pub struct IdentityFields {
    pub username: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
}

pub struct Invocation {
    config: Option<PathBuf>,
    origin: String,
}

impl Invocation {
    pub fn new(config: Option<PathBuf>, origin: String) -> Self {
        // Provenance is embedded in single-line Git comments, independently of actual path arguments.
        let origin = origin.replace('\r', "\\r").replace('\n', "\\n");
        Self { config, origin }
    }

    fn path(&self) -> Result<PathBuf> {
        identities::config_path(self.config.as_deref())
    }

    fn catalog(&self, missing_ok: bool) -> Result<identities::Catalog> {
        identities::Catalog::load(self.path()?, missing_ok)
    }

    pub fn setup(&self, ui: &mut dyn Interaction) -> Result<()> {
        workflows::setup(self, ui)
    }

    pub fn create_identity(
        &self,
        name: Option<String>,
        fields: IdentityFields,
        ui: &mut dyn Interaction,
    ) -> Result<()> {
        workflows::create(self, name, fields, ui)
    }

    pub fn edit_identity(
        &self,
        name: &str,
        fields: IdentityFields,
        ui: &mut dyn Interaction,
    ) -> Result<()> {
        workflows::edit(self, name, fields, ui)
    }

    pub fn remove_identity(&self, name: &str, ui: &mut dyn Interaction) -> Result<()> {
        workflows::remove(self, name, ui)
    }

    pub fn list_identities(&self, ui: &mut dyn Interaction) -> Result<()> {
        workflows::list(self, ui)
    }

    pub fn switch_identity(&self, name: &str, ui: &mut dyn Interaction) -> Result<()> {
        workflows::switch(self, name, ui)
    }

    pub fn require_repository(&self) -> Result<RepositoryActions<'_>> {
        Ok(RepositoryActions {
            invocation: self,
            repo: repositories::Repository::require()?,
        })
    }

    pub fn show_preferences(
        &self,
        pref: Option<Preference>,
        ui: &mut dyn Interaction,
    ) -> Result<()> {
        workflows::show_preferences(pref, ui)
    }

    pub fn set_preference(
        &self,
        pref: Preference,
        on: bool,
        ui: &mut dyn Interaction,
    ) -> Result<()> {
        preferences::set(pref, on)?;
        workflows::show_preferences(Some(pref), ui)?;
        if matches!(pref, Preference::Autoswitch) && on {
            self.enter_repository(ui)?;
        }
        Ok(())
    }

    pub fn enter_repository(&self, ui: &mut dyn Interaction) -> Result<()> {
        workflows::enter(self, ui)
    }

    pub fn status(&self, full: bool, ui: &mut dyn Interaction) -> Result<()> {
        workflows::status(self, full, ui)
    }

    pub fn welcome(&self, ui: &mut dyn Interaction) -> Result<()> {
        workflows::welcome(self, ui)
    }

    pub fn doctor(&self, ui: &mut dyn Interaction) -> Result<()> {
        workflows::doctor(self, ui)
    }

    pub fn handle_hook(
        &self,
        name: &str,
        args: &[OsString],
        ui: &mut dyn Interaction,
    ) -> Result<()> {
        anyhow::ensure!(
            repositories::HOOK_NAMES.contains(&name),
            "Unknown Git hook: {name}"
        );
        repositories::hook(
            name,
            args,
            &|| self.path(),
            &accounts::AccountQueries,
            &mut || self.enter_repository(ui),
        )
    }

    pub fn shell_initialization(&self, kind: ShellKind) -> Result<String> {
        shell::initialization(self, kind)
    }

    pub fn forward_git(&self, args: &[OsString], ui: &mut dyn Interaction) -> Result<i32> {
        shell::forward(self, args, ui)
    }
}

pub struct RepositoryActions<'a> {
    invocation: &'a Invocation,
    repo: repositories::Repository,
}

impl RepositoryActions<'_> {
    pub fn status(&self, ui: &mut dyn Interaction) -> Result<()> {
        let catalog = self.invocation.catalog(false)?;
        workflows::repo_status(&self.repo, &catalog, ui)
    }

    pub fn check(&self) -> Result<()> {
        self.repo
            .commit_check(&self.invocation.catalog(false)?, &accounts::AccountQueries)
    }

    pub fn protections(&self, state: Option<bool>, ui: &mut dyn Interaction) -> Result<()> {
        match state {
            Some(false) => self.repo.disable()?,
            Some(true) => {
                let config = self.invocation.path()?;
                workflows::with_writes(self.invocation, ui, |writes, _| {
                    self.repo.enable(&config, &self.invocation.origin, writes)?;
                    Ok(())
                })?;
            }
            None => (),
        }
        ui.heading("Repository protections");
        ui.on_off("Protection", self.repo.protections()?);
        Ok(())
    }

    pub fn list_allowed(&self, ui: &mut dyn Interaction) -> Result<()> {
        let stored = self.repo.stored()?;
        let catalog = self.invocation.catalog(false).ok();
        ui.heading("Allowed identities");
        if stored.is_empty() {
            ui.message("none selected", Meaning::Warning);
        } else {
            for name in stored {
                let resolved = catalog
                    .as_ref()
                    .is_some_and(|c| c.find(name.name()).is_some());
                ui.row("Identity", name.name(), Meaning::Identity, false);
                if let Some(identity) = catalog.as_ref().and_then(|c| c.find(name.name())) {
                    ui.row(
                        "GitHub username",
                        identity.username(),
                        Meaning::Username,
                        true,
                    );
                }
                if !resolved {
                    ui.warning("Unresolved permission; remove it or update permissions.");
                }
            }
        }
        Ok(())
    }

    pub fn add_allowed(&self, name: &str, ui: &mut dyn Interaction) -> Result<()> {
        let catalog = self.invocation.catalog(false)?;
        let target = catalog.resolve(&identities::IdentitySelector::new(name))?;
        let mut names = self
            .repo
            .stored()?
            .iter()
            .map(|s| s.name().to_string())
            .collect::<Vec<_>>();
        names.push(target.name().into());
        let selection = repositories::PermissionSelection::resolve(&catalog, &names)?;
        let selected = accounts::selected()?;
        self.repo.authorize(selection, selected.as_ref())?;
        self.list_allowed(ui)
    }

    pub fn remove_allowed(&self, name: &str, ui: &mut dyn Interaction) -> Result<()> {
        let catalog = self.invocation.catalog(false).ok();
        let selected = accounts::selected()?;
        self.repo
            .revoke(name, catalog.as_ref(), selected.as_ref())?;
        self.list_allowed(ui)
    }

    pub fn update_allowed(&self, ui: &mut dyn Interaction) -> Result<()> {
        let catalog = self.invocation.catalog(false)?;
        workflows::pick_permissions(&self.repo, &catalog, ui)?;
        self.list_allowed(ui)
    }
}

fn recovery(path: &Path) -> String {
    let option = format!("--config {}", platform::quote(&path.to_string_lossy()));
    format!(
        "Identity saved in {}\nRun: mgh {option} setup\nRun: mgh {option} switch <identity>",
        path.display()
    )
}
