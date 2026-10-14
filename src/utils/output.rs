use crate::config::{CommitDetails, Identity};
use std::{
    cell::RefCell,
    env,
    io::{self, IsTerminal},
    path::Path,
};

#[derive(Clone, Copy)]
enum Color {
    Heading,
    Identity,
    Username,
    CommitName,
    Email,
    Path,
    Success,
    Warning,
    Muted,
    Error,
}

#[derive(Clone, Copy)]
pub enum Stream {
    Stdout,
    Stderr,
}

#[derive(Clone, Copy, Default)]
enum Padding {
    #[default]
    Unwritten,
    Content,
    Blank,
}

thread_local! {
    static PADDING: RefCell<[Padding; 2]> = RefCell::default();
}

fn stream_index(stream: Stream) -> usize {
    if matches!(stream, Stream::Stdout)
        || (io::stdout().is_terminal() && io::stderr().is_terminal())
    {
        0
    } else {
        1
    }
}

fn line(value: &str, stream: Stream) {
    PADDING.with_borrow_mut(|padding| {
        let state = &mut padding[stream_index(stream)];
        let blank = value.trim().is_empty();
        if blank && matches!(state, Padding::Blank) {
            return;
        }
        let value = if blank { "" } else { value };
        let prefix = if matches!(state, Padding::Unwritten) && !blank {
            "\n"
        } else {
            ""
        };
        match stream {
            Stream::Stdout => println!("{prefix}{value}"),
            Stream::Stderr => eprintln!("{prefix}{value}"),
        }
        *state = if blank {
            Padding::Blank
        } else {
            Padding::Content
        };
    });
}

fn separate(stream: Stream) {
    line("", stream);
}

pub fn finish() {
    for stream in [Stream::Stdout, Stream::Stderr] {
        if PADDING
            .with_borrow(|padding| !matches!(padding[stream_index(stream)], Padding::Unwritten))
        {
            separate(stream);
        }
    }
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

fn paint(value: &str, color: Color, stream: Stream) -> String {
    if !color_enabled(stream) {
        return value.to_owned();
    }

    let code = match color {
        Color::Heading => "1;38;2;244;114;182",
        Color::Identity => "1;38;2;103;232;249",
        Color::Username => "38;2;192;132;252",
        Color::CommitName | Color::Email => "38;2;251;146;60",
        Color::Path | Color::Muted => "38;2;148;163;184",
        Color::Success => "1;38;2;134;239;172",
        Color::Warning => "1;38;2;251;191;36",
        Color::Error => "1;38;2;251;113;133",
    };

    let value = value.replace("\x1b[0m", &format!("\x1b[0m\x1b[{code}m"));
    format!("\x1b[{code}m{value}\x1b[0m")
}

pub fn identity_name(value: &str, stream: Stream) -> String {
    paint(value, Color::Identity, stream)
}

pub fn username(value: &str, stream: Stream) -> String {
    paint(value, Color::Username, stream)
}

#[derive(Clone, Copy)]
pub enum IdentityField {
    Name,
    Username,
    CommitName,
    Email,
}

impl IdentityField {
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Identity name",
            Self::Username => "GitHub username",
            Self::CommitName => "Commit name",
            Self::Email => "Commit email",
        }
    }

    pub fn value(self, value: &str) -> String {
        let color = match self {
            Self::Name => Color::Identity,
            Self::Username => Color::Username,
            Self::CommitName => Color::CommitName,
            Self::Email => Color::Email,
        };
        paint(value, color, Stream::Stderr)
    }
}

pub fn input_theme(field: IdentityField) -> impl cliclack::Theme {
    struct InputTheme(IdentityField);

    impl cliclack::Theme for InputTheme {
        fn format_input(
            &self,
            state: &cliclack::ThemeState,
            cursor: &cliclack::StringCursor,
        ) -> String {
            let input = match state {
                cliclack::ThemeState::Active | cliclack::ThemeState::Error(_) => {
                    self.cursor_with_style(cursor, &self.input_style(&cliclack::ThemeState::Active))
                }
                _ => cursor.to_string(),
            };
            input
                .lines()
                .map(|line| {
                    format!(
                        "{}  {}\n",
                        self.bar_color(state).apply_to("│"),
                        self.0.value(line)
                    )
                })
                .collect()
        }
    }

    InputTheme(field)
}

pub fn section(title: &str) {
    separate(Stream::Stdout);
    text(&paint(title, Color::Heading, Stream::Stdout));
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

fn heading(label: &str, value: &str, color: Color) {
    separate(Stream::Stdout);
    line(
        &columns(
            label,
            &paint(value, color, Stream::Stdout),
            Color::Heading,
            2,
        ),
        Stream::Stdout,
    );
}

pub fn identity_heading(label: &str, name: &str) {
    heading(label, name, Color::Identity);
}

pub fn repository_heading(label: &str, value: &str) {
    heading(label, value, Color::Path);
}

pub fn form_repository(value: &Path) -> String {
    form_heading(
        "Repository",
        &paint(&value.to_string_lossy(), Color::Path, Stream::Stderr),
    )
}

pub fn form_heading(label: &str, value: &str) -> String {
    columns(label, value, Color::Heading, 3)
        .trim_start()
        .to_owned()
}

pub fn form_intro(title: &str) -> io::Result<()> {
    separate(Stream::Stderr);
    cliclack::intro(title)?;
    form_written();
    Ok(())
}

pub fn form_outro(message: &str) -> io::Result<()> {
    cliclack::outro(message)?;
    form_written();
    separate(Stream::Stderr);
    Ok(())
}

fn form_written() {
    PADDING.with_borrow_mut(|padding| {
        padding[stream_index(Stream::Stderr)] = Padding::Content;
    });
}

fn row(label: &str, value: &str, color: Color) {
    nested_row(label, value, color, 2);
}

fn nested_row(label: &str, value: &str, color: Color, indent: usize) {
    line(
        &columns(
            label,
            &paint(value, color, Stream::Stdout),
            Color::Muted,
            indent,
        ),
        Stream::Stdout,
    );
}

pub fn identity_value(label: &str, name: &str) {
    row(label, name, Color::Identity);
}

pub fn github_username(label: &str, value: &str) {
    row(label, value, Color::Username);
}

pub fn commit_email(value: &str) {
    row("Commit email", value, Color::Email);
}

pub fn path(label: &str, value: &Path) {
    row(label, &value.to_string_lossy(), Color::Path);
}

pub fn path_section(label: &str, value: &Path) {
    section(label);
    text(&paint(
        &value.to_string_lossy(),
        Color::Path,
        Stream::Stdout,
    ));
}

pub fn identity_file(name: &str, value: &Path) {
    let label = format!("Identity ({name})");
    let rendered = columns(
        &label,
        &paint(&value.to_string_lossy(), Color::Path, Stream::Stdout),
        Color::Muted,
        2,
    );
    let painted_label = paint(&label, Color::Muted, Stream::Stdout);
    let styled_label = paint(
        &format!("Identity ({})", identity_name(name, Stream::Stdout)),
        Color::Muted,
        Stream::Stdout,
    );
    line(
        &rendered.replacen(&painted_label, &styled_label, 1),
        Stream::Stdout,
    );
}

pub fn success(label: &str, value: &str) {
    row(label, value, Color::Success);
}

pub fn warning_row(label: &str, value: &str) {
    row(label, value, Color::Warning);
}

pub fn toggle(label: &str, enabled: bool) {
    row(
        label,
        if enabled { "ON" } else { "OFF" },
        if enabled {
            Color::Success
        } else {
            Color::Muted
        },
    );
}

pub fn preference(label: &str, enabled: bool, description: &str) {
    separate(Stream::Stdout);
    toggle(label, enabled);
    text(description);
}

fn change(label: &str, before: &str, after: &str, color: Color) {
    if before == after {
        row(label, after, color);
    } else {
        let before = if before.is_empty() {
            paint("not configured", Color::Muted, Stream::Stdout)
        } else {
            paint(before, color, Stream::Stdout)
        };
        let value = format!("{before} → {}", paint(after, color, Stream::Stdout));
        line(&columns(label, &value, Color::Muted, 2), Stream::Stdout);
    }
}

pub fn account_change(before: &str, after: &str) {
    change("GitHub account", before, after, Color::Username);
}

pub fn global_commit_details(identity: &Identity, before: &CommitDetails) {
    section("Global commit defaults");
    change(
        "Name",
        &before.name,
        &identity.commit_name,
        Color::CommitName,
    );
    change("Email", &before.email, &identity.commit_email, Color::Email);
}

pub fn commit_details(details: &CommitDetails, before: Option<&CommitDetails>) {
    if let Some(before) = before {
        change(
            "Commit name",
            &before.name,
            &details.name,
            Color::CommitName,
        );
        change("Commit email", &before.email, &details.email, Color::Email);
    } else {
        row("Commit name", &details.name, Color::CommitName);
        commit_email(&details.email);
    }
}

pub enum AccountState {
    Active,
    SignedIn,
    NotSignedIn,
}

fn marker(state: AccountState) -> String {
    match state {
        AccountState::Active => format!(" {}", paint("(Active)", Color::Success, Stream::Stdout)),
        AccountState::SignedIn => String::new(),
        AccountState::NotSignedIn => format!(
            " {}",
            paint("(Not signed in)", Color::Warning, Stream::Stdout)
        ),
    }
}

pub fn identity(name: &str, identity: &Identity, location: &Path, state: AccountState) {
    separate(Stream::Stdout);
    text(&format!(
        "{}{}",
        identity_name(name, Stream::Stdout),
        marker(state)
    ));
    nested_row("GitHub username", &identity.username, Color::Username, 4);
    nested_row("Commit name", &identity.commit_name, Color::CommitName, 4);
    nested_row("Commit email", &identity.commit_email, Color::Email, 4);
    nested_row("Identity file", &location.to_string_lossy(), Color::Path, 4);
}

pub fn account(login: &str, active: bool) {
    text(&format!(
        "{}{}",
        username(login, Stream::Stdout),
        marker(if active {
            AccountState::Active
        } else {
            AccountState::SignedIn
        })
    ));
}

pub fn allowed_identity(name: &str, login: Option<&str>) {
    let value = login
        .map(|login| username(login, Stream::Stdout))
        .unwrap_or_else(|| {
            paint(
                "Not configured; remove this permission or run mgh repo allowed update",
                Color::Warning,
                Stream::Stdout,
            )
        });
    line(&columns(name, &value, Color::Identity, 2), Stream::Stdout);
}

pub fn hook(name: &str, location: &Path, executable: bool) {
    let value = format!(
        "{}{}",
        paint(&location.to_string_lossy(), Color::Path, Stream::Stdout),
        if executable {
            String::new()
        } else {
            format!(
                " {}",
                paint("(not executable; skipped)", Color::Warning, Stream::Stdout)
            )
        }
    );
    line(&columns(name, &value, Color::Muted, 2), Stream::Stdout);
}

pub fn autoswitch(name: &str, already_active: bool) {
    separate(Stream::Stdout);
    let name = identity_name(name, Stream::Stdout);
    let message = if already_active {
        format!("Only one allowed identity detected; selected {name} (already active).")
    } else {
        format!("Only one allowed identity detected; selecting {name}.")
    };
    success("Autoswitch", &message);
}

pub fn text(value: &str) {
    indented(value, Stream::Stdout);
}

fn indented(value: &str, stream: Stream) {
    for value in value.lines() {
        line(&format!("  {value}"), stream);
    }
}

pub fn paragraph(value: &str) {
    separate(Stream::Stdout);
    text(value);
    separate(Stream::Stdout);
}

pub fn warning(message: &str) {
    separate(Stream::Stdout);
    text(&paint(
        &format!("⚠ {message}"),
        Color::Warning,
        Stream::Stdout,
    ));
    separate(Stream::Stdout);
}

pub fn error(message: &str) {
    separate(Stream::Stderr);
    line(
        &format!("  {}", paint("✕ mgh", Color::Error, Stream::Stderr)),
        Stream::Stderr,
    );
    indented(message, Stream::Stderr);
    separate(Stream::Stderr);
}

pub fn block(value: &str, stream: Stream) {
    if value.trim().is_empty() {
        return;
    }
    separate(stream);
    indented(value.trim_matches(['\n', '\r']), stream);
    separate(stream);
}

pub fn cli_message(error: &clap::Error) {
    let stream = if error.use_stderr() {
        Stream::Stderr
    } else {
        Stream::Stdout
    };
    let rendered = error.render();
    let message = if color_enabled(stream) {
        rendered.ansi().to_string()
    } else {
        rendered.to_string()
    };
    separate(stream);
    for value in message.trim_matches(['\n', '\r']).lines() {
        line(value, stream);
    }
    separate(stream);
}
