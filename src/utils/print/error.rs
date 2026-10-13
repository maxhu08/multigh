use super::padding;
use std::{
    fmt::Display,
    io::{self, Write},
};

pub fn print(message: impl Display) {
    let _ = padding::print(&mut io::stderr().lock(), |output| {
        writeln!(output, "error: {message}")
    });
}
