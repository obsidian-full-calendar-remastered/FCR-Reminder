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

mod platform;

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
    let mut uri_arg: Option<String> = None;

    let mut skip_next = false;
    for (i, arg) in args.iter().enumerate().skip(1) {
        if skip_next {
            skip_next = false;
            continue;
        }
        match arg.as_str() {
            "--debug" | "-d" => is_debug = true,
            "--cleanup" | "--uninstall" | "-c" => is_cleanup = true,
            "--help" | "-h" => is_help = true,
            "--uri" | "-u" => {
                if i + 1 < args.len() {
                    uri_arg = Some(args[i + 1].clone());
                    skip_next = true;
                } else {
                    eprintln!("Missing value for --uri option");
                    std::process::exit(1);
                }
            }
            other => {
                if other.starts_with("fcr-reminder://") {
                    uri_arg = Some(other.to_string());
                } else {
                    eprintln!("Unknown argument: {}", other);
                    print_help();
                    std::process::exit(1);
                }
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

    if let Some(uri) = uri_arg {
        handle_protocol_uri(&uri);
        std::process::exit(0);
    }

    // 2. Headless Console Window Handling
    if !is_debug {
        platform::hide_console();
    }

    reminder_core::log_info!("=== starting full-calendar-remastered reminder daemon ===");

    // 3. Extract assets and register platform integrations
    ensure_assets_extracted();

    if let Err(e) = platform::init() {
        reminder_core::log_error!("Platform initialization failed: {}", e);
    }

    // Determine and display database storage path for transparency
    if let Some(path) = reminder_core::get_storage_path() {
        reminder_core::log_info!("Storage path: {}", path.to_string_lossy());
    } else {
        reminder_core::log_error!("Warning: Could not determine data storage path.");
    }

    // 4. Create and initialize system tray icon on a dedicated OS thread
    run_tray_thread();

    // Spawn system tray menu click event handler thread
    // This blocks at the OS level on menu_rx.recv() to ensure 0% active idle CPU wakeups
    let menu_rx = tray_icon::menu::MenuEvent::receiver().clone();
    std::thread::spawn(move || {
        while let Ok(event) = menu_rx.recv() {
            if event.id.as_ref() == "quit" {
                reminder_core::log_info!(
                    "Quit menu option clicked in system tray. Shutting down daemon..."
                );
                std::process::exit(0);
            }
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
        .route("/snooze", post(handle_snooze))
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
  -u, --uri <URI>   Handle a custom protocol activation URI (used internally for snooze/actions)

Branding & Behavior:
  By default, the daemon runs headlessly in the background with a system tray icon.
  Syncs reminders from Obsidian Full Calendar Remastered plugin via HTTP on port 45677."#
    );
}

/// Loads the embedded calendar/clock reminder icon to RGBA raw buffer.
fn load_tray_icon() -> tray_icon::Icon {
    let icon_bytes = include_bytes!("../../../assets/icon.png");
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

        platform::run_event_loop();
    });
}

/// Performs a complete cleanup of registry keys, autostart run entries, and AppData files,
/// leaving the user's operating system in a 100% clean state.
fn perform_complete_cleanup() {
    println!("\n=== Performing Complete System Cleanup for FCR Reminder ===");

    // 1. Clean platform-specific configurations (registry keys on Windows, systemd files on Linux, etc.)
    if let Err(e) = platform::cleanup() {
        eprintln!("Platform Cleanup Error: {}", e);
    }

    // 2. Remove Local AppData Directories and Files
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

        let icon_bytes = include_bytes!("../../../assets/icon.png");

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

    // Load active reminders from disk exactly once on startup to maintain an in-memory cache
    let mut reminders = reminder_core::load_reminders().unwrap_or_default();

    loop {
        let current_time = chrono::Utc::now().timestamp();

        // Filter for active future reminders and sort ascending using in-memory list
        let mut active: Vec<Reminder> = reminders
            .iter()
            .filter(|r| r.trigger_at_epoch > current_time)
            .cloned()
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
            // Reload reminders from disk when woke up by a synchronization event
            reminders = reminder_core::load_reminders().unwrap_or_default();
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

                // Update the in-memory cache directly without reading from disk
                reminders.retain(|r| r.id != next_reminder.id);

                // Write remaining reminders to disk for durability (single write, no disk reads)
                if let Err(e) = reminder_core::save_reminders(&reminders) {
                    reminder_core::log_error!("Failed to save reminders list after firing: {}", e);
                }
            }
            res = rx.changed() => {
                if res.is_err() {
                    break; // Watch channel closed, terminate scheduler
                }
                reminder_core::log_info!("Synchronization signal received. Reloading reminders from disk and rescheduling...");
                // Reload reminders from disk since the local database was modified by HTTP routes (/sync or /snooze)
                reminders = reminder_core::load_reminders().unwrap_or_default();
            }
        }
    }
}

/// Dispatches a native operating system notification.
fn trigger_notification(reminder: &Reminder) {
    if let Err(e) = platform::trigger_notification(reminder) {
        reminder_core::log_error!("Notification error: {}", e);
    }
}

/// Handles a custom protocol URI action by parsing query params and communicating with the running daemon.
fn handle_protocol_uri(uri: &str) {
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

        if let (Some(id), Some(title), Some(body), Some(action_url), Some(minutes)) =
            (id, title, body, action_url, minutes)
        {
            match send_snooze_request(&id, &title, &body, &action_url, minutes) {
                Ok(_) => {
                    // Successfully sent snooze command.
                }
                Err(e) => {
                    eprintln!("Failed to send snooze request to daemon: {}", e);
                }
            }
        } else {
            eprintln!("Invalid protocol URI parameters.");
        }
    } else {
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
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let payload = serde_json::json!({
        "id": id,
        "title": title,
        "body": body,
        "action_url": action_url,
        "minutes": minutes,
    });
    let body_str = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let request = format!(
        "POST /snooze HTTP/1.1\r\n\
         Host: 127.0.0.1:45677\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n\
         {}",
        body_str.len(),
        body_str
    );

    let mut stream = TcpStream::connect("127.0.0.1:45677")
        .map_err(|e| format!("Failed to connect to daemon: {}", e))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();

    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("Failed to write to stream: {}", e))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| format!("Failed to read response: {}", e))?;

    if response.contains("200 OK") || response.contains("201 Created") {
        Ok(())
    } else {
        Err(format!(
            "Daemon returned non-success response: {}",
            response
        ))
    }
}

/// JSON payload structure for snoozing.
#[derive(serde::Deserialize)]
struct SnoozePayload {
    id: String,
    title: String,
    body: String,
    action_url: String,
    minutes: i64,
}

/// Endpoint exposing local loopback snooze commands.
async fn handle_snooze(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SnoozePayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    reminder_core::log_info!(
        "Received Snooze request for reminder '{}' (snooze: {} minutes).",
        payload.title,
        payload.minutes
    );

    let current_time = chrono::Utc::now().timestamp();
    let new_trigger = current_time + (payload.minutes * 60);

    let snoozed_reminder = Reminder {
        id: payload.id,
        title: payload.title,
        body: payload.body,
        trigger_at_epoch: new_trigger,
        action_url: payload.action_url,
    };

    // Load active reminders, insert the new snoozed one, and save
    let mut reminders = reminder_core::load_reminders().unwrap_or_default();
    reminders.retain(|r| r.id != snoozed_reminder.id);
    reminders.push(snoozed_reminder);

    if let Err(e) = reminder_core::save_reminders(&reminders) {
        reminder_core::log_error!("Failed to save snoozed reminder: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save: {}", e),
        ));
    }

    // Wake up the scheduler loop to recalculate target timestamps
    let _ = state.tx.send(());
    Ok(StatusCode::OK)
}
