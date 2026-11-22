use std::{env, io, process::Command};

fn forward() -> io::Result<i32> {
    let mgh = env::current_exe()?.with_file_name(format!("mgh{}", env::consts::EXE_SUFFIX));
    let mut command = Command::new(mgh);
    if let Some(config) = env::var_os("MGH_POWERSHELL_CONFIG") {
        command.arg("--config").arg(config);
    }
    let status = command
        .args(["internal", "git", "--"])
        .args(env::args_os().skip(1))
        .status()?;
    Ok(status.code().unwrap_or(1))
}

fn main() {
    std::process::exit(forward().unwrap_or_else(|error| {
        eprintln!("Cannot forward Git through mgh: {error}");
        1
    }));
}
