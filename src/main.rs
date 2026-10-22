mod application;
mod cli;
mod commands;
mod presentation;

use clap::Parser;

fn main() {
    let cli = match cli::Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let code = error.exit_code();
            presentation::parser_error(&error);
            std::process::exit(code);
        }
    };
    let mut terminal = presentation::Terminal::new();
    let result = commands::run(cli, &mut terminal);
    let code = match result {
        Ok(code) => code,
        Err(error) if error.downcast_ref::<application::Cancelled>().is_some() => {
            terminal.warning("Operation cancelled.");
            1
        }
        Err(error) => {
            terminal.error(&format!("{error:#}"));
            1
        }
    };
    terminal.finish();
    std::process::exit(code);
}
