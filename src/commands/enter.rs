use crate::{config::Config, hooks, output, policy, settings};
use anyhow::Result;
use std::path::PathBuf;

pub fn run(path: PathBuf) -> Result<()> {
    let protections = settings::enabled("protections")?;
    let verbose = settings::enabled("verbose")?;

    if (!protections && !verbose) || !policy::repository()? {
        return Ok(());
    }

    let config = Config::load(path)?;
    let allowed = policy::allowed(&config)?;
    let mut chosen = false;

    let protected = protections && (hooks::installed()? || hooks::integrate()?);

    if protections && !protected {
        for problem in hooks::problems()? {
            output::warning(&problem);
        }

        output::warning(
            "Account protections are not active. Run mgh setup to repair shared hooks; custom global or command overrides need manual integration.",
        );
    }

    if protections && allowed.is_empty() {
        let pending = if protected {
            "No accounts selected; commits and pushes remain blocked."
        } else {
            "No accounts selected; repair hooks and choose accounts before committing or pushing."
        };

        if policy::interactive() {
            match policy::choose(&config) {
                Ok(()) => chosen = true,
                Err(error) => output::warning(&format!("{error}\n\n  {pending}")),
            }
        } else {
            output::warning(&format!(
                "{pending}\nEnter this repository in an interactive terminal, or run mgh protections --allow <accounts>."
            ));
        }
    }

    if verbose {
        let detected = hooks::detected()?;

        if !detected.is_empty() {
            output::section("Existing hooks detected");

            if protected {
                println!(
                    "  {}",
                    output::paint(
                        "Account protections are active; your existing checks are preserved.",
                        output::Color::Changed,
                        false
                    )
                );
            }
            println!();
            for (name, path, active) in detected {
                output::row(
                    &name,
                    &format!(
                        "{}{}",
                        path.display(),
                        if active {
                            ""
                        } else {
                            " (not executable; skipped)"
                        }
                    ),
                    if active {
                        output::Color::Value
                    } else {
                        output::Color::Warning
                    },
                );
            }
        }

        if !chosen {
            policy::show(&config)?;
        }
    }

    Ok(())
}
