use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use directories::ProjectDirs;
use crate::models::Reminder;

/// Returns the path to the directory where application data is stored.
pub fn get_app_dir() -> Option<PathBuf> {
    ProjectDirs::from("com", "fullcalendar", "ReminderApp")
        .map(|proj_dirs| proj_dirs.data_local_dir().to_path_buf())
}

/// Returns the path to the `reminders.json` database file.
pub fn get_storage_path() -> Option<PathBuf> {
    get_app_dir().map(|path| path.join("reminders.json"))
}

/// Loads the list of saved reminders from disk.
/// If the file does not exist, returns an empty vector.
pub fn load_reminders() -> Result<Vec<Reminder>, String> {
    let path = get_storage_path().ok_or_else(|| "Could not determine local app data directory".to_string())?;
    
    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut file = File::open(&path).map_err(|e| format!("Failed to open database file: {}", e))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|e| format!("Failed to read database file: {}", e))?;

    if contents.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str(&contents).map_err(|e| format!("Failed to parse database file: {}", e))
}

/// Overwrites the saved reminders on disk with the provided list.
pub fn save_reminders(reminders: &[Reminder]) -> Result<(), String> {
    let app_dir = get_app_dir().ok_or_else(|| "Could not determine local app data directory".to_string())?;
    let path = app_dir.join("reminders.json");

    // Ensure the application directory exists
    if !app_dir.exists() {
        create_dir_all(&app_dir).map_err(|e| format!("Failed to create application directory: {}", e))?;
    }

    let json_data = serde_json::to_string_pretty(reminders)
        .map_err(|e| format!("Failed to serialize reminders: {}", e))?;

    let mut file = File::create(&path).map_err(|e| format!("Failed to create database file: {}", e))?;
    file.write_all(json_data.as_bytes())
        .map_err(|e| format!("Failed to write to database file: {}", e))?;

    Ok(())
}
