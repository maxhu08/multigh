use crate::{git::global::Writer, utils::output};
use anyhow::Result;

pub(super) enum Report {
    Pending,
    Printed,
    Suppressed,
}

impl Report {
    pub fn print(&mut self, writer: &Writer<'_>) {
        if !matches!(self, Self::Pending) {
            return;
        }

        output::section(&if writer.changed() {
            format!("✓ Global Git config updated by {}", writer.command())
        } else {
            "Global Git config unchanged".to_owned()
        });
        println!();
        *self = Self::Printed;
    }

    pub fn suppress(&mut self) {
        *self = Self::Suppressed;
    }
}

pub(super) fn update<T>(
    command: &str,
    update: impl FnOnce(&mut Writer<'_>, &mut Report) -> Result<T>,
) -> Result<T> {
    let mut writer = Writer::new(command);
    let mut report = Report::Pending;
    let result = update(&mut writer, &mut report);

    if writer.changed() || result.is_ok() {
        report.print(&writer);
    }

    result
}
