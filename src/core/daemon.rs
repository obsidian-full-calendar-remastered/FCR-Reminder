use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use tower_http::cors::{Any, CorsLayer};

use crate::core::api::{
    handle_doctor, handle_events, handle_next, handle_restart, handle_snooze, handle_start,
    handle_status, handle_stop, handle_storage, handle_sync, handle_updates,
    request_json_from_daemon, AppState,
};
use crate::core::commands::{
    execute_inspect_command, execute_lifecycle_command, handle_protocol_uri, parse_inspect_command,
    perform_complete_cleanup, print_help, InspectCommand, LifecycleCommand,
};
use crate::core::release_updates::{
    ReleaseUpdateService, UpdateRefreshResult, UpdateStateSnapshot,
};
use crate::core::scheduler::run_scheduler;
use crate::core::storage::{get_app_dir, get_storage_path};
use crate::{log_error, log_info};

#[tokio::main(flavor = "current_thread")]
pub async fn run_daemon() {
    // 1. Check for command-line arguments to handle options
    let args: Vec<String> = std::env::args().collect();

    let mut is_cleanup = false;
    let mut is_help = false;
    let mut needs_console = false;
    let mut is_gui = false;
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
            "--gui" | "--view" => {
                is_gui = true;
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
            "--updates" => {
                inspect_command = Some(InspectCommand::Updates);
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
                    crate::platform::prepare_console_for_cli();
                    eprintln!("Missing value for --inspect option");
                    std::process::exit(1);
                }
            }
            "--uri" | "-u" => {
                if i + 1 < args.len() {
                    uri_arg = Some(args[i + 1].clone());
                    skip_next = true;
                } else {
                    crate::platform::prepare_console_for_cli();
                    eprintln!("Missing value for --uri option");
                    std::process::exit(1);
                }
            }
            other => {
                if other.starts_with("fcr-reminder://") {
                    uri_arg = Some(other.to_string());
                } else {
                    crate::platform::prepare_console_for_cli();
                    eprintln!("Unknown argument: {}", other);
                    print_help();
                    std::process::exit(1);
                }
            }
        }
    }

    if needs_console {
        crate::platform::prepare_console_for_cli();
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

    if is_gui {
        crate::core::open_event_viewer();
        std::process::exit(0);
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], 45677));
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => listener,
        Err(e) => {
            if request_json_from_daemon("/status").is_ok() {
                log_info!(
                    "FCR Reminder is already running on 127.0.0.1:45677. Reusing the active instance."
                );
                std::process::exit(0);
            }
            log_error!(
                "CRITICAL ERROR: Failed to bind to 127.0.0.1:45677. Is another instance running?"
            );
            log_error!("Error: {}", e);
            std::process::exit(1);
        }
    };

    log_info!("=== starting full-calendar-remastered reminder daemon ===");

    // 3. Extract assets and register platform integrations
    ensure_assets_extracted();

    if let Err(e) = crate::platform::init() {
        log_error!("Platform initialization failed: {}", e);
    }

    // Determine and display database storage path for transparency
    if let Some(path) = get_storage_path() {
        log_info!("Storage path: {}", path.to_string_lossy());
    } else {
        log_error!("Warning: Could not determine data storage path.");
    }

    let update_service = ReleaseUpdateService::new();
    let update_snapshot = std::sync::Arc::new(std::sync::Mutex::new(
        update_service
            .as_ref()
            .map(|service| service.load_snapshot())
            .unwrap_or_else(|error| UpdateStateSnapshot::unavailable(error.clone())),
    ));

    // 4. Create and initialize system tray icon on a dedicated OS thread
    run_tray_thread(std::sync::Arc::clone(&update_snapshot));

    // Spawn system tray menu click event handler thread
    // This blocks at the OS level on menu_rx.recv() to ensure 0% active idle CPU wakeups
    let menu_rx = tray_icon::menu::MenuEvent::receiver().clone();
    let menu_update_state = std::sync::Arc::clone(&update_snapshot);
    std::thread::spawn(move || {
        while let Ok(event) = menu_rx.recv() {
            match event.id.as_ref() {
                "events" => {
                    crate::core::open_event_viewer();
                }
                "info" => {
                    let snapshot = menu_update_state.lock().unwrap().clone();
                    if let Err(error) = crate::platform::show_about_dialog(&snapshot) {
                        log_error!("Failed to open About dialog: {}", error);
                    }
                }
                "update" => {
                    let snapshot = menu_update_state.lock().unwrap().clone();
                    if let Err(error) = crate::platform::open_url(&snapshot.action_url()) {
                        log_error!("Failed to open release page: {}", error);
                    }
                }
                "quit" => {
                    log_info!("Quit menu option clicked in system tray. Shutting down daemon...");
                    std::process::exit(0);
                }
                _ => {}
            }
        }
    });

    if let Ok(service) = update_service {
        let service = std::sync::Arc::new(service);
        let shared_snapshot = std::sync::Arc::clone(&update_snapshot);
        tokio::spawn(async move {
            loop {
                let refresh = service.refresh_if_due(false).await;
                apply_update_refresh(&service, &shared_snapshot, refresh);

                let next_check_at = shared_snapshot.lock().unwrap().next_check_at_epoch;
                let now_epoch = chrono::Utc::now().timestamp();
                let sleep_seconds = (next_check_at - now_epoch).max(60) as u64;
                tokio::time::sleep(std::time::Duration::from_secs(sleep_seconds)).await;
            }
        });
    } else if let Err(error) = update_service {
        log_error!("Failed to initialize release update service: {}", error);
    }

    // Create a watch channel to notify the scheduler of database updates
    let (tx, rx) = watch::channel(());
    let state = Arc::new(AppState {
        tx,
        fired_notifications: Arc::new(Mutex::new(Vec::new())),
        update_snapshot: Arc::clone(&update_snapshot),
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
        .route("/updates", get(handle_updates))
        .route("/lifecycle/start", post(handle_start))
        .route("/lifecycle/stop", post(handle_stop))
        .route("/lifecycle/restart", post(handle_restart))
        .route("/sync", post(handle_sync))
        .route("/snooze", post(handle_snooze))
        .with_state(state)
        .layer(cors);

    log_info!("HTTP Server listening on: http://{}", addr);

    if let Err(e) = axum::serve(listener, app).await {
        log_error!("HTTP Server error: {}", e);
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
fn run_tray_thread(update_snapshot: std::sync::Arc<std::sync::Mutex<UpdateStateSnapshot>>) {
    std::thread::spawn(move || {
        let initial_snapshot = update_snapshot.lock().unwrap().clone();
        let tray_menu = tray_icon::menu::Menu::new();
        let status_item = tray_icon::menu::MenuItem::new("Status: Running", false, None);
        let events_item =
            tray_icon::menu::MenuItem::with_id("events", "Active Reminders...", true, None);
        let info_item = tray_icon::menu::MenuItem::with_id("info", "Info", true, None);
        let update_item = tray_icon::menu::MenuItem::with_id(
            "update",
            &initial_snapshot.menu_label,
            initial_snapshot.update_available,
            None,
        );
        let quit_item = tray_icon::menu::MenuItem::with_id("quit", "Quit", true, None);

        let _ = tray_menu.append(&status_item);
        let _ = tray_menu.append(&events_item);
        let _ = tray_menu.append(&info_item);
        let _ = tray_menu.append(&update_item);
        let _ = tray_menu.append(&tray_icon::menu::PredefinedMenuItem::separator());
        let _ = tray_menu.append(&quit_item);

        let icon = load_tray_icon();
        let _tray_icon = tray_icon::TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("FCR Reminder")
            .with_icon(icon)
            .build()
            .unwrap();

        let mut last_menu_label = initial_snapshot.menu_label;
        let mut last_enabled = initial_snapshot.update_available;

        loop {
            let snapshot = update_snapshot.lock().unwrap().clone();
            if snapshot.menu_label != last_menu_label {
                update_item.set_text(&snapshot.menu_label);
                last_menu_label = snapshot.menu_label.clone();
            }

            if snapshot.update_available != last_enabled {
                update_item.set_enabled(snapshot.update_available);
                last_enabled = snapshot.update_available;
            }

            crate::platform::run_event_loop_once(std::time::Duration::from_millis(500));
        }
    });
}

fn apply_update_refresh(
    service: &ReleaseUpdateService,
    shared_snapshot: &std::sync::Arc<std::sync::Mutex<UpdateStateSnapshot>>,
    refresh: UpdateRefreshResult,
) {
    {
        let mut snapshot = shared_snapshot.lock().unwrap();
        *snapshot = refresh.snapshot.clone();
    }

    if let Some(release) = refresh.should_notify {
        match crate::platform::trigger_update_notification(&release) {
            Ok(()) => {
                log_info!(
                    "Update notification displayed for version {}.",
                    release.version
                );
                service.mark_notified(&release.version);
            }
            Err(error) => {
                log_error!(
                    "Failed to display update notification for version {}: {}",
                    release.version,
                    error
                );
            }
        }
    }
}

/// Extracts the embedded calendar/clock reminder icon to local AppData.
fn ensure_assets_extracted() {
    if let Some(app_dir) = get_app_dir() {
        let icon_path = app_dir.join("icon.png");

        let _ = std::fs::create_dir_all(&app_dir);

        let icon_bytes = include_bytes!("../../assets/icon.png");

        if !icon_path.exists() {
            if let Err(e) = std::fs::write(&icon_path, icon_bytes) {
                log_error!("Failed to extract app icon: {}", e);
            } else {
                log_info!("Successfully extracted premium reminder icon to AppData.");
            }
        }
    }
}
