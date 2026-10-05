pub mod install;
pub mod preserved;
pub mod readiness;

use crate::settings;
use anyhow::Result;
use std::path::PathBuf;

const MARKER: &str = "# Managed by mgh; existing hooks are preserved.";

pub(crate) const NAMES: &[&str] = &[
    "applypatch-msg",
    "pre-applypatch",
    "post-applypatch",
    "pre-commit",
    "pre-merge-commit",
    "prepare-commit-msg",
    "commit-msg",
    "post-commit",
    "pre-rebase",
    "pre-push",
    "post-checkout",
    "post-merge",
    "pre-receive",
    "update",
    "proc-receive",
    "post-receive",
    "post-update",
    "reference-transaction",
    "push-to-checkout",
    "pre-auto-gc",
    "post-rewrite",
    "sendemail-validate",
    "fsmonitor-watchman",
];

pub fn directory() -> Result<PathBuf> {
    Ok(settings::directory()?.join("hooks"))
}
