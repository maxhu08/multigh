use super::{
    identities::{Catalog, EditSession, Identity, IdentitySelector},
    repositories::{Repository, Writes},
    *,
};
use anyhow::{Context, bail, ensure};

enum WriteReport {
    Full,
    Identity,
}

fn report_global_writes(
    inv: &Invocation,
    writes: &Writes,
    ui: &mut dyn Interaction,
    successful_noop: bool,
    identity_reported: bool,
) {
    let mut changes = writes
        .completed
        .iter()
        .filter(|change| {
            !identity_reported || !matches!(change.key.as_str(), "user.name" | "user.email")
        })
        .peekable();
    if changes.peek().is_some() {
        ui.heading(&format!("Global Git config updated by {}", inv.origin));
        for change in changes {
            let value = changed(
                change.before.as_deref(),
                change.after.as_deref().unwrap_or("removed"),
            );
            let meaning = if change.key == "user.name" || change.key == "user.email" {
                Meaning::Commit
            } else {
                Meaning::Path
            };
            ui.row(&change.key, &value, meaning, false);
        }
    } else if successful_noop && !identity_reported {
        ui.separate();
        ui.message("Global Git config unchanged", Meaning::Path);
    }
}

pub(super) fn with_writes<T>(
    inv: &Invocation,
    ui: &mut dyn Interaction,
    operation: impl FnOnce(&mut Writes, &mut dyn Interaction) -> Result<T>,
) -> Result<T> {
    with_write_report(inv, ui, WriteReport::Full, operation)
}

fn with_write_report<T>(
    inv: &Invocation,
    ui: &mut dyn Interaction,
    report: WriteReport,
    operation: impl FnOnce(&mut Writes, &mut dyn Interaction) -> Result<T>,
) -> Result<T> {
    let mut writes = repositories::new_writes();
    let result = operation(&mut writes, ui);
    report_global_writes(
        inv,
        &writes,
        ui,
        result.is_ok(),
        result.is_ok() && matches!(report, WriteReport::Identity),
    );
    result
}

fn collect_new(
    inv: &Invocation,
    name: Option<String>,
    fields: IdentityFields,
    ui: &mut dyn Interaction,
) -> Result<(Catalog, String)> {
    let session = EditSession::open(inv.path()?)?;
    let name = field_or_prompt(
        name,
        "Identity name",
        None,
        "identity argument",
        Meaning::Identity,
        ui,
    )?;
    let username = field_or_prompt(
        fields.username,
        "GitHub username",
        None,
        "--username",
        Meaning::Username,
        ui,
    )?;
    let commit_name = if let Some(name) = fields.name {
        name
    } else if ui.interactive() {
        ui.field("Commit name", Some(username.trim()), Meaning::Commit)?
    } else {
        username.trim().to_string()
    };
    let email = field_or_prompt(
        fields.email,
        "Commit email",
        None,
        "--email",
        Meaning::Commit,
        ui,
    )?;
    let pending = session.create(&name, &username, &email, &commit_name)?;
    let identity = pending.catalog().resolve(&IdentitySelector::new(&name))?;
    ui.before_native();
    accounts::ensure_login(identity.username())?;
    let catalog = pending.save(true)?;
    ui.value_heading("Identity created", &name, Meaning::Identity);
    ui.message_parts(&[
        ("Identity saved in ", Meaning::Success),
        (&catalog.path().display().to_string(), Meaning::Path),
    ]);
    Ok((catalog, name.to_ascii_lowercase()))
}

fn field_or_prompt(
    value: Option<String>,
    label: &str,
    default: Option<&str>,
    flag: &str,
    meaning: Meaning,
    ui: &mut dyn Interaction,
) -> Result<String> {
    if let Some(value) = value {
        return Ok(value);
    }
    ensure!(
        ui.interactive(),
        "{label} is required; provide {flag}. Available flags: --username --email --name"
    );
    ui.field(label, default, meaning)
}

pub(super) fn setup(inv: &Invocation, ui: &mut dyn Interaction) -> Result<()> {
    let initial = inv.catalog(true)?;
    let (catalog, created) = if initial.is_empty() {
        ensure!(
            ui.interactive(),
            "Setup needs a first identity in {}.\n\
             Run: mgh identity new --username <username> --email <email> <identity>",
            initial.path().display()
        );
        let (catalog, name) = collect_new(inv, None, IdentityFields::default(), ui)?;
        (catalog, Some(name))
    } else {
        (initial, None)
    };
    let report = if created.is_some() {
        WriteReport::Identity
    } else {
        WriteReport::Full
    };
    let result = with_write_report(inv, ui, report, |writes, ui| {
        install(inv, &catalog, ui, writes)?;
        if let Some(name) = &created {
            switch_inner(inv, name, SwitchKind::Explicit, ui, writes)?;
        }
        Ok(())
    });
    if created.is_some() {
        result.with_context(|| recovery(catalog.path()))
    } else {
        result
    }
}

fn install(
    inv: &Invocation,
    catalog: &Catalog,
    ui: &mut dyn Interaction,
    writes: &mut Writes,
) -> Result<()> {
    let repo = Repository::discover()?;
    repositories::preflight_hooks()?;
    repositories::refresh(catalog, &inv.origin, writes)?;
    let readiness = repositories::install(repo.as_ref(), catalog.path(), &inv.origin, writes)?;
    preferences::finish_setup()?;
    ui.heading("multigh setup");
    ui.row(
        "Identity config",
        &catalog.path().display().to_string(),
        Meaning::Path,
        false,
    );
    for i in catalog.all() {
        ui.row(
            "Identity file",
            &repositories::state_identity(i)?.display().to_string(),
            Meaning::Path,
            false,
        );
    }
    ui.row(
        "Global Git config",
        &repositories::global_path(writes)?,
        Meaning::Path,
        false,
    );
    preference_rows(None, ui)?;
    show_readiness(
        &readiness,
        repo.as_ref()
            .map(Repository::protections)
            .transpose()?
            .unwrap_or(true),
        ui,
    );
    setup_guidance(ui);
    Ok(())
}

fn setup_guidance(ui: &mut dyn Interaction) {
    ui.heading("Shell integration");
    ui.message(
        "Add the matching command to your shell's configuration file.\n\
         This enables repository entry checks and tab completion.",
        Meaning::Plain,
    );

    for (shell, config, command) in [
        (
            "Fish config",
            "~/.config/fish/config.fish",
            "mgh shell init fish | source",
        ),
        (
            "Bash config",
            "~/.bashrc",
            "eval \"$(mgh shell init bash)\"",
        ),
        ("Zsh config", "~/.zshrc", "eval \"$(mgh shell init zsh)\""),
    ] {
        ui.separate();
        ui.row(shell, config, Meaning::Path, false);
        ui.row("Add this line", command, Meaning::Plain, true);
    }

    ui.separate();
    ui.message(
        "After saving, restart your shell or run its command above to enable it now.",
        Meaning::Plain,
    );

    ui.heading("Next steps");
    ui.message(
        "Add more identities: mgh identity new\n\
         Inside a repository, choose which identities may commit and push.\n\
         Choose permissions in a repo: mgh repo allowed update",
        Meaning::Plain,
    );
}

pub(super) fn create(
    inv: &Invocation,
    name: Option<String>,
    fields: IdentityFields,
    ui: &mut dyn Interaction,
) -> Result<()> {
    let (catalog, name) = collect_new(inv, name, fields, ui)?;
    with_write_report(inv, ui, WriteReport::Identity, |writes, ui| {
        install(inv, &catalog, ui, writes)?;
        switch_inner(inv, &name, SwitchKind::Explicit, ui, writes)
    })
    .with_context(|| recovery(catalog.path()))
}

pub(super) fn edit(
    inv: &Invocation,
    name: &str,
    fields: IdentityFields,
    ui: &mut dyn Interaction,
) -> Result<()> {
    let session = EditSession::open(inv.path()?)?;
    let old = session
        .catalog()
        .resolve(&IdentitySelector::new(name))?
        .clone();
    let fields = if fields.username.is_none() && fields.email.is_none() && fields.name.is_none() {
        ensure!(
            ui.interactive(),
            "Identity editing needs a terminal or --username, --email or --name flags"
        );
        IdentityFields {
            username: Some(ui.field("GitHub username", Some(old.username()), Meaning::Username)?),
            email: Some(ui.field("Commit email", Some(old.email()), Meaning::Commit)?),
            name: Some(ui.field("Commit name", Some(old.commit_name()), Meaning::Commit)?),
        }
    } else {
        fields
    };
    let pending = session.edit(
        name,
        fields.username.as_deref(),
        fields.email.as_deref(),
        fields.name.as_deref(),
    )?;
    let identity = pending.catalog().resolve(&IdentitySelector::new(name))?;
    ui.before_native();
    accounts::ensure_login(identity.username())?;
    let catalog = pending.save(true)?;
    ui.value_heading("Identity updated", name, Meaning::Identity);
    ui.message_parts(&[
        ("Identity saved in ", Meaning::Success),
        (&catalog.path().display().to_string(), Meaning::Path),
    ]);
    with_writes(inv, ui, |writes, _| {
        repositories::refresh(&catalog, &inv.origin, writes)
    })
    .with_context(|| recovery(catalog.path()))
}

pub(super) fn remove(inv: &Invocation, name: &str, ui: &mut dyn Interaction) -> Result<()> {
    let pending = EditSession::open(inv.path()?)?.remove(name)?;
    let catalog = pending.save(false)?;
    ui.heading("Identity removed");
    ui.message_parts(&[
        ("Removed identity ", Meaning::Success),
        (name, Meaning::Identity),
        (
            ". Authentication and repository permissions were preserved.",
            Meaning::Success,
        ),
    ]);
    ui.warning("Stale repository permissions remain blocked until explicitly updated.");
    with_writes(inv, ui, |writes, _| {
        repositories::refresh(&catalog, &inv.origin, writes)
    })
    .with_context(|| recovery(catalog.path()))
}

fn report_identity(
    identity: &Identity,
    accounts: Option<&[accounts::LiveAccount]>,
    ui: &mut dyn Interaction,
) -> Result<()> {
    let account = accounts
        .unwrap_or_default()
        .iter()
        .find(|a| a.login.eq_ignore_ascii_case(identity.username()));
    let suffix = match (accounts, account) {
        (Some(_), Some(a)) if a.active => " (Active)",
        (Some(_), Some(a)) if a.state == "success" => "",
        (None, _) => " (Accounts unavailable)",
        _ => " (Not signed in)",
    };
    ui.identity_heading(&format!("{}{}", identity.name(), suffix));
    ui.row(
        "GitHub username",
        identity.username(),
        Meaning::Username,
        true,
    );
    ui.row("Commit name", identity.commit_name(), Meaning::Commit, true);
    ui.row("Commit email", identity.email(), Meaning::Commit, true);
    ui.row(
        "Identity file",
        &repositories::state_identity(identity)?
            .display()
            .to_string(),
        Meaning::Path,
        true,
    );
    Ok(())
}

pub(super) fn list(inv: &Invocation, ui: &mut dyn Interaction) -> Result<()> {
    let catalog = inv.catalog(false)?;
    let accounts = match accounts::all() {
        Ok(a) => Some(a),
        Err(e) => {
            ui.warning(&format!("GitHub account query unavailable: {e:#}"));
            None
        }
    };
    ui.heading("Identities");
    ui.row(
        "Identity config",
        &catalog.path().display().to_string(),
        Meaning::Path,
        false,
    );
    for identity in catalog.all() {
        report_identity(identity, accounts.as_deref(), ui)?;
    }
    if catalog.is_empty() {
        ui.message(
            "No identities configured. Run: mgh identity new",
            Meaning::Warning,
        );
    }
    if let Some(accounts) = accounts {
        report_other_accounts(Some(&catalog), &accounts, ui);
    }
    Ok(())
}

fn report_other_accounts(
    catalog: Option<&Catalog>,
    accounts: &[accounts::LiveAccount],
    ui: &mut dyn Interaction,
) {
    let mut heading_shown = false;
    for account in accounts.iter().filter(|a| {
        !catalog.is_some_and(|catalog| {
            catalog
                .all()
                .any(|i| i.username().eq_ignore_ascii_case(&a.login))
        })
    }) {
        if !heading_shown {
            ui.heading(if catalog.is_some() {
                "Unconfigured GitHub accounts"
            } else {
                "GitHub accounts"
            });
            heading_shown = true;
        }
        ui.row(
            if catalog.is_some() {
                "Unconfigured account"
            } else {
                "GitHub account"
            },
            &format!(
                "{}{}",
                account.login,
                if account.active { " (Active)" } else { "" }
            ),
            Meaning::Username,
            false,
        );
    }
}

fn report_unresolved_permission(name: &str, ui: &mut dyn Interaction) {
    ui.message_parts(&[
        ("Unresolved permission '", Meaning::Warning),
        (name, Meaning::Identity),
        ("'; remove it or update permissions.", Meaning::Warning),
    ]);
}

fn changed(before: Option<&str>, after: &str) -> String {
    match before {
        Some(b) => format!("{b} → {after}"),
        None => format!("not configured → {after}"),
    }
}

pub(super) fn switch(inv: &Invocation, name: &str, ui: &mut dyn Interaction) -> Result<()> {
    with_write_report(inv, ui, WriteReport::Identity, |writes, ui| {
        switch_inner(inv, name, SwitchKind::Explicit, ui, writes)
    })
}

enum SwitchKind {
    Explicit,
    Automatic,
}

fn selected_identity<'a>(
    catalog: &'a Catalog,
    account: Option<&'a accounts::SelectedAccount>,
) -> &'a str {
    match account {
        Some(account) => match catalog
            .all()
            .find(|identity| identity.username().eq_ignore_ascii_case(account.login()))
        {
            Some(identity) => identity.name(),
            None => account.login(),
        },
        None => "no current identity",
    }
}

fn disallowed_identity_warning(identity: &str, allowed: &[&Identity]) -> String {
    let (identities, command) = if allowed.is_empty() {
        ("none".to_string(), "mgh repo allowed update")
    } else {
        (
            allowed
                .iter()
                .map(|identity| identity.name())
                .collect::<Vec<_>>()
                .join(", "),
            "mgh switch <identity>",
        )
    };
    format!(
        "Protections are on; [{identity}] cannot commit or push in this repository.\n\
         Allowed identities: {identities}\n\
         If unintended, run: {command}"
    )
}

fn switch_inner(
    inv: &Invocation,
    name: &str,
    kind: SwitchKind,
    ui: &mut dyn Interaction,
    writes: &mut Writes,
) -> Result<()> {
    let catalog = inv.catalog(false)?;
    let repo = Repository::discover()?;
    switch_in_repository(inv, &catalog, repo.as_ref(), name, kind, ui, writes)
}

fn switch_in_repository(
    inv: &Invocation,
    catalog: &Catalog,
    repo: Option<&Repository>,
    name: &str,
    kind: SwitchKind,
    ui: &mut dyn Interaction,
    writes: &mut Writes,
) -> Result<()> {
    let target = catalog.resolve(&IdentitySelector::new(name))?;
    let prepared = repo
        .map(|r| r.prepare_selection(catalog, target))
        .transpose()?;
    let switch_warning = if matches!(kind, SwitchKind::Explicit)
        && let Some(repo) = repo
        && repo.protections()?
    {
        let allowed = repo.allowed(catalog)?;
        if allowed
            .iter()
            .any(|identity| identity.name() == target.name())
        {
            None
        } else {
            Some(disallowed_identity_warning(target.name(), &allowed))
        }
    } else {
        None
    };
    let prior = if matches!(kind, SwitchKind::Explicit) {
        accounts::selected().ok().flatten()
    } else {
        None
    };
    let (prior_name, prior_email) = if matches!(kind, SwitchKind::Explicit) {
        repositories::global_defaults()?
    } else {
        (None, None)
    };
    repositories::refresh(catalog, &inv.origin, writes)?;
    accounts::switch(target.username())?;
    repositories::set_global(target, &inv.origin, writes)?;
    if let Some(prepared) = prepared {
        prepared.apply()?;
    }
    if matches!(kind, SwitchKind::Automatic) {
        return Ok(());
    }
    let prior_identity = selected_identity(catalog, prior.as_ref());
    ui.heading_parts(&[
        ("Identity selected [", Meaning::Heading),
        (prior_identity, Meaning::Identity),
        (" → ", Meaning::HeadingArrow),
        (target.name(), Meaning::Identity),
        ("]", Meaning::Heading),
    ]);
    ui.row(
        "GitHub username",
        &changed(prior.as_ref().map(|a| a.login()), target.username()),
        Meaning::Username,
        false,
    );
    ui.row(
        "Commit name",
        &changed(prior_name.as_deref(), target.commit_name()),
        Meaning::Commit,
        false,
    );
    ui.row(
        "Commit email",
        &changed(prior_email.as_deref(), target.email()),
        Meaning::Commit,
        false,
    );
    if let Some(warning) = switch_warning {
        ui.separate();
        ui.warning(&warning);
    }
    Ok(())
}

pub(super) fn show_preferences(pref: Option<Preference>, ui: &mut dyn Interaction) -> Result<()> {
    ui.heading("Global settings");
    preference_rows(pref, ui)
}

fn preference_rows(pref: Option<Preference>, ui: &mut dyn Interaction) -> Result<()> {
    let all = [
        Preference::Autoswitch,
        Preference::Verbose,
        Preference::Welcome,
    ];
    for preference in pref.as_ref().map(std::slice::from_ref).unwrap_or(&all) {
        ui.on_off(preference.name(), preferences::get(*preference)?);
    }
    Ok(())
}

pub(super) fn pick_permissions(
    repo: &Repository,
    catalog: &Catalog,
    ui: &mut dyn Interaction,
) -> Result<()> {
    ensure!(
        ui.interactive(),
        "Choose permissions with mgh repo allowed update interactively, \
         or mgh repo allowed add <identity> for automation"
    );
    let choices = catalog
        .all()
        .map(|i| Choice {
            name: i.name().into(),
            username: i.username().into(),
        })
        .collect::<Vec<_>>();
    ensure!(
        !choices.is_empty(),
        "No identities configured. Run: mgh identity new"
    );
    let checked = repo
        .stored()?
        .iter()
        .map(|s| s.name().into())
        .collect::<Vec<_>>();
    let selected = ui.permissions(&choices, &checked)?;
    let selection = repositories::PermissionSelection::resolve(catalog, &selected)?;
    let account = accounts::selected()?;
    repo.authorize(selection, account.as_ref())
}

fn show_readiness(
    readiness: &repositories::HookReadiness,
    enabled: bool,
    ui: &mut dyn Interaction,
) {
    if !enabled {
        ui.on_off("Protection", false);
    } else if readiness.healthy() {
        ui.row(
            "Protection",
            "Commit + push checks enabled",
            Meaning::Success,
            false,
        );
    } else {
        ui.warning(
            "Protections are not active. Run: mgh setup. \
             Unsupported global/command overrides need manual integration.",
        );
        for p in &readiness.problems {
            ui.warning(p);
        }
    }
}

pub(super) fn repo_report(
    repo: &Repository,
    catalog: Option<&Catalog>,
    config: &Path,
    ui: &mut dyn Interaction,
) -> Result<()> {
    repo_details(repo, catalog, config, ui)?;
    ui.on_off(
        "Autoswitch (global)",
        preferences::get(Preference::Autoswitch)?,
    );
    Ok(())
}

pub(super) fn repo_status(
    repo: &Repository,
    catalog: &Catalog,
    ui: &mut dyn Interaction,
) -> Result<()> {
    repo_report(repo, Some(catalog), catalog.path(), ui)?;
    report_identity_match(repo, catalog, &accounts::AccountQueries::default(), ui);
    Ok(())
}

fn report_identity_match(
    repo: &Repository,
    catalog: &Catalog,
    accounts: &accounts::AccountQueries,
    ui: &mut dyn Interaction,
) {
    ui.separate();
    match repo.commit_check(catalog, accounts) {
        Err(error) => ui.warning(&format!("{error:#}")),
        Ok(()) => ui.message("Identity and commit details match", Meaning::Success),
    }
}

fn repo_details(
    repo: &Repository,
    catalog: Option<&Catalog>,
    config: &Path,
    ui: &mut dyn Interaction,
) -> Result<()> {
    ui.heading("Repository");
    ui.row(
        "Current repository",
        &repo.root().display().to_string(),
        Meaning::Path,
        false,
    );
    ui.row(
        "Repo config",
        &repo.config().display().to_string(),
        Meaning::Path,
        false,
    );
    ui.row(
        "Active identity",
        repo.current()?.as_deref().unwrap_or("not configured"),
        Meaning::Identity,
        false,
    );
    let stored = repo.stored()?;
    ui.row(
        "Allowed identities",
        &if stored.is_empty() {
            "none selected".to_string()
        } else {
            stored
                .iter()
                .map(|s| s.name())
                .collect::<Vec<_>>()
                .join(", ")
        },
        Meaning::Identity,
        false,
    );
    if let Some(catalog) = catalog {
        for p in &stored {
            if catalog.find(p.name()).is_none() {
                report_unresolved_permission(p.name(), ui);
            }
        }
    }
    let (name, email) = repo.effective_details()?;
    ui.row(
        "Commit name",
        name.as_deref().unwrap_or("not configured"),
        Meaning::Commit,
        false,
    );
    ui.row(
        "Commit email",
        email.as_deref().unwrap_or("not configured"),
        Meaning::Commit,
        false,
    );
    let readiness = repo.readiness(config)?;
    let enabled = repo.protections()?;
    show_readiness(&readiness, enabled, ui);
    if enabled && stored.is_empty() {
        ui.warning("No identities selected; commits and pushes remain blocked. Run: mgh repo allowed update");
    }
    for hook in readiness.project_hooks {
        ui.row("Project hook", &hook, Meaning::Path, false);
    }
    Ok(())
}

pub(super) fn enter(inv: &Invocation, ui: &mut dyn Interaction) -> Result<()> {
    let Some(repo) = Repository::discover()? else {
        return Ok(());
    };
    enter_repository(inv, &repo, ui).map(|_| ())
}

fn enter_repository(
    inv: &Invocation,
    repo: &Repository,
    ui: &mut dyn Interaction,
) -> Result<Option<accounts::SelectedAccount>> {
    let enabled = repo.protections()?;
    let auto = preferences::get(Preference::Autoswitch)?;
    let verbose = preferences::get(Preference::Verbose)?;
    if !enabled && !auto && !verbose {
        return Ok(None);
    }
    if auto {
        ui.progress("Autoswitch: checking identity", false);
    }
    let catalog = inv.catalog(false)?;
    let mut allowed = repo.allowed(&catalog)?;
    let mut progress_interrupted = allowed.is_empty();
    if auto && allowed.is_empty() {
        ui.progress("; no allowed identities configured.", true);
    }
    let mut readiness = repo.readiness(catalog.path())?;
    let mut writes = repositories::new_writes();
    if enabled && !readiness.healthy() {
        let failure = match repo.install_hooks(catalog.path(), &inv.origin, &mut writes) {
            Ok(repaired) => {
                readiness = repaired;
                None
            }
            Err(error) => Some(error),
        };
        if !readiness.healthy() {
            progress_interrupted = true;
            ui.heading("Repository integration");
            if let Some(error) = failure {
                ui.warning(&format!("Hook repair failed: {error:#}"));
            }
            show_readiness(&readiness, true, ui);
        }
    }
    let mut selected = false;
    if enabled && allowed.is_empty() {
        if let Err(e) = pick_permissions(repo, &catalog, ui) {
            ui.warning(&format!("{e:#}"));
            ui.warning("No identities selected; commits and pushes remain blocked. Run: mgh repo allowed update");
            if !readiness.healthy() {
                ui.warning("Repair hooks with mgh setup and choose identities before committing or pushing.");
            }
            return Ok(None);
        }
        allowed = repo.allowed(&catalog)?;
        selected = true;
    }
    let mut unchanged_account = None;
    if auto && !allowed.is_empty() {
        selected = true;
        if progress_interrupted {
            ui.progress("Autoswitch: checking identity", false);
        }
        let attempt = (|| {
            let account = accounts::selected()?;
            let current = allowed.iter().find(|i| {
                account
                    .as_ref()
                    .is_some_and(|a| a.login().eq_ignore_ascii_case(i.username()))
            });
            let name = selected_identity(&catalog, account.as_ref());
            if allowed.len() == 1 && current.is_some() {
                ui.progress(&format!("; {name} is allowed, no switch needed."), true);
                unchanged_account = account;
                return Ok(());
            }
            let target = if allowed.len() == 1 {
                let status = if account.is_none() {
                    name.to_string()
                } else {
                    format!("{name} is not allowed")
                };
                ui.progress(
                    &format!("; {status}, switching to {}...", allowed[0].name()),
                    false,
                );
                allowed[0].name().to_string()
            } else {
                let status = if account.is_none() {
                    name.to_string()
                } else if current.is_some() {
                    format!("{name} is allowed")
                } else {
                    format!("{name} is not allowed")
                };
                ui.progress(&format!("; {status}, choose an allowed identity."), true);
                ensure!(
                    ui.interactive(),
                    "Multiple allowed identities require a terminal. Run: mgh switch <allowed-identity>"
                );
                let choices = allowed
                    .iter()
                    .map(|i| Choice {
                        name: i.name().into(),
                        username: i.username().into(),
                    })
                    .collect::<Vec<_>>();
                let current = current.map(|i| i.name());
                let target = ui.active(&choices, current)?;
                ui.progress(&format!("Autoswitch: switching to {target}..."), false);
                target
            };
            ensure!(
                allowed.iter().any(|i| i.name() == target),
                "Selected identity is not allowed"
            );
            switch_in_repository(
                inv,
                &catalog,
                Some(repo),
                &target,
                SwitchKind::Automatic,
                ui,
                &mut writes,
            )?;
            ui.progress(" done.", true);
            Ok(())
        })();
        if let Err(e) = attempt {
            if allowed.len() > 1 {
                ui.message(&format!("Autoswitch failed: {e:#}."), Meaning::Path);
            } else {
                ui.progress(&format!("; autoswitch failed: {e:#}."), true);
            }
        }
    }
    if verbose && !selected {
        repo_report(repo, Some(&catalog), catalog.path(), ui)?;
    }
    Ok(unchanged_account)
}

pub(super) fn status(inv: &Invocation, full: bool, ui: &mut dyn Interaction) -> Result<()> {
    let catalog = match inv.catalog(false) {
        Ok(c) => Some(c),
        Err(e) => {
            ui.warning(&format!("{e:#}"));
            None
        }
    };
    let accounts = accounts::all()?;
    let active = accounts
        .iter()
        .find(|a| a.active)
        .context("No active GitHub account")?;
    ui.heading("Identities");
    ui.row(
        "Identity config",
        &inv.path()?.display().to_string(),
        Meaning::Path,
        false,
    );
    if active.state != "success" {
        ui.warning(&format!(
            "GitHub authentication warning: {}",
            active.error.as_deref().unwrap_or(&active.state)
        ));
    }
    if let Some(catalog) = &catalog {
        for i in catalog.all() {
            report_identity(i, Some(&accounts), ui)?;
        }
    }
    report_other_accounts(catalog.as_ref(), &accounts, ui);
    if let Some(repo) = Repository::discover()? {
        repo_details(&repo, catalog.as_ref(), &inv.path()?, ui)?;
        if let Some(catalog) = &catalog {
            report_identity_match(
                &repo,
                catalog,
                &accounts::AccountQueries::from_accounts(&accounts),
                ui,
            );
        }
    }
    if full {
        let report = accounts::full()?;
        ui.native_report(&report.stdout, &report.stderr)?;
        anyhow::ensure!(
            report.status.success(),
            "GitHub authentication check failed"
        );
    }
    Ok(())
}

pub(super) fn startup(inv: &Invocation, ui: &mut dyn Interaction) -> Result<()> {
    let repo = Repository::discover()?;
    let unchanged_account = match &repo {
        Some(repo) => enter_repository(inv, repo, ui)?,
        None => None,
    };
    if preferences::get(Preference::Welcome)? {
        welcome_in_repository(inv, repo.as_ref(), unchanged_account, ui)?;
    }
    Ok(())
}

pub(super) fn welcome(inv: &Invocation, ui: &mut dyn Interaction) -> Result<()> {
    if !preferences::get(Preference::Welcome)? {
        return Ok(());
    }
    welcome_in_repository(inv, Repository::discover()?.as_ref(), None, ui)
}

fn welcome_in_repository(
    inv: &Invocation,
    repo: Option<&Repository>,
    unchanged_account: Option<accounts::SelectedAccount>,
    ui: &mut dyn Interaction,
) -> Result<()> {
    // Reuse the fresh entry query only when authentication did not change.
    let account = match unchanged_account {
        Some(account) => Some(account),
        None => accounts::selected()?,
    };
    let catalog = match inv.catalog(false) {
        Ok(c) => Some(c),
        Err(e) => {
            ui.warning(&format!("{e:#}"));
            None
        }
    };
    let identity = catalog.as_ref().and_then(|c| {
        c.all().find(|i| {
            account
                .as_ref()
                .is_some_and(|a| a.login().eq_ignore_ascii_case(i.username()))
        })
    });
    let email = if let Some(identity) = identity {
        Some(identity.email().to_string())
    } else if let Some(repo) = &repo {
        repo.effective_details()?.1
    } else {
        repositories::global_defaults()?.1
    };
    ui.heading("multigh");
    ui.row(
        "Identity",
        identity.map(Identity::name).unwrap_or("not configured"),
        Meaning::Identity,
        false,
    );
    ui.row(
        "GitHub username",
        account.as_ref().map(|a| a.login()).unwrap_or("unavailable"),
        Meaning::Username,
        false,
    );
    ui.row(
        "Commit email",
        email.as_deref().unwrap_or("not configured"),
        Meaning::Commit,
        false,
    );
    if let (Some(repo), Some(catalog)) = (&repo, &catalog) {
        let stored = repo.stored()?;
        let mut allowed = vec![];
        for stored in &stored {
            match catalog.find(stored.name()) {
                Some(i) => allowed.push(i),
                None => report_unresolved_permission(stored.name(), ui),
            }
        }

        if repo.protections()?
            && !allowed.iter().any(|i| {
                account
                    .as_ref()
                    .is_some_and(|a| a.login().eq_ignore_ascii_case(i.username()))
            })
        {
            ui.separate();
            ui.warning(&disallowed_identity_warning(
                selected_identity(catalog, account.as_ref()),
                &allowed,
            ));
        } else if let Some(identity) = identity {
            let (name, email) = repo.effective_details()?;
            if name.as_deref() != Some(identity.commit_name())
                || !email.is_some_and(|e| identity.accepts(&e))
            {
                ui.message_parts(&[
                    ("Commit details need: mgh switch ", Meaning::Warning),
                    (identity.name(), Meaning::Identity),
                ]);
            }
        }
    }
    Ok(())
}

pub(super) fn doctor(inv: &Invocation, ui: &mut dyn Interaction) -> Result<()> {
    ui.heading("multigh diagnostics");
    let mut problems = vec![];
    match repositories::git_version() {
        Ok(version) => {
            let number = version.split_whitespace().nth(2).unwrap_or("");
            let nums = number
                .split('.')
                .take(2)
                .map(str::parse::<u32>)
                .collect::<std::result::Result<Vec<_>, _>>();
            match nums {
                Ok(n) if n.len() == 2 && (n[0] > 2 || n[0] == 2 && n[1] >= 45) => {
                    ui.row("Git", version.trim(), Meaning::Plain, false)
                }
                _ => problems.push(format!("Git 2.45+ required: {version}")),
            }
        }
        Err(e) => problems.push(format!("{e:#}")),
    }
    match platform::successful(std::process::Command::new("gh").arg("--version")) {
        Ok(_) => (),
        Err(e) => problems.push(format!("{e:#}")),
    }
    match inv.catalog(false) {
        Ok(c) if !c.is_empty() => (),
        Ok(_) => problems.push("Identity configuration is empty. Run: mgh identity new".into()),
        Err(e) => problems.push(format!("{e:#}")),
    }
    if let Err(e) = accounts::active() {
        problems.push(format!("{e:#}"));
    }
    let repo = match Repository::discover() {
        Ok(r) => r,
        Err(e) => {
            problems.push(format!("{e:#}"));
            None
        }
    };
    match repositories::readiness(repo.as_ref(), inv.path().ok().as_deref()) {
        Ok(r) => problems.extend(r.problems),
        Err(e) => problems.push(format!("{e:#}")),
    }
    if !problems.is_empty() {
        for p in problems {
            ui.warning(&p);
        }
        bail!("Installation checks failed. Repair the reported problems and run mgh setup.");
    }
    ui.message("Installation checks passed", Meaning::Success);
    Ok(())
}
