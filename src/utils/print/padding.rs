use std::io::{self, Write};

pub fn print<W: Write>(
    output: &mut W,
    render: impl FnOnce(&mut W) -> io::Result<()>,
) -> io::Result<()> {
    writeln!(output)?;
    render(output)?;
    writeln!(output)
}
