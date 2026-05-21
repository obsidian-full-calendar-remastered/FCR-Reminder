use crate::models::Reminder;
use directories::ProjectDirs;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;

/// Returns the path to the directory where application data is stored.
pub fn get_app_dir() -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        // Dev build: Store all state within the workspace's local 'dev' directory
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir.parent().map(|p| p.join("dev"))
    } else {
        // Production build: Store all state in the standard OS local AppData directory
        ProjectDirs::from("com", "fullcalendar", "ReminderApp")
            .map(|proj_dirs| proj_dirs.data_local_dir().to_path_buf())
    }
}

/// Returns the path to the `reminders.json` database file.
pub fn get_storage_path() -> Option<PathBuf> {
    get_app_dir().map(|path| path.join("reminders.json"))
}

/// Loads the list of saved reminders from disk.
/// If the file does not exist, returns an empty vector.
pub fn load_reminders() -> Result<Vec<Reminder>, String> {
    let path = get_storage_path()
        .ok_or_else(|| "Could not determine local app data directory".to_string())?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut file = File::open(&path).map_err(|e| format!("Failed to open database file: {}", e))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| format!("Failed to read database file: {}", e))?;

    if contents.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str(&contents).map_err(|e| format!("Failed to parse database file: {}", e))
}

/// Overwrites the saved reminders on disk with the provided list.
pub fn save_reminders(reminders: &[Reminder]) -> Result<(), String> {
    let app_dir =
        get_app_dir().ok_or_else(|| "Could not determine local app data directory".to_string())?;
    let path = app_dir.join("reminders.json");

    // Ensure the application directory exists
    if !app_dir.exists() {
        create_dir_all(&app_dir)
            .map_err(|e| format!("Failed to create application directory: {}", e))?;
    }

    let json_data = serde_json::to_string_pretty(reminders)
        .map_err(|e| format!("Failed to serialize reminders: {}", e))?;

    let mut file =
        File::create(&path).map_err(|e| format!("Failed to create database file: {}", e))?;
    file.write_all(json_data.as_bytes())
        .map_err(|e| format!("Failed to write to database file: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reminder_serialization_deserialization() {
        let reminder = Reminder {
            id: "test-123".to_string(),
            title: "Test Title".to_string(),
            body: "Test Body".to_string(),
            trigger_at_epoch: 1234567890,
            action_url: "obsidian://open".to_string(),
        };

        let serialized = serde_json::to_string(&reminder).unwrap();
        let deserialized: Reminder = serde_json::from_str(&serialized).unwrap();

        assert_eq!(reminder, deserialized);
    }

    #[test]
    fn test_get_app_dir() {
        let app_dir = get_app_dir();
        assert!(app_dir.is_some());

        let path = app_dir.unwrap();
        // Since we are running cargo test, debug_assertions is always active,
        // so the path must end with "dev"
        assert!(path.to_string_lossy().ends_with("dev"));
    }

    #[test]
    fn test_save_load_reminders() {
        let test_reminders = vec![
            Reminder {
                id: "test-1".to_string(),
                title: "Title 1".to_string(),
                body: "Body 1".to_string(),
                trigger_at_epoch: 1000,
                action_url: "url1".to_string(),
            },
            Reminder {
                id: "test-2".to_string(),
                title: "Title 2".to_string(),
                body: "Body 2".to_string(),
                trigger_at_epoch: 2000,
                action_url: "url2".to_string(),
            },
        ];

        // Backup existing reminders database to avoid polluting state
        let db_path = get_storage_path().unwrap();
        let backup_path = db_path.with_extension("json.bak");
        let had_backup = if db_path.exists() {
            std::fs::rename(&db_path, &backup_path).is_ok()
        } else {
            false
        };

        // Run save test
        let save_res = save_reminders(&test_reminders);
        assert!(save_res.is_ok());

        // Run load test
        let load_res = load_reminders();
        assert!(load_res.is_ok());
        let loaded = load_res.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, "test-1");
        assert_eq!(loaded[1].title, "Title 2");

        // Clean up test file
        if db_path.exists() {
            let _ = std::fs::remove_file(&db_path);
        }

        // Restore backup if we had one
        if had_backup {
            let _ = std::fs::rename(&backup_path, &db_path);
        }
    }
}
