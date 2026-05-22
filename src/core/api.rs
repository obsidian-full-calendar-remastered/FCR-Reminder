use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use crate::core::models::Reminder;
use crate::core::release_updates::UpdateStateSnapshot;
use crate::core::storage::{get_app_dir, get_storage_path, load_reminders, save_reminders};
use crate::core::scheduler::FiredNotification;
use crate::{log_info, log_error};
use std::net::TcpStream;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

pub struct AppState {
    pub tx: watch::Sender<()>,
    pub fired_notifications: Arc<Mutex<Vec<FiredNotification>>>,
    pub update_snapshot: Arc<Mutex<UpdateStateSnapshot>>,
}

#[derive(serde::Serialize)]
pub struct StorageDetails {
    pub app_dir: String,
    pub app_dir_url: String,
    pub storage_path: String,
    pub storage_url: String,
    pub storage_exists: bool,
}

#[derive(serde::Serialize)]
pub struct ReminderDiagnostics {
    pub id: String,
    pub title: String,
    pub body: String,
    pub trigger_at_epoch: i64,
    pub trigger_at_rfc3339: String,
    pub seconds_until_fire: i64,
    pub action_url: String,
}

#[derive(serde::Serialize)]
pub struct StatusResponse {
    pub status: &'static str,
    pub checked_at_epoch: i64,
    pub checked_at_rfc3339: String,
    pub active_reminders: usize,
    pub storage: StorageDetails,
    pub next_event: Option<ReminderDiagnostics>,
}

#[derive(serde::Serialize)]
pub struct NextEventResponse {
    pub status: &'static str,
    pub checked_at_epoch: i64,
    pub checked_at_rfc3339: String,
    pub next_event: Option<ReminderDiagnostics>,
}

#[derive(serde::Serialize)]
pub struct EventsResponse {
    pub status: &'static str,
    pub checked_at_epoch: i64,
    pub checked_at_rfc3339: String,
    pub storage: StorageDetails,
    pub events: Vec<ReminderDiagnostics>,
}

#[derive(serde::Serialize)]
pub struct StorageResponse {
    pub status: &'static str,
    pub checked_at_epoch: i64,
    pub checked_at_rfc3339: String,
    pub storage: StorageDetails,
    pub stored_reminders: usize,
}

#[derive(serde::Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub ok: bool,
}

#[derive(serde::Serialize)]
pub struct DoctorInstance {
    pub pid: u32,
    pub executable_path: String,
    pub running_from_path: String,
    pub server_url: String,
}

#[derive(serde::Serialize)]
pub struct DoctorResponse {
    pub status: &'static str,
    pub checked_at_epoch: i64,
    pub checked_at_rfc3339: String,
    pub instance: DoctorInstance,
    pub storage: StorageDetails,
    pub active_reminders: usize,
    pub next_event: Option<ReminderDiagnostics>,
    pub checks: Vec<DoctorCheck>,
}

pub type UpdateStatusResponse = UpdateStateSnapshot;

#[derive(serde::Deserialize)]
pub struct SnoozePayload {
    pub id: String,
    pub title: String,
    pub body: String,
    pub action_url: String,
    pub minutes: i64,
}

/// Endpoint to check daemon health and database stats.
pub async fn handle_status() -> Result<Json<StatusResponse>, (StatusCode, String)> {
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

pub async fn handle_events() -> Result<Json<EventsResponse>, (StatusCode, String)> {
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

pub async fn handle_next() -> Result<Json<NextEventResponse>, (StatusCode, String)> {
    let reminders = load_sorted_reminders()?;
    let now_epoch = chrono::Utc::now().timestamp();

    Ok(Json(NextEventResponse {
        status: "running",
        checked_at_epoch: now_epoch,
        checked_at_rfc3339: format_timestamp(now_epoch),
        next_event: build_next_event(&reminders, now_epoch),
    }))
}

pub async fn handle_storage() -> Result<Json<StorageResponse>, (StatusCode, String)> {
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

pub async fn handle_doctor() -> Result<Json<DoctorResponse>, (StatusCode, String)> {
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

pub async fn handle_updates(
    State(state): State<Arc<AppState>>,
) -> Result<Json<UpdateStatusResponse>, (StatusCode, String)> {
    let snapshot = state
        .update_snapshot
        .lock()
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read update state snapshot".to_string(),
            )
        })?
        .clone();

    Ok(Json(snapshot))
}

pub async fn handle_start() -> Result<StatusCode, (StatusCode, String)> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn handle_stop() -> Result<StatusCode, (StatusCode, String)> {
    schedule_process_exit(None);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn handle_restart() -> Result<StatusCode, (StatusCode, String)> {
    let current_exe = std::env::current_exe()
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to locate current executable: {}", error)))?;
    schedule_process_exit(Some(current_exe));
    Ok(StatusCode::NO_CONTENT)
}

/// Endpoint to receive standard reminder synchronization payloads from Obsidian.
pub async fn handle_sync(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Vec<Reminder>>,
) -> Result<StatusCode, (StatusCode, String)> {
    log_info!(
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
                    log_info!(
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

    if let Err(e) = save_reminders(&filtered_payload) {
        log_error!("Failed to save reminders: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save: {}", e),
        ));
    }

    // Wake up the scheduler loop to recalculate target timestamps
    let _ = state.tx.send(());
    Ok(StatusCode::OK)
}

/// Endpoint exposing local loopback snooze commands.
pub async fn handle_snooze(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SnoozePayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    log_info!(
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
    let mut reminders = load_reminders().unwrap_or_default();
    reminders.retain(|r| r.id != snoozed_reminder.id);
    reminders.push(snoozed_reminder);

    if let Err(e) = save_reminders(&reminders) {
        log_error!("Failed to save snoozed reminder: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save: {}", e),
        ));
    }

    // Wake up the scheduler loop to recalculate target timestamps
    let _ = state.tx.send(());
    Ok(StatusCode::OK)
}

pub fn load_sorted_reminders() -> Result<Vec<Reminder>, (StatusCode, String)> {
    let mut reminders = load_reminders()
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;
    reminders.sort_by_key(|reminder| reminder.trigger_at_epoch);
    Ok(reminders)
}

pub fn build_storage_details() -> Result<StorageDetails, (StatusCode, String)> {
    let app_dir = get_app_dir().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not determine application data directory".to_string(),
        )
    })?;
    let storage_path = get_storage_path().ok_or_else(|| {
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

pub fn build_next_event(reminders: &[Reminder], now_epoch: i64) -> Option<ReminderDiagnostics> {
    reminders
        .iter()
        .find(|reminder| reminder.trigger_at_epoch > now_epoch)
        .map(|reminder| build_reminder_diagnostics(reminder, now_epoch))
}

pub fn build_reminder_diagnostics(reminder: &Reminder, now_epoch: i64) -> ReminderDiagnostics {
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

pub fn format_timestamp(epoch: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(epoch, 0)
        .map(|timestamp| timestamp.to_rfc3339())
        .unwrap_or_else(|| format!("invalid-epoch:{}", epoch))
}

pub fn path_to_file_url(path: &Path) -> Result<String, String> {
    url::Url::from_file_path(path)
        .map(|url| url.to_string())
        .map_err(|_| format!("Failed to convert '{}' to a file URL", path.display()))
}

fn build_doctor_checks(reminders: &[Reminder]) -> Vec<DoctorCheck> {
    let mut checks = vec![
        DoctorCheck {
            name: "storage_path_resolved".to_string(),
            ok: get_storage_path().is_some(),
        },
        DoctorCheck {
            name: "storage_file_present".to_string(),
            ok: get_storage_path()
                .map(|path| path.exists())
                .unwrap_or(false),
        },
        DoctorCheck {
            name: "icon_asset_extracted".to_string(),
            ok: get_app_dir()
                .map(|dir| dir.join("icon.png").exists())
                .unwrap_or(false),
        },
        DoctorCheck {
            name: "scheduler_data_loaded".to_string(),
            ok: !reminders.is_empty() || get_storage_path().is_some(),
        },
        DoctorCheck {
            name: "loopback_server_expected".to_string(),
            ok: true,
        },
    ];

    checks.extend(
        crate::platform::doctor_checks()
            .into_iter()
            .map(|(name, ok)| DoctorCheck {
                name: name.to_string(),
                ok,
            }),
    );

    checks
}

pub fn schedule_process_exit(restart_exe: Option<std::path::PathBuf>) {
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

pub fn request_json_from_daemon(path: &str) -> Result<serde_json::Value, String> {
    let response = send_loopback_request("GET", path, None)?;
    let (_, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "Daemon response did not include an HTTP body".to_string())?;

    serde_json::from_str(body.trim())
        .map_err(|error| format!("Failed to parse daemon response body: {}", error))
}

pub fn send_loopback_request(method: &str, path: &str, body: Option<&str>) -> Result<String, String> {
    use std::io::{Read, Write};
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn handle_updates_returns_shared_snapshot() {
        let (tx, _) = watch::channel(());
        let expected = UpdateStateSnapshot::unavailable("network blocked");
        let state = Arc::new(AppState {
            tx,
            fired_notifications: Arc::new(Mutex::new(Vec::new())),
            update_snapshot: Arc::new(Mutex::new(expected.clone())),
        });

        let Json(snapshot) = handle_updates(State(state)).await.unwrap();
        assert_eq!(snapshot, expected);
    }
}
