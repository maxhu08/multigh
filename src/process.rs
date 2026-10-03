use anyhow::{Context, Result, ensure};
use std::process::{Command, Output};

pub fn capture(program: &str, args: &[&str]) -> Result<Output> {
    Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Could not run {program}"))
}

pub fn run(program: &str, args: &[&str]) -> Result<String> {
    let result = capture(program, args)?;

    ensure!(
        result.status.success(),
        "{program}: {}",
        String::from_utf8_lossy(&result.stderr).trim()
    );

    Ok(String::from_utf8(result.stdout)?
        .trim_end_matches('\n')
        .to_owned())
}

pub fn optional(program: &str, args: &[&str]) -> Result<Option<String>> {
    let result = capture(program, args)?;

    Ok(result.status.success().then(|| {
        String::from_utf8_lossy(&result.stdout)
            .trim_end_matches('\n')
            .to_owned()
    }))
}

pub fn interactive(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("Could not run {program}"))?;

    ensure!(status.success(), "{program}: command failed ({status})");

    Ok(())
}

pub fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
