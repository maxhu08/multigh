use super::color::{self, Color};
use crate::utils::config::Identity;
use std::io::{self, Write};

pub fn print(output: &mut impl Write, name: &str, identity: &Identity) -> io::Result<()> {
    writeln!(output, "  {}", color::paint(name, Color::Identity))?;

    for (label, value) in [
        ("GitHub username", &identity.username),
        ("Commit name", &identity.commit.name),
        ("Commit email", &identity.commit.email),
    ] {
        writeln!(
            output,
            "    {}: {}",
            color::paint(label, Color::Label),
            color::paint(value, Color::Value)
        )?;
    }

    Ok(())
}
