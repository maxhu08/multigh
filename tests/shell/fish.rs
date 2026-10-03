#[test]
fn startup_and_directory_events_report_identities_and_respect_verbose_off() {
    super::entry_reports(
        "fish",
        &["--no-config", "-ic"],
        "mgh init fish | source; cd OTHER",
    );
}

#[test]
fn interactive_startup_prompts_even_with_verbose_off() {
    super::startup_picker("fish", &["--no-config", "-ic"], "mgh init fish | source");
}

#[test]
fn noninteractive_shells_do_not_run_entry_checks() {
    super::noninteractive(
        "fish",
        &["--no-config", "-c"],
        "mgh init fish | source; cd OTHER",
    );
}
