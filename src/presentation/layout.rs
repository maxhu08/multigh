use std::io::{self, Write};

pub(super) const SECTION: usize = 2;
pub(super) const BODY: usize = 4;
pub(super) const NESTED: usize = 6;
pub(super) const VALUE: usize = 30;

/// Add one outer indentation level without changing existing whitespace or ANSI bytes.
pub(super) fn write_indented(
    stream: &mut dyn Write,
    bytes: &[u8],
    columns: usize,
) -> io::Result<()> {
    let prefix = vec![b' '; columns];
    for line in bytes.split_inclusive(|byte| *byte == b'\n') {
        if line != b"\n" && line != b"\r\n" {
            stream.write_all(&prefix)?;
        }
        stream.write_all(line)?;
    }
    Ok(())
}

pub(super) fn indent(text: &str, columns: usize) -> String {
    let mut bytes = Vec::new();
    write_indented(&mut bytes, text.as_bytes(), columns).expect("Writing to a vector cannot fail");
    String::from_utf8(bytes).expect("Adding spaces preserves UTF-8")
}
