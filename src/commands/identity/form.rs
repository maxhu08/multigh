use crate::terminal;
use anyhow::{Context, Result};

pub(super) fn field(value: Option<String>, label: &str, default: Option<&str>) -> Result<String> {
    if let Some(value) = value {
        return Ok(value.trim().to_owned());
    }

    if !terminal::interactive() {
        return default.map(str::to_owned).with_context(|| {
            format!("{label} requires a terminal; supply the identity name, --username and --email")
        });
    }

    let message = match default {
        Some(default) => format!("{label} (default: {default}): "),
        None => format!("{label}: "),
    };
    let mut prompt = cliclack::input(message).required(false);

    if let Some(default) = default {
        prompt = prompt.default_input(default);
    }

    let answer: String = prompt.interact()?;
    let answer = answer.trim();

    Ok(if answer.is_empty() {
        default.unwrap_or(answer)
    } else {
        answer
    }
    .to_owned())
}
