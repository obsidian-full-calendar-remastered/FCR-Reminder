use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

pub fn run_main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{}", error);
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let daemon_path = resolve_daemon_path()?;
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let forwarded_args = if args.is_empty() {
        vec![OsString::from("--start")]
    } else {
        args
    };

    let status = Command::new(&daemon_path)
        .args(&forwarded_args)
        .status()
        .map_err(|error| {
            format!(
                "Failed to launch '{}' from '{}': {}",
                daemon_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("fcr-reminder.exe"),
                daemon_path.display(),
                error
            )
        })?;

    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
}

fn resolve_daemon_path() -> Result<PathBuf, String> {
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("Failed to resolve current executable path: {}", error))?;
    let daemon_name = if cfg!(windows) {
        "fcr-reminder.exe"
    } else {
        "fcr-reminder"
    };

    let daemon_path = current_exe
        .parent()
        .ok_or_else(|| "Failed to resolve executable directory".to_string())?
        .join(daemon_name);

    if daemon_path.exists() {
        Ok(daemon_path)
    } else {
        Err(format!(
            "Could not find the GUI daemon executable at '{}'.",
            daemon_path.display()
        ))
    }
}
