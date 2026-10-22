//! Black-box regression tests for the mgh executable.
//! Each case documents the behavior and state-preservation rule it checks.
#![cfg(unix)]

mod support;

#[path = "cases/cli.rs"]
mod cli;

#[path = "cases/configuration.rs"]
mod configuration;

#[path = "cases/identities.rs"]
mod identities;

#[path = "cases/repositories.rs"]
mod repositories;

#[path = "cases/entry.rs"]
mod entry;

#[path = "cases/hooks.rs"]
mod hooks;

#[path = "cases/push.rs"]
mod push;

#[path = "cases/shell.rs"]
mod shell;

#[path = "cases/presentation.rs"]
mod presentation;
