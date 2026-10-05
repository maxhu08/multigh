use anyhow::{Context, Result, ensure};
use std::process::{Command, Output};

pub fn capture(program: &str, args: &[&str]) -> Result<Output> {
    capture_with_env(program, args, &[])
}

pub fn capture_with_env(program: &str, args: &[&str], env: &[(&str, &str)]) -> Result<Output> {
    Command::new(program)
        .args(args)
        .envs(env.iter().copied())
        .output()
        .with_context(|| format!("Could not run {program}"))
}

pub fn run(program: &str, args: &[&str]) -> Result<String> {
    let result = capture(program, args)?;

    successful_output(program, result)
}

pub fn successful_output(program: &str, result: Output) -> Result<String> {
    ensure!(
        result.status.success(),
        "{program}: {}",
        String::from_utf8_lossy(&result.stderr).trim()
    );

    Ok(String::from_utf8(result.stdout)?
        .trim_end_matches('\n')
        .to_owned())
}

pub fn optional(program: &str, args: &[&str], missing_codes: &[i32]) -> Result<Option<String>> {
    let result = capture(program, args)?;

    if result
        .status
        .code()
        .is_some_and(|code| missing_codes.contains(&code))
    {
        return Ok(None);
    }

    successful_output(program, result).map(Some)
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

#[cfg(all(test, unix))]
mod tests {
    use super::optional;

    #[test]
    fn optional_distinguishes_values_absence_and_unexpected_failure() {
        assert_eq!(
            optional("sh", &["-c", "printf 'value\\n'"], &[1]).unwrap(),
            Some("value".into())
        );
        assert_eq!(optional("sh", &["-c", "exit 1"], &[1]).unwrap(), None);

        let error = optional(
            "sh",
            &["-c", "printf 'cannot read config' >&2; exit 2"],
            &[1],
        )
        .unwrap_err();
        assert!(error.to_string().contains("cannot read config"));
    }
}
