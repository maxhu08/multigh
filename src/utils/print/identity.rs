use crate::utils::config::Identity;
use std::io::{self, Write};

pub fn print(output: &mut impl Write, name: &str, identity: &Identity) -> io::Result<()> {
    writeln!(output, "  {name}")?;
    writeln!(output, "    GitHub username: {}", identity.username)?;
    writeln!(output, "    Commit name: {}", identity.commit.name)?;
    writeln!(output, "    Commit email: {}", identity.commit.email)?;
    writeln!(output)
}
