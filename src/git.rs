pub mod global;
pub mod identity_files;

use crate::process;
use anyhow::Result;
use std::collections::BTreeMap;

pub fn run(args: &[&str]) -> Result<String> {
    process::run("git", args)
}

pub fn optional_config(args: &[&str]) -> Result<Option<String>> {
    process::optional("git", args, &[1])
}

pub fn entries(scope: Option<&str>, pattern: &str) -> Result<BTreeMap<String, Vec<String>>> {
    let mut args = vec!["config"];

    if let Some(scope) = scope {
        args.push(scope);
    }

    args.extend(["--null", "--get-regexp", pattern]);

    Ok(parse_entries(
        optional_config(&args)?.as_deref().unwrap_or_default(),
    ))
}

pub(crate) fn parse_entries(source: &str) -> BTreeMap<String, Vec<String>> {
    let mut values: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for entry in source.split('\0') {
        if let Some((key, value)) = entry.split_once('\n') {
            values
                .entry(key.to_owned())
                .or_default()
                .push(value.to_owned());
        }
    }

    values
}
