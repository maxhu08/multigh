use std::{
    env,
    io::{self, IsTerminal},
};

pub enum Color {
    Identity,
    Label,
    Value,
}

pub fn paint(text: &str, color: Color) -> String {
    if !io::stdout().is_terminal() || env::var_os("NO_COLOR").is_some() {
        return text.to_owned();
    }

    let code = match color {
        Color::Identity => "1;36",
        Color::Label => "2",
        Color::Value => "32",
    };

    format!("\x1b[{code}m{text}\x1b[0m")
}
