use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use reminder_core::Reminder;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::watch;
use tower_http::cors::{Any, CorsLayer};

struct AppState {
    tx: watch::Sender<()>,
}

#[tokio::main]
async fn main() {
    // 1. Check for command-line arguments to handle options
    let args: Vec<String> = std::env::args().collect();

    let mut is_debug = false;
    let mut is_cleanup = false;
    let mut is_help = false;

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--debug" | "-d" => is_debug = true,
            "--cleanup" | "--uninstall" | "-c" => is_cleanup = true,
            "--help" | "-h" => is_help = true,
            other => {
                eprintln!("Unknown argument: {}", other);
                print_help();
                std::process::exit(1);
            }
        }
    }

    if is_help {
        print_help();
        std::process::exit(0);
    }

    if is_cleanup {
        perform_complete_cleanup();
        std::process::exit(0);
    }

    // 2. Headless Console Window Handling
    if !is_debug {
        hide_console_window();
    }

    reminder_core::log_info!("=== starting full-calendar-remastered reminder daemon ===");

    // 3. Extract assets and register in Windows Registry
    ensure_assets_extracted();

    #[cfg(target_os = "windows")]
    {
        register_custom_app_id();
        register_autostart();
    }

    // Determine and display database storage path for transparency
    if let Some(path) = reminder_core::get_storage_path() {
        reminder_core::log_info!("Storage path: {}", path.to_string_lossy());
    } else {
        reminder_core::log_error!("Warning: Could not determine data storage path.");
    }

    // 4. Create and initialize system tray icon on a dedicated OS thread
    run_tray_thread();

    // Spawn Tokio system tray menu click event handler task
    let menu_rx = tray_icon::menu::MenuEvent::receiver().clone();
    tokio::spawn(async move {
        loop {
            while let Ok(event) = menu_rx.try_recv() {
                if event.id.as_ref() == "quit" {
                    reminder_core::log_info!(
                        "Quit menu option clicked in system tray. Shutting down daemon..."
                    );
                    std::process::exit(0);
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    // Create a watch channel to notify the scheduler of database updates
    let (tx, rx) = watch::channel(());
    let state = Arc::new(AppState { tx });

    // Spawn the scheduler task in the background
    let mut scheduler_rx = rx.clone();
    tokio::spawn(async move {
        run_scheduler(&mut scheduler_rx).await;
    });

    // Define Axum router with CORS support for Obsidian plugin calls
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/status", get(handle_status))
        .route("/sync", post(handle_sync))
        .with_state(state)
        .layer(cors);

    // Bind to 127.0.0.1:45677 (localhost only, for security)
    let addr = SocketAddr::from(([127, 0, 0, 1], 45677));
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            reminder_core::log_error!(
                "CRITICAL ERROR: Failed to bind to 127.0.0.1:45677. Is another instance running?"
            );
            reminder_core::log_error!("Error: {}", e);
            std::process::exit(1);
        }
    };

    reminder_core::log_info!("HTTP Server listening on: http://{}", addr);

    if let Err(e) = axum::serve(listener, app).await {
        reminder_core::log_error!("HTTP Server error: {}", e);
    }
}

/// Prints a detailed CLI help menu.
fn print_help() {
    println!(
        r#"FCR Reminder Background Daemon

Usage:
  desktop.exe [OPTIONS]

Options:
  -h, --help        Show this help message and exit
  -d, --debug       Run in debug mode (keeps terminal window visible and prints active logs)
  -c, --cleanup     Completely uninstall/cleanup registry entries and local database files

Branding & Behavior:
  By default, the daemon runs headlessly in the background with a system tray icon.
  Syncs reminders from Obsidian Full Calendar Remastered plugin via HTTP on port 45677."#
    );
}

/// Dynamically hides the active console window on Windows.
#[cfg(target_os = "windows")]
fn hide_console_window() {
    use windows_sys::Win32::System::Console::GetConsoleWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

    let hwnd = unsafe { GetConsoleWindow() };
    if hwnd != 0 {
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn hide_console_window() {}

/// Registers the application in the Windows Registry to automatically run on user login.
#[cfg(target_os = "windows")]
fn register_autostart() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    if let Ok(current_exe) = std::env::current_exe() {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let subkey_path = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

        match hkcu.open_subkey_with_flags(subkey_path, winreg::enums::KEY_WRITE) {
            Ok(key) => {
                if let Err(e) =
                    key.set_value("FCRReminder", &current_exe.to_string_lossy().to_string())
                {
                    reminder_core::log_error!(
                        "Failed to register FCR Reminder in Startup Run registry key: {}",
                        e
                    );
                } else {
                    reminder_core::log_info!(
                        "Registered FCR Reminder for automatic Windows startup."
                    );
                }
            }
            Err(e) => {
                reminder_core::log_warn!(
                    "Failed to open Run registry key for startup registration: {}",
                    e
                );
            }
        }
    }
}

/// Loads the embedded calendar/clock reminder icon to RGBA raw buffer.
fn load_tray_icon() -> tray_icon::Icon {
    let icon_bytes = include_bytes!("../../assets/icon.png");
    let image = image::load_from_memory(icon_bytes)
        .expect("Failed to load icon from memory")
        .to_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    tray_icon::Icon::from_rgba(rgba, width, height).expect("Failed to create tray icon")
}

/// Dedicated background OS thread setup to register the tray icon and run the message loop.
fn run_tray_thread() {
    std::thread::spawn(move || {
        let tray_menu = tray_icon::menu::Menu::new();
        let status_item = tray_icon::menu::MenuItem::new("Status: Running", false, None);
        let quit_item = tray_icon::menu::MenuItem::with_id("quit", "Quit", true, None);

        let _ = tray_menu.append(&status_item);
        let _ = tray_menu.append(&tray_icon::menu::PredefinedMenuItem::separator());
        let _ = tray_menu.append(&quit_item);

        let icon = load_tray_icon();
        let _tray_icon = tray_icon::TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("FCR Reminder")
            .with_icon(icon)
            .build()
            .unwrap();

        #[cfg(target_os = "windows")]
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                DispatchMessageW, GetMessageW, TranslateMessage, MSG,
            };
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        #[cfg(not(target_os = "windows"))]
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    });
}

/// Performs a complete cleanup of registry keys, autostart run entries, and AppData files,
/// leaving the user's operating system in a 100% clean state.
fn perform_complete_cleanup() {
    println!("\n=== Performing Complete System Cleanup for FCR Reminder ===");

    // 1. Remove Windows Registry AppUserModelId entry
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let subkey_path = "Software\\Classes\\AppUserModelId\\FCRReminder";

        if hkcu.open_subkey(subkey_path).is_ok() {
            match hkcu.delete_subkey(subkey_path) {
                Ok(_) => println!("Registry: Successfully removed 'FCRReminder' AppUserModelId from Windows Registry."),
                Err(e) => eprintln!("Registry Warning: Failed to remove registry subkey: {}", e),
            }
        } else {
            println!("Registry: No 'FCRReminder' AppUserModelId entries found (already clean).");
        }
    }

    // 2. Remove Windows Startup Run entry
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let subkey_path = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

        if let Ok(key) = hkcu.open_subkey_with_flags(subkey_path, winreg::enums::KEY_WRITE) {
            match key.delete_value("FCRReminder") {
                Ok(_) => println!("Registry: Successfully removed 'FCRReminder' from Windows Startup Run entries."),
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        eprintln!("Registry Warning: Failed to delete startup Run value: {}", e);
                    } else {
                        println!("Registry: No Startup Run entry found (already clean).");
                    }
                }
            }
        }
    }

    // 3. Remove Local AppData Directories and Files
    if let Some(app_dir) = reminder_core::get_app_dir() {
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
}

/// Extracts the embedded calendar/clock reminder icon to local AppData.
fn ensure_assets_extracted() {
    if let Some(app_dir) = reminder_core::get_app_dir() {
        let icon_path = app_dir.join("icon.png");

        let _ = std::fs::create_dir_all(&app_dir);

        let icon_bytes = include_bytes!("../../assets/icon.png");

        if !icon_path.exists() {
            if let Err(e) = std::fs::write(&icon_path, icon_bytes) {
                reminder_core::log_error!("Failed to extract app icon: {}", e);
            } else {
                reminder_core::log_info!(
                    "Successfully extracted premium reminder icon to AppData."
                );
            }
        }
    }
}

/// Registers the AppUserModelId in Windows Registry under HKEY_CURRENT_USER
/// to enable custom Application Name ("FCR Reminder") and icon in Toast notifications without admin rights.
#[cfg(target_os = "windows")]
fn register_custom_app_id() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    if let Some(app_dir) = reminder_core::get_app_dir() {
        let icon_path = app_dir.join("icon.png");
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let subkey_path = "Software\\Classes\\AppUserModelId\\FCRReminder";

        match hkcu.create_subkey(subkey_path) {
            Ok((key, _)) => {
                let _ = key.set_value("DisplayName", &"FCR Reminder");
                let _ = key.set_value("IconUri", &icon_path.to_string_lossy().to_string());
                reminder_core::log_info!(
                    "Registered custom AppUserModelId 'FCRReminder' in Windows Registry."
                );
            }
            Err(e) => {
                reminder_core::log_warn!("Failed to create custom Registry AppId subkey: {}", e);
            }
        }
    }
}

/// Endpoint to check daemon health and database stats.
async fn handle_status() -> Json<serde_json::Value> {
    let reminders = reminder_core::load_reminders().unwrap_or_default();
    Json(serde_json::json!({
        "status": "running",
        "active_reminders": reminders.len(),
        "database_path": reminder_core::get_storage_path().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
    }))
}

/// Endpoint to receive standard reminder synchronization payloads from Obsidian.
async fn handle_sync(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Vec<Reminder>>,
) -> Result<StatusCode, (StatusCode, String)> {
    reminder_core::log_info!(
        "Received Sync request: {} reminders provided.",
        payload.len()
    );

    if let Err(e) = reminder_core::save_reminders(&payload) {
        reminder_core::log_error!("Failed to save reminders: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save: {}", e),
        ));
    }

    // Wake up the scheduler loop to recalculate target timestamps
    let _ = state.tx.send(());
    Ok(StatusCode::OK)
}

/// Core background scheduler loop.
/// Sleeps until the next active reminder, woke up instantly on sync updates.
async fn run_scheduler(rx: &mut watch::Receiver<()>) {
    reminder_core::log_info!("Background scheduler started.");

    loop {
        let current_time = chrono::Utc::now().timestamp();
        let reminders = reminder_core::load_reminders().unwrap_or_default();

        // Filter for active future reminders and sort ascending
        let mut active: Vec<Reminder> = reminders
            .into_iter()
            .filter(|r| r.trigger_at_epoch > current_time)
            .collect();

        active.sort_by_key(|r| r.trigger_at_epoch);

        if active.is_empty() {
            reminder_core::log_info!(
                "No active future reminders. Sleeping until next synchronization."
            );
            // Sleep indefinitely until we receive a wakeup signal
            if rx.changed().await.is_err() {
                break; // Watch channel closed, terminate scheduler
            }
            continue;
        }

        let next_reminder = active[0].clone();
        let delay_secs = next_reminder.trigger_at_epoch - current_time;

        reminder_core::log_info!(
            "Next reminder scheduled: \"{}\" in {} seconds (at Epoch {}).",
            next_reminder.title,
            delay_secs,
            next_reminder.trigger_at_epoch
        );

        let delay = std::time::Duration::from_secs(delay_secs.max(0) as u64);

        // Sleep until either the timer expires OR a sync signal occurs
        tokio::select! {
            _ = tokio::time::sleep(delay) => {
                reminder_core::log_info!("Reminder triggered! Firing notification for \"{}\".", next_reminder.title);
                trigger_notification(&next_reminder);

                // Remove the fired reminder from the persistent JSON store so it won't fire again
                let updated: Vec<Reminder> = reminder_core::load_reminders().unwrap_or_default()
                    .into_iter()
                    .filter(|r| r.id != next_reminder.id)
                    .collect();
                let _ = reminder_core::save_reminders(&updated);
            }
            res = rx.changed() => {
                if res.is_err() {
                    break; // Watch channel closed, terminate scheduler
                }
                reminder_core::log_info!("Synchronization signal received. Rescheduling reminders...");
            }
        }
    }
}

/// Dispatches a native operating system notification.
fn trigger_notification(reminder: &Reminder) {
    #[cfg(target_os = "windows")]
    {
        use winrt_notification::{Duration, Sound, Toast};

        // We use our custom registered AppUserModelId 'FCRReminder' to display
        // the app name "FCR Reminder" and the extracted high-quality icon!
        let app_id = "FCRReminder";

        let result = Toast::new(app_id)
            .title(&reminder.title)
            .text1(&reminder.body)
            .sound(Some(Sound::Reminder))
            .duration(Duration::Long)
            .show();

        if let Err(e) = result {
            reminder_core::log_error!("Windows Toast error: {:?}", e);
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use notify_rust::Notification;

        let result = Notification::new()
            .summary(&reminder.title)
            .body(&reminder.body)
            .show();

        if let Err(e) = result {
            reminder_core::log_error!("Desktop notification error: {:?}", e);
        }
    }
}
