use std::{
    env,
    io::{self, IsTerminal},
};

pub enum Color {
    Heading,
    Value,
    Changed,
    Warning,
    Muted,
    Error,
}

pub fn paint(value: &str, color: Color, error: bool) -> String {
    let terminal = if error {
        io::stderr().is_terminal()
    } else {
        io::stdout().is_terminal()
    };

    if !terminal
        || env::var_os("NO_COLOR").is_some()
        || env::var("TERM").is_ok_and(|term| term == "dumb")
    {
        return value.to_owned();
    }

    let code = match color {
        Color::Heading => "1;38;2;192;132;252",
        Color::Value => "1;38;2;103;232;249",
        Color::Changed => "1;38;2;134;239;172",
        Color::Warning => "1;38;2;251;191;36",
        Color::Muted => "38;2;148;163;184",
        Color::Error => "1;38;2;251;113;133",
    };

    format!("\x1b[{code}m{value}\x1b[0m")
}

pub fn section(title: &str) {
    println!("\n  {}", paint(title, Color::Heading, false));
}

pub fn row(label: &str, value: &str, color: Color) {
    println!(
        "  {}{}",
        paint(&format!("{label:<17}"), Color::Muted, false),
        paint(value, color, false)
    );
}

pub fn change(label: &str, before: &str, after: &str) {
    if before == after {
        row(label, after, Color::Value);
    } else {
        println!(
            "  {}{} → {}",
            paint(&format!("{label:<17}"), Color::Muted, false),
            paint(
                if before.is_empty() {
                    "not configured"
                } else {
                    before
                },
                Color::Muted,
                false
            ),
            paint(after, Color::Changed, false)
        );
    }
}

pub fn warning(message: &str) {
    println!(
        "\n  {}",
        paint(&format!("⚠ {message}"), Color::Warning, false)
    );
}

pub fn error(message: &str) {
    eprintln!(
        "\n  {}\n  {}\n",
        paint("✕ mgh", Color::Error, true),
        message.replace('\n', "\n\n  ")
    );
}
