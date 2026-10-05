use std::io::{self, IsTerminal};

pub fn interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal() && io::stderr().is_terminal()
}
