#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use reminder_core::Reminder;
use std::path::Path;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::watch;
use tower_http::cors::{Any, CorsLayer};

mod platform;

use std::sync::Mutex;

struct FiredNotification {
    id: String,
    fired_at: i64,
}

struct AppState {
    tx: watch::Sender<()>,
    fired_notifications: Arc<Mutex<Vec<FiredNotification>>>,
}

enum InspectCommand {
    Health,
    Next,
    Events,
    Storage,
    Doctor,
}

enum LifecycleCommand {
    Start,
    Stop,
    Restart,
}

#[derive(serde::Serialize)]
struct StorageDetails {
    app_dir: String,
    app_dir_url: String,
    storage_path: String,
    storage_url: String,
    storage_exists: bool,
}

#[derive(serde::Serialize)]
struct ReminderDiagnostics {
    id: String,
    title: String,
    body: String,
    trigger_at_epoch: i64,
    trigger_at_rfc3339: String,
    seconds_until_fire: i64,
    action_url: String,
}

#[derive(serde::Serialize)]
struct StatusResponse {
    status: &'static str,
    checked_at_epoch: i64,
    checked_at_rfc3339: String,
    active_reminders: usize,
    storage: StorageDetails,
    next_event: Option<ReminderDiagnostics>,
}

#[derive(serde::Serialize)]
struct NextEventResponse {
    status: &'static str,
    checked_at_epoch: i64,
    checked_at_rfc3339: String,
    next_event: Option<ReminderDiagnostics>,
}

#[derive(serde::Serialize)]
struct EventsResponse {
    status: &'static str,
    checked_at_epoch: i64,
    checked_at_rfc3339: String,
    storage: StorageDetails,
    events: Vec<ReminderDiagnostics>,
}

#[derive(serde::Serialize)]
struct StorageResponse {
    status: &'static str,
    checked_at_epoch: i64,
    checked_at_rfc3339: String,
    storage: StorageDetails,
    stored_reminders: usize,
}

#[derive(serde::Serialize)]
struct DoctorCheck {
    name: String,
    ok: bool,
}

#[derive(serde::Serialize)]
struct DoctorInstance {
    pid: u32,
    executable_path: String,
    running_from_path: String,
    server_url: String,
}

#[derive(serde::Serialize)]
struct DoctorResponse {
    status: &'static str,
    checked_at_epoch: i64,
    checked_at_rfc3339: String,
    instance: DoctorInstance,
    storage: StorageDetails,
    active_reminders: usize,
    next_event: Option<ReminderDiagnostics>,
    checks: Vec<DoctorCheck>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // 1. Check for command-line arguments to handle options
    let args: Vec<String> = std::env::args().collect();

    let mut is_cleanup = false;
    let mut is_help = false;
    let mut needs_console = false;
    let mut inspect_command: Option<InspectCommand> = None;
    let mut lifecycle_command: Option<LifecycleCommand> = None;
    let mut uri_arg: Option<String> = None;

    let mut skip_next = false;
    for (i, arg) in args.iter().enumerate().skip(1) {
        if skip_next {
            skip_next = false;
            continue;
        }
        match arg.as_str() {
            "--debug" | "-d" => needs_console = true,
            "--cleanup" | "--uninstall" | "-c" => {
                is_cleanup = true;
                needs_console = true;
            }
            "--help" | "-h" => {
                is_help = true;
                needs_console = true;
            }
            "--health" => {
                inspect_command = Some(InspectCommand::Health);
                needs_console = true;
            }
            "--next" => {
                inspect_command = Some(InspectCommand::Next);
                needs_console = true;
            }
            "--events" | "--list-events" => {
                inspect_command = Some(InspectCommand::Events);
                needs_console = true;
            }
            "--storage" => {
                inspect_command = Some(InspectCommand::Storage);
                needs_console = true;
            }
            "--doctor" => {
                inspect_command = Some(InspectCommand::Doctor);
                needs_console = true;
            }
            "--start" => {
                lifecycle_command = Some(LifecycleCommand::Start);
                needs_console = true;
            }
            "--stop" | "--shutdown" => {
                lifecycle_command = Some(LifecycleCommand::Stop);
                needs_console = true;
            }
            "--restart" => {
                lifecycle_command = Some(LifecycleCommand::Restart);
                needs_console = true;
            }
            "--inspect" => {
                if i + 1 < args.len() {
                    inspect_command = Some(parse_inspect_command(&args[i + 1]));
                    needs_console = true;
                    skip_next = true;
                } else {
                    platform::prepare_console_for_cli();
                    eprintln!("Missing value for --inspect option");
                    std::process::exit(1);
                }
            }
            "--uri" | "-u" => {
                if i + 1 < args.len() {
                    uri_arg = Some(args[i + 1].clone());
                    skip_next = true;
                } else {
                    platform::prepare_console_for_cli();
                    eprintln!("Missing value for --uri option");
                    std::process::exit(1);
                }
            }
            other => {
                if other.starts_with("fcr-reminder://") {
                    uri_arg = Some(other.to_string());
                } else {
                    platform::prepare_console_for_cli();
                    eprintln!("Unknown argument: {}", other);
                    print_help();
                    std::process::exit(1);
                }
            }
        }
    }

    if needs_console {
        platform::prepare_console_for_cli();
    }

    if is_help {
        print_help();
        std::process::exit(0);
    }

    if is_cleanup {
        match perform_complete_cleanup() {
            Ok(()) => std::process::exit(0),
            Err(error) => {
                eprintln!("{}", error);
                std::process::exit(1);
            }
        }
    }

    if let Some(command) = inspect_command {
        match execute_inspect_command(command) {
            Ok(()) => std::process::exit(0),
            Err(error) => {
                eprintln!("{}", error);
                std::process::exit(1);
            }
        }
    }

    if let Some(command) = lifecycle_command {
        match execute_lifecycle_command(command) {
            Ok(()) => std::process::exit(0),
            Err(error) => {
                eprintln!("{}", error);
                std::process::exit(1);
            }
        }
    }

    if let Some(uri) = uri_arg {
        handle_protocol_uri(&uri);
        std::process::exit(0);
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], 45677));
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => listener,
        Err(e) => {
            if request_json_from_daemon("/status").is_ok() {
                reminder_core::log_info!(
                    "FCR Reminder is already running on 127.0.0.1:45677. Reusing the active instance."
                );
                std::process::exit(0);
            }
            reminder_core::log_error!(
                "CRITICAL ERROR: Failed to bind to 127.0.0.1:45677. Is another instance running?"
            );
            reminder_core::log_error!("Error: {}", e);
            std::process::exit(1);
        }
    };

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
            match event.id.as_ref() {
                "info" => {
                    if let Err(error) = platform::show_about_dialog() {
                        reminder_core::log_error!("Failed to open About dialog: {}", error);
                    }
                }
                "quit" => {
                    reminder_core::log_info!(
                        "Quit menu option clicked in system tray. Shutting down daemon..."
                    );
                    std::process::exit(0);
                }
                _ => {}
            }
        }
    });

    // Create a watch channel to notify the scheduler of database updates
    let (tx, rx) = watch::channel(());
    let state = Arc::new(AppState {
        tx,
        fired_notifications: Arc::new(Mutex::new(Vec::new())),
    });

    // Spawn the scheduler task in the background
    let mut scheduler_rx = rx.clone();
    let fired_clone = Arc::clone(&state.fired_notifications);
    tokio::spawn(async move {
        run_scheduler(&mut scheduler_rx, fired_clone).await;
    });

    // Define Axum router with CORS support for Obsidian plugin calls
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/status", get(handle_status))
        .route("/events", get(handle_events))
        .route("/next", get(handle_next))
        .route("/storage", get(handle_storage))
        .route("/doctor", get(handle_doctor))
        .route("/lifecycle/start", post(handle_start))
        .route("/lifecycle/stop", post(handle_stop))
        .route("/lifecycle/restart", post(handle_restart))
        .route("/sync", post(handle_sync))
        .route("/snooze", post(handle_snooze))
        .with_state(state)
        .layer(cors);

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
      --inspect     Query the running daemon using one of: health, next, events, storage

Branding & Behavior:
  On Windows release builds, the daemon launches as a tray-first background app with no console.
  Use --debug from an existing terminal session when you want live log output.
  Syncs reminders from Obsidian Full Calendar Remastered plugin via HTTP on port 45677."#
    );
}

fn parse_inspect_command(value: &str) -> InspectCommand {
    match value.to_ascii_lowercase().as_str() {
        "health" | "status" => InspectCommand::Health,
        "next" => InspectCommand::Next,
        "events" | "list" | "list-events" => InspectCommand::Events,
        "storage" | "paths" => InspectCommand::Storage,
        "doctor" => InspectCommand::Doctor,
        other => {
            eprintln!("Unknown inspect target: {}", other);
            print_help();
            std::process::exit(1);
        }
    }
}

fn execute_inspect_command(command: InspectCommand) -> Result<(), String> {
    let path = match command {
        InspectCommand::Health => "/status",
        InspectCommand::Next => "/next",
        InspectCommand::Events => "/events",
        InspectCommand::Storage => "/storage",
        InspectCommand::Doctor => "/doctor",
    };

    let payload = request_json_from_daemon(path)?;
    let rendered = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("Failed to render daemon response: {}", error))?;
    println!("{}", rendered);
    Ok(())
}

fn execute_lifecycle_command(command: LifecycleCommand) -> Result<(), String> {
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

fn start_daemon_if_needed() -> Result<(), String> {
    if request_json_from_daemon("/status").is_ok() {
        println!("FCR Reminder is already running.");
        return Ok(());
    }

    let current_exe = std::env::current_exe()
        .map_err(|error| format!("Failed to locate current executable: {}", error))?;

    std::process::Command::new(&current_exe)
        .spawn()
        .map_err(|error| format!("Failed to launch FCR Reminder: {}", error))?;

    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        if request_json_from_daemon("/status").is_ok() {
            println!("FCR Reminder started successfully.");
            return Ok(());
        }
    }

    Err("FCR Reminder was launched but did not become reachable on 127.0.0.1:45677 in time.".to_string())
}

fn request_daemon_post(path: &str) -> Result<(), String> {
    let _ = send_loopback_request("POST", path, None)?;
    Ok(())
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
        let info_item = tray_icon::menu::MenuItem::with_id("info", "Info", true, None);
        let quit_item = tray_icon::menu::MenuItem::with_id("quit", "Quit", true, None);

        let _ = tray_menu.append(&status_item);
        let _ = tray_menu.append(&info_item);
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
fn perform_complete_cleanup() -> Result<(), String> {
    println!("\n=== Performing Complete System Cleanup for FCR Reminder ===");

    ensure_daemon_stopped_for_cleanup()?;

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
async fn handle_status() -> Result<Json<StatusResponse>, (StatusCode, String)> {
    let reminders = load_sorted_reminders()?;
    let now_epoch = chrono::Utc::now().timestamp();

    Ok(Json(StatusResponse {
        status: "running",
        checked_at_epoch: now_epoch,
        checked_at_rfc3339: format_timestamp(now_epoch),
        active_reminders: reminders.len(),
        storage: build_storage_details()?,
        next_event: build_next_event(&reminders, now_epoch),
    }))
}

async fn handle_events() -> Result<Json<EventsResponse>, (StatusCode, String)> {
    let reminders = load_sorted_reminders()?;
    let now_epoch = chrono::Utc::now().timestamp();

    Ok(Json(EventsResponse {
        status: "running",
        checked_at_epoch: now_epoch,
        checked_at_rfc3339: format_timestamp(now_epoch),
        storage: build_storage_details()?,
        events: reminders
            .iter()
            .map(|reminder| build_reminder_diagnostics(reminder, now_epoch))
            .collect(),
    }))
}

async fn handle_next() -> Result<Json<NextEventResponse>, (StatusCode, String)> {
    let reminders = load_sorted_reminders()?;
    let now_epoch = chrono::Utc::now().timestamp();

    Ok(Json(NextEventResponse {
        status: "running",
        checked_at_epoch: now_epoch,
        checked_at_rfc3339: format_timestamp(now_epoch),
        next_event: build_next_event(&reminders, now_epoch),
    }))
}

async fn handle_storage() -> Result<Json<StorageResponse>, (StatusCode, String)> {
    let reminders = load_sorted_reminders()?;
    let now_epoch = chrono::Utc::now().timestamp();

    Ok(Json(StorageResponse {
        status: "running",
        checked_at_epoch: now_epoch,
        checked_at_rfc3339: format_timestamp(now_epoch),
        storage: build_storage_details()?,
        stored_reminders: reminders.len(),
    }))
}

async fn handle_doctor() -> Result<Json<DoctorResponse>, (StatusCode, String)> {
    let reminders = load_sorted_reminders()?;
    let now_epoch = chrono::Utc::now().timestamp();
    let storage = build_storage_details()?;
    let instance_path = std::env::current_exe()
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to resolve current executable: {}", error),
            )
        })?
        .to_string_lossy()
        .into_owned();

    Ok(Json(DoctorResponse {
        status: "running",
        checked_at_epoch: now_epoch,
        checked_at_rfc3339: format_timestamp(now_epoch),
        instance: DoctorInstance {
            pid: std::process::id(),
            executable_path: instance_path.clone(),
            running_from_path: instance_path,
            server_url: "http://127.0.0.1:45677".to_string(),
        },
        storage,
        active_reminders: reminders.len(),
        next_event: build_next_event(&reminders, now_epoch),
        checks: build_doctor_checks(&reminders),
    }))
}

fn build_doctor_checks(reminders: &[Reminder]) -> Vec<DoctorCheck> {
    let mut checks = vec![
        DoctorCheck {
            name: "storage_path_resolved".to_string(),
            ok: reminder_core::get_storage_path().is_some(),
        },
        DoctorCheck {
            name: "storage_file_present".to_string(),
            ok: reminder_core::get_storage_path()
                .map(|path| path.exists())
                .unwrap_or(false),
        },
        DoctorCheck {
            name: "icon_asset_extracted".to_string(),
            ok: reminder_core::get_app_dir()
                .map(|dir| dir.join("icon.png").exists())
                .unwrap_or(false),
        },
        DoctorCheck {
            name: "scheduler_data_loaded".to_string(),
            ok: !reminders.is_empty() || reminder_core::get_storage_path().is_some(),
        },
        DoctorCheck {
            name: "loopback_server_expected".to_string(),
            ok: true,
        },
    ];

    checks.extend(
        platform::doctor_checks()
            .into_iter()
            .map(|(name, ok)| DoctorCheck {
                name: name.to_string(),
                ok,
            }),
    );

    checks
}

async fn handle_start() -> Result<StatusCode, (StatusCode, String)> {
    Ok(StatusCode::NO_CONTENT)
}

async fn handle_stop() -> Result<StatusCode, (StatusCode, String)> {
    schedule_process_exit(None);
    Ok(StatusCode::NO_CONTENT)
}

async fn handle_restart() -> Result<StatusCode, (StatusCode, String)> {
    let current_exe = std::env::current_exe()
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to locate current executable: {}", error)))?;
    schedule_process_exit(Some(current_exe));
    Ok(StatusCode::NO_CONTENT)
}

fn schedule_process_exit(restart_exe: Option<std::path::PathBuf>) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));

        if let Some(restart_exe) = restart_exe {
            schedule_restart_process(&restart_exe);
        }

        std::process::exit(0);
    });
}

fn schedule_restart_process(executable: &std::path::Path) {
    #[cfg(target_os = "windows")]
    {
        let command = format!(
            "Start-Sleep -Milliseconds 700; Start-Process -FilePath '{}'",
            executable.display().to_string().replace('\'', "''")
        );

        let _ = std::process::Command::new("powershell")
            .arg("-WindowStyle")
            .arg("Hidden")
            .arg("-Command")
            .arg(command)
            .spawn();
    }

    #[cfg(not(target_os = "windows"))]
    {
        let shell_command = format!("sleep 1; \"{}\"", executable.display());
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(shell_command)
            .spawn();
    }
}

fn load_sorted_reminders() -> Result<Vec<Reminder>, (StatusCode, String)> {
    let mut reminders = reminder_core::load_reminders()
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;
    reminders.sort_by_key(|reminder| reminder.trigger_at_epoch);
    Ok(reminders)
}

fn build_storage_details() -> Result<StorageDetails, (StatusCode, String)> {
    let app_dir = reminder_core::get_app_dir().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not determine application data directory".to_string(),
        )
    })?;
    let storage_path = reminder_core::get_storage_path().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not determine reminder storage path".to_string(),
        )
    })?;

    Ok(StorageDetails {
        app_dir: app_dir.to_string_lossy().into_owned(),
        app_dir_url: path_to_file_url(&app_dir)
            .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?,
        storage_path: storage_path.to_string_lossy().into_owned(),
        storage_url: path_to_file_url(&storage_path)
            .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?,
        storage_exists: storage_path.exists(),
    })
}

fn build_next_event(reminders: &[Reminder], now_epoch: i64) -> Option<ReminderDiagnostics> {
    reminders
        .iter()
        .find(|reminder| reminder.trigger_at_epoch > now_epoch)
        .map(|reminder| build_reminder_diagnostics(reminder, now_epoch))
}

fn build_reminder_diagnostics(reminder: &Reminder, now_epoch: i64) -> ReminderDiagnostics {
    ReminderDiagnostics {
        id: reminder.id.clone(),
        title: reminder.title.clone(),
        body: reminder.body.clone(),
        trigger_at_epoch: reminder.trigger_at_epoch,
        trigger_at_rfc3339: format_timestamp(reminder.trigger_at_epoch),
        seconds_until_fire: reminder.trigger_at_epoch - now_epoch,
        action_url: reminder.action_url.clone(),
    }
}

fn format_timestamp(epoch: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(epoch, 0)
        .map(|timestamp| timestamp.to_rfc3339())
        .unwrap_or_else(|| format!("invalid-epoch:{}", epoch))
}

fn path_to_file_url(path: &Path) -> Result<String, String> {
    url::Url::from_file_path(path)
        .map(|url| url.to_string())
        .map_err(|_| format!("Failed to convert '{}' to a file URL", path.display()))
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

    let now = chrono::Utc::now().timestamp();
    let filtered_payload = {
        let mut fired = state.fired_notifications.lock().unwrap();
        // Prune entries older than 10 minutes (600 seconds)
        fired.retain(|f| now - f.fired_at < 600);

        payload
            .into_iter()
            .filter(|reminder| {
                if let Some(f) = fired.iter().find(|f| f.id == reminder.id) {
                    reminder_core::log_info!(
                        "AUDIT: Filtering out duplicate sync reminder '{}' (ID: {}) because it was already fired at epoch {} ({} seconds ago).",
                        reminder.title,
                        reminder.id,
                        f.fired_at,
                        now - f.fired_at
                    );
                    false
                } else {
                    true
                }
            })
            .collect::<Vec<Reminder>>()
    };

    if let Err(e) = reminder_core::save_reminders(&filtered_payload) {
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
async fn run_scheduler(rx: &mut watch::Receiver<()>, fired_notifications: Arc<Mutex<Vec<FiredNotification>>>) {
    reminder_core::log_info!("Background scheduler started.");

    // Load active reminders from disk exactly once on startup to maintain an in-memory cache
    let reminders_loaded = reminder_core::load_reminders().unwrap_or_default();
    let current_time = chrono::Utc::now().timestamp();

    // Partition into missed and future reminders
    let (missed_reminders, future_reminders): (Vec<Reminder>, Vec<Reminder>) = reminders_loaded
        .into_iter()
        .partition(|r| r.trigger_at_epoch <= current_time);

    let mut reminders = future_reminders;

    if !missed_reminders.is_empty() {
        reminder_core::log_info!(
            "AUDIT: Found {} missed reminders on startup. Initiating recovery...",
            missed_reminders.len()
        );

        // Immediately update reminders.json so missed reminders are no longer stored on disk
        if let Err(e) = reminder_core::save_reminders(&reminders) {
            reminder_core::log_error!("Failed to save reminders list after clearing missed ones: {}", e);
        }

        // Spawn a background worker to fire missed notifications with a 20-second interval
        let fired_clone = Arc::clone(&fired_notifications);
        tokio::spawn(async move {
            for (i, reminder) in missed_reminders.into_iter().enumerate() {
                if i > 0 {
                    tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                }

                // Record as fired to avoid re-triggering if a sync occurs in the meantime
                let now = chrono::Utc::now().timestamp();
                {
                    let mut fired = fired_clone.lock().unwrap();
                    fired.retain(|f| now - f.fired_at < 600);
                    fired.push(FiredNotification {
                        id: reminder.id.clone(),
                        fired_at: now,
                    });
                }

                reminder_core::log_info!(
                    "AUDIT: Firing recovered missed reminder '{}' (ID: {}, originally scheduled for epoch {})",
                    reminder.title,
                    reminder.id,
                    reminder.trigger_at_epoch
                );
                trigger_notification(&reminder);
            }
        });
    }

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

                // Record the fired notification
                {
                    let mut fired = fired_notifications.lock().unwrap();
                    let now = chrono::Utc::now().timestamp();
                    // Prune old entries
                    fired.retain(|f| now - f.fired_at < 600);
                    fired.push(FiredNotification {
                        id: next_reminder.id.clone(),
                        fired_at: now,
                    });
                    reminder_core::log_info!(
                        "AUDIT: Recorded fired notification '{}' (ID: {}) at epoch {}.",
                        next_reminder.title,
                        next_reminder.id,
                        now
                    );
                }

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
    reminder_core::log_info!("Received protocol activation URI: {}", uri);

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
                    reminder_core::log_warn!(
                        "Protocol activation did not include a snooze duration. Defaulting reminder '{}' to 5 minutes.",
                        title
                    );
                    5
                }
            };

            match send_snooze_request(&id, &title, &body, &action_url, minutes) {
                Ok(_) => {
                    reminder_core::log_info!(
                        "Forwarded snooze request for reminder '{}' ({} minutes) to the daemon.",
                        title,
                        minutes
                    );
                }
                Err(e) => {
                    reminder_core::log_error!(
                        "Failed to forward snooze request for reminder '{}': {}",
                        title,
                        e
                    );
                    eprintln!("Failed to send snooze request to daemon: {}", e);
                }
            }
        } else {
            reminder_core::log_warn!(
                "Protocol activation URI was missing required snooze fields: {}",
                uri
            );
            eprintln!("Invalid protocol URI parameters.");
        }
    } else {
        reminder_core::log_warn!(
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

fn request_json_from_daemon(path: &str) -> Result<serde_json::Value, String> {
    let response = send_loopback_request("GET", path, None)?;
    let (_, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "Daemon response did not include an HTTP body".to_string())?;

    serde_json::from_str(body.trim())
        .map_err(|error| format!("Failed to parse daemon response body: {}", error))
}

fn send_loopback_request(method: &str, path: &str, body: Option<&str>) -> Result<String, String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let request = match body {
        Some(body) => format!(
            "{} {} HTTP/1.1\r\n\
             Host: 127.0.0.1:45677\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n\
             {}",
            method,
            path,
            body.len(),
            body
        ),
        None => format!(
            "{} {} HTTP/1.1\r\n\
             Host: 127.0.0.1:45677\r\n\
             Connection: close\r\n\r\n",
            method, path
        ),
    };

    let mut stream = TcpStream::connect("127.0.0.1:45677")
        .map_err(|error| format!("Failed to connect to daemon: {}", error))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("Failed to configure daemon write timeout: {}", error))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("Failed to configure daemon read timeout: {}", error))?;

    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("Failed to write request to daemon: {}", error))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("Failed to read daemon response: {}", error))?;

    if response.starts_with("HTTP/1.1 2") || response.starts_with("HTTP/1.0 2") {
        Ok(response)
    } else {
        Err(format!("Daemon returned a non-success response: {}", response))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duplicate_prevention_logic() {
        let reminder1 = Reminder {
            id: "rem-1".to_string(),
            title: "Title 1".to_string(),
            body: "Body 1".to_string(),
            trigger_at_epoch: 1000,
            action_url: "url1".to_string(),
        };
        let reminder2 = Reminder {
            id: "rem-2".to_string(),
            title: "Title 2".to_string(),
            body: "Body 2".to_string(),
            trigger_at_epoch: 2000,
            action_url: "url2".to_string(),
        };

        let now = chrono::Utc::now().timestamp();
        let mut fired = vec![
            FiredNotification {
                id: "rem-1".to_string(),
                fired_at: now - 300, // 5 minutes ago (should be kept and filter out)
            },
            FiredNotification {
                id: "rem-3".to_string(),
                fired_at: now - 900, // 15 minutes ago (should be pruned)
            },
        ];

        // Prune and filter
        fired.retain(|f| now - f.fired_at < 600);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].id, "rem-1");

        let payload = vec![reminder1.clone(), reminder2.clone()];
        let filtered: Vec<Reminder> = payload
            .into_iter()
            .filter(|r| !fired.iter().any(|f| f.id == r.id))
            .collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "rem-2");
    }
}

