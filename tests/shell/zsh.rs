#[test]
fn startup_and_directory_events_report_identities_and_respect_verbose_off() {
    super::entry_reports("zsh", &["-fic"], "eval \"$(mgh init zsh)\"; cd OTHER");
}

#[test]
fn interactive_startup_prompts_even_with_verbose_off() {
    super::startup_picker("zsh", &["-fic"], "eval \"$(mgh init zsh)\"");
}

#[test]
fn noninteractive_shells_do_not_run_entry_checks() {
    super::noninteractive("zsh", &["-fc"], "eval \"$(mgh init zsh)\"; cd OTHER");
}
