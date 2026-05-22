use crate::core::Reminder;
use std::error::Error;

/// Fallback startup initialization for other platforms (no-op).
pub fn init() -> Result<(), Box<dyn Error>> {
    Ok(())
}

/// Fallback cleanup/uninstallation for other platforms (no-op).
pub fn cleanup() -> Result<(), Box<dyn Error>> {
    Ok(())
}

/// Fallback console preparation for CLI modes (no-op).
pub fn prepare_console_for_cli() {}

/// Fallback notification trigger (no-op).
pub fn trigger_notification(_reminder: &Reminder) -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub fn doctor_checks() -> Vec<(&'static str, bool)> {
    Vec::new()
}

pub fn show_about_dialog() -> Result<(), Box<dyn Error>> {
    Ok(())
}

/// Fallback message loop/event handler that sleeps indefinitely.
pub fn run_event_loop() {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
