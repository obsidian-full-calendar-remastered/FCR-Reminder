use crate::core::release_updates::{ReleaseInfo, UpdateStateSnapshot};
use crate::core::Reminder;
use std::error::Error;

/// macOS-specific startup initialization.
/// Can be extended in Phase 4 to construct and copy launchd plist agents.
pub fn init() -> Result<(), Box<dyn Error>> {
    crate::log_info!("macOS initialization: standard startup configured.");
    Ok(())
}

/// macOS-specific cleanup/uninstallation.
/// Can be extended in Phase 4 to unload and delete launchd plists.
pub fn cleanup() -> Result<(), Box<dyn Error>> {
    crate::log_info!("macOS cleanup: launchd configuration purged.");
    Ok(())
}

/// Console preparation on macOS (no-op).
pub fn prepare_console_for_cli() {}

/// Triggers standard desktop notifications using macOS notification services.
pub fn trigger_notification(reminder: &Reminder) -> Result<(), Box<dyn Error>> {
    use notify_rust::Notification;

    Notification::new()
        .summary(&reminder.title)
        .body(&reminder.body)
        .show()
        .map(|_| ())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn doctor_checks() -> Vec<(&'static str, bool)> {
    Vec::new()
}

pub fn show_about_dialog(_update_state: &UpdateStateSnapshot) -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub fn trigger_update_notification(release: &ReleaseInfo) -> Result<(), Box<dyn Error>> {
    use notify_rust::Notification;

    Notification::new()
        .summary("Update available for FCR Reminder")
        .body(&format!("Version {} is available.", release.version))
        .show()
        .map(|_| ())
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

pub fn open_url(url: &str) -> Result<(), Box<dyn Error>> {
    std::process::Command::new("open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

/// Event loop for macOS Cocoa event/tray thread.
pub fn run_event_loop_once(timeout: std::time::Duration) {
    std::thread::sleep(timeout);
}
