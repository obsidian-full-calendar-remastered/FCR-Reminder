use crate::core::storage::{get_app_dir, get_storage_path};

#[derive(Debug, Clone)]
pub struct GuiContext {
    pub version: String,
    pub description: String,
    pub license: String,
    pub api_url: String,
    pub storage_path: String,
    pub executable_path: String,
    pub icon_path: String,
}

/// Orchestrates the launch of the event viewer by assembling context variables
/// and invoking the platform-specific wrapper.
pub fn open_event_viewer() {
    let version = env!("CARGO_PKG_VERSION").to_string();
    let description = env!("CARGO_PKG_DESCRIPTION").to_string();
    let license = env!("CARGO_PKG_LICENSE").to_string();
    let api_url = "http://127.0.0.1:45677".to_string();

    let executable_path = std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "Unavailable".to_string());

    let storage_path = get_storage_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "Unavailable".to_string());

    let icon_path = get_app_dir()
        .map(|dir| dir.join("icon.png").display().to_string())
        .unwrap_or_default();

    let context = GuiContext {
        version,
        description,
        license,
        api_url,
        storage_path,
        executable_path,
        icon_path,
    };

    crate::log_info!("Orchestrating Event Viewer GUI launch...");

    if let Err(e) = crate::platform::show_events_dialog(&context) {
        crate::log_error!("Failed to launch Event Viewer: {}", e);
    }
}
