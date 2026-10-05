use crate::{config::Config, hooks, output, policy, repository::Repository, settings, terminal};
use anyhow::Result;
use std::path::PathBuf;

pub fn run(path: PathBuf) -> Result<()> {
    let Some(repository) = Repository::discover()? else {
        return Ok(());
    };

    enter(&repository, path)
}

pub(super) fn enter(repository: &Repository, path: PathBuf) -> Result<()> {
    let protections_enabled = settings::protections(repository)?;
    let verbose = settings::enabled(settings::Preference::Verbose)?;
    let autoswitch = settings::enabled(settings::Preference::Autoswitch)?;

    if !protections_enabled && !verbose && !autoswitch {
        return Ok(());
    }

    let config = Config::load(path)?;
    let allowed = policy::allowed(repository, &config)?;
    policy::migrate(repository)?;
    let protections_active = protections_enabled
        && (hooks::readiness::installed(repository)? || hooks::install::integrate(repository)?);
    let mut chosen = false;

    if protections_enabled && !protections_active {
        for problem in hooks::readiness::problems(repository)? {
            output::warning(&problem);
        }

        output::warning(
            "Identity protections are not active. Run mgh setup to repair shared hooks; custom global or command overrides need manual integration.",
        );
    }

    if protections_enabled && allowed.is_empty() {
        let pending = if protections_active {
            "No identities selected; commits and pushes remain blocked."
        } else {
            "No identities selected; repair hooks and choose identities before committing or pushing."
        };

        if terminal::interactive() {
            match super::repo::selection::choose(repository, &config) {
                Ok(()) => chosen = true,
                Err(error) => output::warning(&format!("{error}\n\n{pending}")),
            }
        } else {
            output::warning(&format!(
                "{pending}\nEnter this repository in an interactive terminal, or run mgh repo allowed add <identity>."
            ));
        }
    }

    let selection_handled = if autoswitch {
        match super::settings::autoswitch::select(repository, &config) {
            Ok(handled) => handled,
            Err(error) => {
                output::warning(&format!("{error:#}"));
                false
            }
        }
    } else {
        false
    };

    if verbose && !selection_handled {
        let detected = hooks::preserved::detected(repository)?;

        if !detected.is_empty() {
            output::section("Existing hooks detected");

            if protections_active {
                println!(
                    "  {}",
                    output::paint(
                        "Identity protections are active; your existing checks are preserved.",
                        output::Color::Changed,
                        false
                    )
                );
            }
            println!();
            for hook in detected {
                output::row(
                    &hook.name,
                    &format!(
                        "{}{}",
                        hook.path.display(),
                        if hook.executable {
                            ""
                        } else {
                            " (not executable; skipped)"
                        }
                    ),
                    if hook.executable {
                        output::Color::Value
                    } else {
                        output::Color::Warning
                    },
                );
            }
        }

        if !chosen {
            super::repo::selection::show(repository, &config)?;
        }
    }

    Ok(())
}
