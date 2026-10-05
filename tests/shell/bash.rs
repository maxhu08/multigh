use crate::support::terminal;

#[test]
fn prompt_events_report_startup_and_directory_changes_and_respect_verbose_off() {
    super::entry_reports(
        "bash",
        &["--noprofile", "--norc", "-ic"],
        "eval \"$(mgh shell init bash)\"; eval \"$PROMPT_COMMAND\"; eval \"$PROMPT_COMMAND\"; cd OTHER; eval \"$PROMPT_COMMAND\"",
    );
}

#[test]
fn interactive_startup_prompts_even_with_verbose_off() {
    super::startup_picker(
        "bash",
        &["--noprofile", "--norc", "-ic"],
        "eval \"$(mgh shell init bash)\"; eval \"$PROMPT_COMMAND\"",
    );
}

#[test]
fn noninteractive_shells_do_not_run_entry_checks() {
    super::noninteractive(
        "bash",
        &["--noprofile", "--norc", "-c"],
        "eval \"$(mgh shell init bash)\"; cd OTHER",
    );
}

#[test]
fn repeated_initialization_preserves_string_and_array_prompt_handlers() {
    for setup in [
        "PROMPT_COMMAND='printf preserved'",
        "PROMPT_COMMAND=('printf preserved' 'printf second')",
    ] {
        let sandbox = super::configured_repositories();
        let script = format!(
            "{setup}; eval \"$(mgh shell init bash)\"; eval \"$(mgh shell init bash)\"; for handler in \"${{PROMPT_COMMAND[@]}}\"; do eval \"$handler\"; done"
        );
        let (status, output) = terminal(
            sandbox
                .command("bash")
                .args(["--noprofile", "--norc", "-ic", &script]),
            &[],
        );

        assert!(status.success(), "{output}");
        assert_eq!(output.matches("preserved").count(), 1, "{output}");
        assert_eq!(output.matches("Allowed identities").count(), 1, "{output}");

        if setup.contains("second") {
            assert!(output.contains("second"), "{output}");
        }
    }
}

#[test]
fn autoswitch_selects_on_startup_and_switches_on_directory_entry() {
    super::autoswitch_on_startup_and_directory_entry(
        "bash",
        &["--noprofile", "--norc", "-ic"],
        "eval \"$(mgh shell init bash)\"; eval \"$PROMPT_COMMAND\"; cd OTHER; eval \"$PROMPT_COMMAND\"",
    );
}
