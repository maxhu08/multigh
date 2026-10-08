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

#[derive(Clone, Copy)]
pub enum Stream {
    Stdout,
    Stderr,
}

pub fn color_enabled(stream: Stream) -> bool {
    let terminal = match stream {
        Stream::Stdout => io::stdout().is_terminal(),
        Stream::Stderr => io::stderr().is_terminal(),
    };

    terminal
        && env::var_os("NO_COLOR").is_none()
        && !env::var("TERM").is_ok_and(|term| term == "dumb")
}

pub fn paint(value: &str, color: Color, stream: Stream) -> String {
    if !color_enabled(stream) {
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
    println!("\n  {}", paint(title, Color::Heading, Stream::Stdout));
}

fn columns(label: &str, value: &str, color: Color, indent: usize) -> String {
    const VALUE_COLUMN: usize = 30;

    let padding = VALUE_COLUMN.saturating_sub(indent + label.chars().count());
    let gap = if padding == 0 {
        format!("\n{:VALUE_COLUMN$}", "")
    } else {
        " ".repeat(padding)
    };

    format!(
        "{:indent$}{}{gap}{}",
        "",
        paint(label, color, Stream::Stdout),
        value.replace('\n', &format!("\n{:VALUE_COLUMN$}", ""))
    )
}

pub fn heading(label: &str, value: &str, color: Color) {
    println!(
        "\n{}",
        columns(
            label,
            &paint(value, color, Stream::Stdout),
            Color::Heading,
            2
        )
    );
}

pub fn form_heading(label: &str, value: &str) -> String {
    columns(label, value, Color::Heading, 3)
        .trim_start()
        .to_owned()
}

pub fn row(label: &str, value: &str, color: Color) {
    nested_row(label, value, color, 2);
}

pub fn nested_row(label: &str, value: &str, color: Color, indent: usize) {
    println!(
        "{}",
        columns(
            label,
            &paint(value, color, Stream::Stdout),
            Color::Muted,
            indent
        )
    );
}

pub fn change(label: &str, before: &str, after: &str) {
    if before == after {
        row(label, after, Color::Value);
    } else {
        let value = format!(
            "{} → {}",
            paint(
                if before.is_empty() {
                    "not configured"
                } else {
                    before
                },
                Color::Muted,
                Stream::Stdout
            ),
            paint(after, Color::Changed, Stream::Stdout)
        );
        println!("{}", columns(label, &value, Color::Muted, 2));
    }
}

pub fn warning(message: &str) {
    println!(
        "\n  {}",
        paint(
            &format!("⚠ {}", message.replace('\n', "\n  ")),
            Color::Warning,
            Stream::Stdout
        )
    );
}

pub fn error(message: &str) {
    eprintln!(
        "\n  {}\n  {}\n",
        paint("✕ mgh", Color::Error, Stream::Stderr),
        message.replace('\n', "\n\n  ")
    );
}

pub fn block(value: &str, stream: Stream) {
    for line in value.lines() {
        match stream {
            Stream::Stdout => println!("  {line}"),
            Stream::Stderr => eprintln!("  {line}"),
        }
    }
}
