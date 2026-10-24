mod layout;

use crate::application::{Cancelled, Choice, Meaning, Prompts, Reports};
use anyhow::{Context, Result};
use console::Style;
use layout::{BODY, NESTED, SECTION, VALUE, indent, write_indented};
use std::io::{self, IsTerminal, Write};

pub struct Terminal {
    started: bool,
    last_error: bool,
    padded: [bool; 2],
    progress: bool,
}

fn styling(stderr: bool) -> bool {
    std::env::var_os("NO_COLOR").is_none()
        && std::env::var("TERM").as_deref() != Ok("dumb")
        && if stderr {
            io::stderr().is_terminal()
        } else {
            io::stdout().is_terminal()
        }
}

fn color(meaning: Meaning) -> (u8, u8, u8, bool) {
    match meaning {
        Meaning::Heading => (244, 114, 182, true),
        Meaning::HeadingArrow => (244, 114, 182, false),
        Meaning::Identity => (103, 232, 249, true),
        Meaning::Username => (192, 132, 252, false),
        Meaning::Commit => (251, 146, 60, false),
        Meaning::Path => (148, 163, 184, false),
        Meaning::Success => (134, 239, 172, true),
        Meaning::Warning => (251, 191, 36, true),
        Meaning::Error => (251, 113, 133, true),
        Meaning::Plain => (255, 255, 255, false),
    }
}

fn styled(text: &str, meaning: Meaning, stderr: bool) -> String {
    if !styling(stderr) || matches!(meaning, Meaning::Plain) {
        return text.into();
    }
    let (r, g, b, bold) = color(meaning);
    let prefix = format!("\x1b[{};38;2;{r};{g};{b}m", if bold { "1" } else { "22" });
    if matches!(meaning, Meaning::HeadingArrow) {
        return format!("{prefix}{text}\x1b[0m");
    }
    text.split('→')
        .map(|part| {
            if part.is_empty() {
                String::new()
            } else {
                format!("{prefix}{part}\x1b[0m")
            }
        })
        .collect::<Vec<_>>()
        .join("→")
}

fn styled_parts(parts: &[(&str, Meaning)], stderr: bool) -> String {
    parts
        .iter()
        .map(|(text, meaning)| styled(text, *meaning, stderr))
        .collect()
}

impl Terminal {
    pub fn new() -> Self {
        console::set_colors_enabled(styling(false));
        console::set_colors_enabled_stderr(styling(true));
        Self {
            started: false,
            last_error: false,
            padded: [false; 2],
            progress: false,
        }
    }

    fn begin(&mut self, error: bool) {
        self.end_progress();
        if !self.started {
            if !self.padded[usize::from(error)] {
                if error {
                    eprintln!();
                } else {
                    println!();
                }
            }
            self.started = true;
        }
        self.padded[usize::from(error)] = false;
        self.last_error = error;
    }

    fn section_break(&mut self) {
        self.end_progress();
        if self.started && !self.padded[0] {
            println!();
            self.padded[0] = true;
        }
    }

    pub fn warning(&mut self, text: &str) {
        <Self as Reports>::warning(self, text);
    }

    pub fn error(&mut self, text: &str) {
        self.begin(true);
        eprintln!("{}", indent(&styled(text, Meaning::Error, true), SECTION));
    }

    pub fn finish(&mut self) {
        self.end_progress();
        if self.started {
            if !self.padded[usize::from(self.last_error)] {
                if self.last_error {
                    eprintln!();
                } else {
                    println!();
                }
            }
            self.started = false;
            self.padded[usize::from(self.last_error)] = true;
        }
    }

    fn form(&mut self, meaning: Meaning) {
        self.begin(true);
        cliclack::set_theme(FormTheme(meaning));
    }

    fn end_progress(&mut self) {
        if self.progress {
            println!();
            self.progress = false;
        }
    }
}

impl Reports for Terminal {
    fn separate(&mut self) {
        self.section_break();
    }

    fn heading_parts(&mut self, parts: &[(&str, Meaning)]) {
        self.section_break();
        self.begin(false);
        println!("{}", indent(&styled_parts(parts, false), SECTION));
    }

    fn identity_heading(&mut self, text: &str) {
        self.section_break();
        self.begin(false);
        println!("{}", indent(&styled(text, Meaning::Identity, false), BODY));
    }

    fn row(&mut self, label: &str, value: &str, meaning: Meaning, nested: bool) {
        self.begin(false);
        let columns = if nested { NESTED } else { BODY };
        let width = console::measure_text_width(label) + columns;
        let value = indent(&styled(value, meaning, false), VALUE);
        if width >= VALUE - 1 {
            println!("{}", indent(label, columns));
            println!("{value}");
        } else {
            println!(
                "{}{}{}",
                indent(label, columns),
                " ".repeat(VALUE - width),
                value.strip_prefix(&" ".repeat(VALUE)).unwrap_or(&value)
            );
        }
    }

    fn message_parts(&mut self, parts: &[(&str, Meaning)]) {
        self.begin(false);
        println!("{}", indent(&styled_parts(parts, false), BODY));
    }

    fn progress(&mut self, text: &str, finished: bool) {
        if !self.progress {
            self.begin(false);
            print!("{}", " ".repeat(BODY));
            self.progress = true;
        }
        print!("{}", styled(text, Meaning::Path, false));
        if finished {
            self.end_progress();
        }
        let _ = io::stdout().flush();
    }

    fn before_native(&mut self) {
        if self.started {
            let _ = io::stdout().flush();
            let _ = io::stderr().flush();
        }
    }

    fn native_report(&mut self, stdout: &[u8], stderr: &[u8]) -> Result<()> {
        self.finish();
        native_block(&mut io::stdout().lock(), stdout, &mut self.padded[0])?;
        native_block(&mut io::stderr().lock(), stderr, &mut self.padded[1])?;
        Ok(())
    }
}

fn native_block(stream: &mut dyn Write, mut bytes: &[u8], padded: &mut bool) -> io::Result<()> {
    while let Some(rest) = bytes
        .strip_prefix(b"\r\n")
        .or_else(|| bytes.strip_prefix(b"\n"))
    {
        bytes = rest;
    }
    while let Some(rest) = bytes
        .strip_suffix(b"\r\n")
        .or_else(|| bytes.strip_suffix(b"\n"))
    {
        bytes = rest;
    }
    if bytes.is_empty() {
        return Ok(());
    }
    if !*padded {
        writeln!(stream)?;
    }
    write_indented(stream, bytes, SECTION)?;
    writeln!(stream)?;
    writeln!(stream)?;
    stream.flush()?;
    *padded = true;
    Ok(())
}

fn prompt_result<T>(result: io::Result<T>) -> Result<T> {
    match result {
        Err(e) if e.kind() == io::ErrorKind::Interrupted => Err(Cancelled.into()),
        other => other.context("Terminal input failed"),
    }
}

struct FormTheme(Meaning);
impl cliclack::Theme for FormTheme {
    fn input_style(&self, _: &cliclack::ThemeState) -> Style {
        let (r, g, b, bold) = color(self.0);
        let style = Style::new().true_color(r, g, b);
        if bold { style.bold() } else { style }
    }

    fn format_footer(&self, _: &cliclack::ThemeState) -> String {
        String::new()
    }
}

impl Prompts for Terminal {
    fn interactive(&self) -> bool {
        io::stdin().is_terminal() && io::stdout().is_terminal() && io::stderr().is_terminal()
    }

    fn field(&mut self, label: &str, default: Option<&str>, meaning: Meaning) -> Result<String> {
        self.form(meaning);
        let label = match default {
            Some(value) => format!("{label} (default: {}): ", styled(value, meaning, true)),
            None => format!("{label}: "),
        };
        let mut input = cliclack::input(label).required(true);
        if let Some(default) = default {
            input = input.default_input(default);
        }
        let value: String = prompt_result(input.interact())?;
        Ok(value)
    }

    fn permissions(&mut self, choices: &[Choice], checked: &[String]) -> Result<Vec<String>> {
        self.form(Meaning::Identity);
        let mut prompt = cliclack::multiselect("Which identities may use this repository?")
            .required(true)
            .max_rows(7)
            .initial_values(checked.to_vec());
        for choice in choices {
            prompt = prompt.item(
                choice.name.clone(),
                styled(&choice.name, Meaning::Identity, true),
                styled(&choice.username, Meaning::Username, true),
            );
        }
        prompt_result(prompt.interact())
    }

    fn active(&mut self, choices: &[Choice], selected: Option<&str>) -> Result<String> {
        self.form(Meaning::Identity);
        let mut prompt = cliclack::select("Which allowed identity should be active?").max_rows(7);
        for choice in choices {
            prompt = prompt.item(
                choice.name.clone(),
                styled(&choice.name, Meaning::Identity, true),
                styled(&choice.username, Meaning::Username, true),
            );
        }
        if let Some(selected) = selected {
            prompt = prompt.initial_value(selected.to_string());
        }
        prompt_result(prompt.interact())
    }
}

pub fn parser_error(error: &clap::Error) {
    let stderr = error.use_stderr();
    let text = error.to_string();
    if stderr {
        eprintln!("\n{}\n", text.trim_end());
    } else {
        println!("\n{}\n", text.trim_end());
    }
}
