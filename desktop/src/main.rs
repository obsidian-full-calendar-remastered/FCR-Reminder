use std::net::SocketAddr;
use std::sync::Arc;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use tower_http::cors::{Any, CorsLayer};
use tokio::sync::watch;
use reminder_core::Reminder;

struct AppState {
    tx: watch::Sender<()>,
}

#[tokio::main]
async fn main() {
    println!("=== starting full-calendar-remastered reminder daemon ===");

    // Extract embedded assets (like the premium icon) on startup
    ensure_assets_extracted();

    // Register custom AppUserModelId in Windows Registry so notifications look beautiful
    #[cfg(target_os = "windows")]
    register_custom_app_id();

    // Determine and display database storage path for transparency
    if let Some(path) = reminder_core::get_storage_path() {
        println!("Storage path: {}", path.to_string_lossy());
    } else {
        eprintln!("Warning: Could not determine data storage path.");
    }

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
            eprintln!("CRITICAL ERROR: Failed to bind to 127.0.0.1:45677. Is another instance running?");
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    println!("HTTP Server listening on: http://{}", addr);
    
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("HTTP Server error: {}", e);
    }
}

/// Extracts the embedded calendar/clock reminder icon to local AppData.
/// This allows the .exe to remain completely self-contained with no external resource dependencies.
fn ensure_assets_extracted() {
    if let Some(app_dir) = reminder_core::get_app_dir() {
        let icon_path = app_dir.join("icon.png");
        
        // Ensure application directories exist
        let _ = std::fs::create_dir_all(&app_dir);
        
        // Embed the generated high-quality icon inside the compiled binary
        let icon_bytes = include_bytes!("../../assets/icon.png");
        
        // Extract and write to local AppData if missing
        if !icon_path.exists() {
            if let Err(e) = std::fs::write(&icon_path, icon_bytes) {
                eprintln!("Warning: Failed to extract app icon: {}", e);
            } else {
                println!("Successfully extracted premium reminder icon to AppData.");
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
                println!("Registered custom AppUserModelId 'FCRReminder' in Windows Registry.");
            }
            Err(e) => {
                eprintln!("Warning: Failed to create custom Registry AppId subkey: {}", e);
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
    println!("Received Sync request: {} reminders provided.", payload.len());

    if let Err(e) = reminder_core::save_reminders(&payload) {
        eprintln!("Failed to save reminders: {}", e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save: {}", e)));
    }

    // Wake up the scheduler loop to recalculate target timestamps
    let _ = state.tx.send(());
    Ok(StatusCode::OK)
}

/// Core background scheduler loop.
/// Sleeps until the next active reminder, woke up instantly on sync updates.
async fn run_scheduler(rx: &mut watch::Receiver<()>) {
    println!("Background scheduler started.");

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
            println!("No active future reminders. Sleeping until next synchronization.");
            // Sleep indefinitely until we receive a wakeup signal
            if rx.changed().await.is_err() {
                break; // Watch channel closed, terminate scheduler
            }
            continue;
        }

        let next_reminder = active[0].clone();
        let delay_secs = next_reminder.trigger_at_epoch - current_time;
        
        println!(
            "Next reminder scheduled: \"{}\" in {} seconds (at Epoch {}).",
            next_reminder.title, delay_secs, next_reminder.trigger_at_epoch
        );

        let delay = std::time::Duration::from_secs(delay_secs.max(0) as u64);

        // Sleep until either the timer expires OR a sync signal occurs
        tokio::select! {
            _ = tokio::time::sleep(delay) => {
                println!("Reminder triggered! Firing notification for \"{}\".", next_reminder.title);
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
                println!("Synchronization signal received. Rescheduling reminders...");
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
            eprintln!("Windows Toast error: {:?}", e);
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
            eprintln!("Desktop notification error: {:?}", e);
        }
    }
}

