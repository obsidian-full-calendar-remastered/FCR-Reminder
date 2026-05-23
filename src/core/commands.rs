use crate::core::api::{request_json_from_daemon, send_loopback_request};
use crate::core::storage::get_app_dir;
use crate::{log_error, log_info, log_warn};

pub enum InspectCommand {
    Health,
    Next,
    Events,
    Storage,
    Doctor,
    Updates,
}

pub enum LifecycleCommand {
    Start,
    Stop,
    Restart,
}

pub fn parse_inspect_command(value: &str) -> InspectCommand {
    match value.to_ascii_lowercase().as_str() {
        "health" | "status" => InspectCommand::Health,
        "next" => InspectCommand::Next,
        "events" | "list" | "list-events" => InspectCommand::Events,
        "storage" | "paths" => InspectCommand::Storage,
        "doctor" => InspectCommand::Doctor,
        "updates" | "update" | "release" | "releases" => InspectCommand::Updates,
        other => {
            eprintln!("Unknown inspect target: {}", other);
            print_help();
            std::process::exit(1);
        }
    }
}

pub fn execute_inspect_command(command: InspectCommand) -> Result<(), String> {
    let path = match command {
        InspectCommand::Health => "/status",
        InspectCommand::Next => "/next",
        InspectCommand::Events => "/events",
        InspectCommand::Storage => "/storage",
        InspectCommand::Doctor => "/doctor",
        InspectCommand::Updates => "/updates",
    };

    let payload = request_json_from_daemon(path)?;
    let rendered = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("Failed to render daemon response: {}", error))?;
    println!("{}", rendered);
    Ok(())
}

pub fn execute_lifecycle_command(command: LifecycleCommand) -> Result<(), String> {
    match command {
        LifecycleCommand::Start => start_daemon_if_needed(),
        LifecycleCommand::Stop => {
            request_daemon_post("/lifecycle/stop")?;
            println!("Requested FCR Reminder shutdown.");
            Ok(())
        }
        LifecycleCommand::Restart => {
            request_daemon_post("/lifecycle/restart")?;
            println!("Requested FCR Reminder restart.");
            Ok(())
        }
    }
}

pub fn start_daemon_if_needed() -> Result<(), String> {
    if request_json_from_daemon("/status").is_ok() {
        println!("FCR Reminder is already running.");
        return Ok(());
    }

    let current_exe = std::env::current_exe()
        .map_err(|error| format!("Failed to locate current executable: {}", error))?;

    let mut cmd = std::process::Command::new(&current_exe);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW to prevent terminal flashing
    }

    cmd.spawn()
        .map_err(|error| format!("Failed to launch FCR Reminder: {}", error))?;

    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        if request_json_from_daemon("/status").is_ok() {
            println!("FCR Reminder started successfully.");
            return Ok(());
        }
    }

    Err(
        "FCR Reminder was launched but did not become reachable on 127.0.0.1:45677 in time."
            .to_string(),
    )
}

pub fn request_daemon_post(path: &str) -> Result<(), String> {
    let _ = send_loopback_request("POST", path, None)?;
    Ok(())
}

/// Performs a complete cleanup of registry keys, autostart run entries, and AppData files,
/// leaving the user's operating system in a 100% clean state.
pub fn perform_complete_cleanup() -> Result<(), String> {
    println!("\n=== Performing Complete System Cleanup for FCR Reminder ===");

    ensure_daemon_stopped_for_cleanup()?;

    // 1. Clean platform-specific configurations (registry keys on Windows, systemd files on Linux, etc.)
    if let Err(e) = crate::platform::cleanup() {
        eprintln!("Platform Cleanup Error: {}", e);
    }

    // 2. Remove Local AppData Directories and Files
    if let Some(app_dir) = get_app_dir() {
        if app_dir.exists() {
            match std::fs::remove_dir_all(&app_dir) {
                Ok(_) => println!(
                    "AppData: Successfully deleted local app directory at: {}",
                    app_dir.to_string_lossy()
                ),
                Err(e) => eprintln!("AppData Warning: Failed to delete app directory: {}", e),
            }
        } else {
            println!("AppData: Local app directory is already clean.");
        }
    }

    println!("=== Cleanup Complete. Your system is now 100% clean of all assets! ===\n");
    Ok(())
}

fn ensure_daemon_stopped_for_cleanup() -> Result<(), String> {
    if request_json_from_daemon("/status").is_err() {
        println!("Daemon: No running FCR Reminder instance detected.");
        return Ok(());
    }

    println!("Daemon: Running instance detected. Requesting clean shutdown before cleanup...");
    request_daemon_post("/lifecycle/stop")?;

    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        if request_json_from_daemon("/status").is_err() {
            println!("Daemon: FCR Reminder stopped successfully.");
            return Ok(());
        }
    }

    Err("Cleanup aborted: FCR Reminder is still running after a shutdown request. Stop it first, then rerun cleanup.".to_string())
}

/// Handles a custom protocol URI action by parsing query params and communicating with the running daemon.
pub fn handle_protocol_uri(uri: &str) {
    log_info!("Received protocol activation URI: {}", uri);

    if let Some(query_start) = uri.find('?') {
        let query = &uri[query_start + 1..];
        let mut id = None;
        let mut title = None;
        let mut body = None;
        let mut action_url = None;
        let mut minutes = None;

        for part in query.split('&') {
            let mut kv = part.splitn(2, '=');
            if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                let decoded_v = percent_decode_str(v).unwrap_or_else(|| v.to_string());
                match k {
                    "id" => id = Some(decoded_v),
                    "title" => title = Some(decoded_v),
                    "body" => body = Some(decoded_v),
                    "action_url" => action_url = Some(decoded_v),
                    "snoozeTime" => {
                        if let Ok(m) = decoded_v.parse::<i64>() {
                            minutes = Some(m);
                        }
                    }
                    _ => {}
                }
            }
        }

        if let (Some(id), Some(title), Some(body)) = (id, title, body) {
            let action_url = action_url.unwrap_or_default();
            let minutes = match minutes {
                Some(value) => value,
                None => {
                    log_warn!(
                        "Protocol activation did not include a snooze duration. Defaulting reminder '{}' to 5 minutes.",
                        title
                    );
                    5
                }
            };

            match send_snooze_request(&id, &title, &body, &action_url, minutes) {
                Ok(_) => {
                    log_info!(
                        "Forwarded snooze request for reminder '{}' ({} minutes) to the daemon.",
                        title,
                        minutes
                    );
                }
                Err(e) => {
                    log_error!(
                        "Failed to forward snooze request for reminder '{}': {}",
                        title,
                        e
                    );
                    eprintln!("Failed to send snooze request to daemon: {}", e);
                }
            }
        } else {
            log_warn!(
                "Protocol activation URI was missing required snooze fields: {}",
                uri
            );
            eprintln!("Invalid protocol URI parameters.");
        }
    } else {
        log_warn!(
            "Protocol activation URI was missing a query string: {}",
            uri
        );
        eprintln!("Missing query string in protocol URI.");
    }
}

/// Helper to decode percent-encoded URI components.
fn percent_decode_str(input: &str) -> Option<String> {
    let mut bytes = Vec::new();
    let input_bytes = input.as_bytes();
    let mut i = 0;
    while i < input_bytes.len() {
        if input_bytes[i] == b'%' {
            if i + 2 < input_bytes.len() {
                let hex = &input[i + 1..i + 3];
                if let Ok(b) = u8::from_str_radix(hex, 16) {
                    bytes.push(b);
                    i += 3;
                    continue;
                }
            }
            return None;
        } else if input_bytes[i] == b'+' {
            bytes.push(b' ');
            i += 1;
        } else {
            bytes.push(input_bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(bytes).ok()
}

/// Dispatches a loopback TCP request containing the snooze payload to the background daemon.
fn send_snooze_request(
    id: &str,
    title: &str,
    body: &str,
    action_url: &str,
    minutes: i64,
) -> Result<(), String> {
    let payload = serde_json::json!({
        "id": id,
        "title": title,
        "body": body,
        "action_url": action_url,
        "minutes": minutes,
    });
    let body_str = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let response = send_loopback_request("POST", "/snooze", Some(&body_str))?;

    if response.contains("200 OK") || response.contains("201 Created") {
        Ok(())
    } else {
        Err(format!(
            "Daemon returned non-success response: {}",
            response
        ))
    }
}

/// Prints a detailed CLI help menu.
pub fn print_help() {
    println!(
        r#"FCR Reminder Background Daemon

Usage:
  fcr-reminder.exe [OPTIONS]

Options:
  -h, --help        Show this help message and exit
  -d, --debug       Run in debug mode (keeps terminal window visible and prints active logs)
  -c, --cleanup     Completely uninstall/cleanup registry entries and local database files
  -u, --uri <URI>   Handle a custom protocol activation URI (used internally for snooze/actions)
      
      --health      Query the running daemon for its health and current storage details
      --doctor      Run a complete live diagnostic check against the running daemon
      --next        Query the running daemon for the next scheduled reminder
      --events      Query the running daemon for all reminders currently stored on disk
      --storage     Query the running daemon for its dynamically resolved storage paths
      --start       Start the daemon if it is not already running
      --stop        Ask the running daemon to shut itself down cleanly
      --restart     Ask the running daemon to restart itself cleanly
    --updates     Query the running daemon for GitHub release update status
    --inspect     Query the running daemon using one of: health, next, events, storage, doctor, updates
    --gui / --view Launch the interactive Event Viewer GUI window

Branding & Behavior:
  On Windows release builds, the daemon launches as a tray-first background app with no console.
  Use --debug from an existing terminal session when you want live log output.
  Syncs reminders from Obsidian Full Calendar Remastered plugin via HTTP on port 45677."#
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_inspect_command_supports_updates_aliases() {
        assert!(matches!(
            parse_inspect_command("updates"),
            InspectCommand::Updates
        ));
        assert!(matches!(
            parse_inspect_command("update"),
            InspectCommand::Updates
        ));
        assert!(matches!(
            parse_inspect_command("release"),
            InspectCommand::Updates
        ));
        assert!(matches!(
            parse_inspect_command("releases"),
            InspectCommand::Updates
        ));
    }
}
