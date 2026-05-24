use crate::core::api::send_loopback_request;
use crate::core::gui::GuiContext;
use crate::core::release_updates::{ReleaseInfo, UpdateStateSnapshot};
use crate::core::Reminder;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const DESKTOP_FILE_NAME: &str = "fcr-reminder.desktop";
const MIME_SCHEME: &str = "x-scheme-handler/fcr-reminder";

/// Linux-specific startup initialization.
pub fn init() -> Result<(), Box<dyn Error>> {
    register_desktop_entry()?;
    register_autostart()?;
    register_custom_protocol()?;
    crate::log_info!("Linux initialization: XDG autostart and protocol handlers are configured.");
    Ok(())
}

/// Linux-specific cleanup/uninstallation.
pub fn cleanup() -> Result<(), Box<dyn Error>> {
    remove_file_if_exists(&autostart_desktop_path()?);
    remove_file_if_exists(&application_desktop_path()?);
    remove_mimeapps_scheme_registration()?;
    refresh_desktop_database();
    println!("Linux: Removed XDG autostart, desktop entry, and fcr-reminder protocol handler.");
    Ok(())
}

/// Console preparation on Linux (no-op).
pub fn prepare_console_for_cli() {}

/// Triggers a desktop notification using DBus. When the user's notification server
/// exposes actions, it mirrors the Windows notification actions: snooze or open note.
pub fn trigger_notification(reminder: &Reminder) -> Result<(), Box<dyn Error>> {
    let reminder = reminder.clone();

    std::thread::Builder::new()
        .name("fcr-reminder-linux-notification".to_string())
        .spawn(move || {
            if let Err(error) = show_reminder_notification(reminder) {
                crate::log_error!("Linux notification error: {}", error);
            }
        })
        .map(|_| ())
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

pub fn doctor_checks() -> Vec<(&'static str, bool)> {
    vec![
        (
            "linux_desktop_entry_registered",
            application_desktop_path()
                .map(|path| path.exists())
                .unwrap_or(false),
        ),
        (
            "linux_autostart_registered",
            autostart_desktop_path()
                .map(|path| path.exists())
                .unwrap_or(false),
        ),
        (
            "linux_protocol_registered",
            mimeapps_contains_scheme().unwrap_or(false),
        ),
        ("linux_xdg_open_available", command_available("xdg-open")),
    ]
}

pub fn show_about_dialog(update_state: &UpdateStateSnapshot) -> Result<(), Box<dyn Error>> {
    let repository_url = env!("CARGO_PKG_REPOSITORY");
    let issues_url = build_issues_url(repository_url);
    let feature_request_url = build_feature_request_url(repository_url);
    let executable_path = current_exe_display();
    let storage_path = crate::core::get_storage_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "Unavailable".to_string());
    let update_version = update_state
        .latest_release
        .as_ref()
        .map(|release| release.version.clone())
        .unwrap_or_else(|| "Unavailable".to_string());
    let update_checked = if update_state.last_checked_at_epoch > 0 {
        chrono::DateTime::<chrono::Utc>::from_timestamp(update_state.last_checked_at_epoch, 0)
            .map(|timestamp| timestamp.to_rfc3339())
            .unwrap_or_else(|| "Unavailable".to_string())
    } else {
        "Pending".to_string()
    };
    let update_button_text = if update_state.update_available {
        "Update Available"
    } else {
        "Latest Release"
    };

    let html = about_html(
        repository_url,
        &issues_url,
        &feature_request_url,
        &executable_path,
        &storage_path,
        &update_state.status_label,
        &update_version,
        &update_checked,
        &update_state.action_url(),
        update_button_text,
    );
    open_generated_html("about.html", &html)
}

pub fn trigger_update_notification(release: &ReleaseInfo) -> Result<(), Box<dyn Error>> {
    let release = release.clone();
    std::thread::Builder::new()
        .name("fcr-reminder-linux-update-notification".to_string())
        .spawn(move || {
            use notify_rust::Notification;

            match Notification::new()
                .appname("FCR Reminder")
                .summary("Update available for FCR Reminder")
                .body(&format!(
                    "Version {} is available. Open GitHub Releases to download it.",
                    release.version
                ))
                .icon("appointment-soon")
                .action("open", "Open Releases")
                .show()
            {
                Ok(handle) => {
                    handle.wait_for_action(|action| {
                        if action == "open" || action == "default" {
                            if let Err(error) = open_url(&release.html_url) {
                                crate::log_error!("Failed to open release page: {}", error);
                            }
                        }
                    });
                }
                Err(error) => crate::log_error!("Linux update notification error: {}", error),
            }
        })
        .map(|_| ())
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

pub fn open_url(url: &str) -> Result<(), Box<dyn Error>> {
    Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

/// Event loop for Linux system tray and scheduling thread.
pub fn run_event_loop_once(timeout: std::time::Duration) {
    std::thread::sleep(timeout);
}

pub fn show_events_dialog(ctx: &GuiContext) -> Result<(), Box<dyn Error>> {
    let html = events_html(ctx);
    open_generated_html("events-viewer.html", &html)
}

fn show_reminder_notification(reminder: Reminder) -> Result<(), Box<dyn Error>> {
    use notify_rust::Notification;

    let mut notification = Notification::new();
    notification
        .appname("FCR Reminder")
        .summary(&reminder.title)
        .body(&reminder.body)
        .icon("appointment-soon")
        .action("snooze_5", "Snooze 5 minutes")
        .action("snooze_10", "Snooze 10 minutes")
        .action("snooze_15", "Snooze 15 minutes")
        .action("snooze_30", "Snooze 30 minutes")
        .action("snooze_60", "Snooze 1 hour");

    if !reminder.action_url.trim().is_empty() {
        notification.action("open", "Open Note");
    }

    let handle = notification.show()?;
    handle.wait_for_action(|action| match action {
        "open" | "default" => {
            if !reminder.action_url.trim().is_empty() {
                if let Err(error) = open_url(&reminder.action_url) {
                    crate::log_error!("Failed to open reminder URL: {}", error);
                }
            }
        }
        "snooze_5" => snooze_reminder(&reminder, 5),
        "snooze_10" => snooze_reminder(&reminder, 10),
        "snooze_15" => snooze_reminder(&reminder, 15),
        "snooze_30" => snooze_reminder(&reminder, 30),
        "snooze_60" => snooze_reminder(&reminder, 60),
        _ => {}
    });

    Ok(())
}

fn snooze_reminder(reminder: &Reminder, minutes: i64) {
    let payload = serde_json::json!({
        "id": reminder.id,
        "title": reminder.title,
        "body": reminder.body,
        "action_url": reminder.action_url,
        "minutes": minutes,
    });

    match serde_json::to_string(&payload)
        .map_err(|error| error.to_string())
        .and_then(|body| send_loopback_request("POST", "/snooze", Some(&body)).map(|_| ()))
    {
        Ok(()) => crate::log_info!(
            "Forwarded Linux notification snooze request for '{}' ({} minutes).",
            reminder.title,
            minutes
        ),
        Err(error) => crate::log_error!(
            "Failed to snooze reminder from Linux notification: {}",
            error
        ),
    }
}

fn register_desktop_entry() -> Result<(), Box<dyn Error>> {
    let path = application_desktop_path()?;
    write_desktop_entry(&path, false)
}

fn register_autostart() -> Result<(), Box<dyn Error>> {
    let path = autostart_desktop_path()?;
    write_desktop_entry(&path, true)
}

fn register_custom_protocol() -> Result<(), Box<dyn Error>> {
    let path = mimeapps_path()?;
    let mut lines = if path.exists() {
        fs::read_to_string(&path)?
            .lines()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    upsert_mimeapps_default(&mut lines, MIME_SCHEME, DESKTOP_FILE_NAME);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, format!("{}\n", lines.join("\n")))?;
    refresh_desktop_database();
    Ok(())
}

fn write_desktop_entry(path: &Path, autostart: bool) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let current_exe = std::env::current_exe()?;
    let exec = if autostart {
        quote_desktop_exec(&current_exe)
    } else {
        format!("{} --uri %u", quote_desktop_exec(&current_exe))
    };
    let icon = crate::core::get_app_dir()
        .map(|dir| dir.join("icon.png"))
        .filter(|path| path.exists())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "appointment-soon".to_string());

    let content = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=FCR Reminder\n\
         Comment={}\n\
         Exec={}\n\
         Icon={}\n\
         Terminal=false\n\
         Categories=Utility;Office;Calendar;\n\
         StartupNotify=false\n\
         X-GNOME-Autostart-enabled=true\n\
         MimeType={};\n",
        env!("CARGO_PKG_DESCRIPTION"),
        exec,
        icon,
        MIME_SCHEME
    );

    let mut file = fs::File::create(path)?;
    file.write_all(content.as_bytes())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
    }

    Ok(())
}

fn quote_desktop_exec(path: &Path) -> String {
    let escaped = path
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

fn upsert_mimeapps_default(lines: &mut Vec<String>, scheme: &str, desktop_file: &str) {
    let section = "[Default Applications]";
    let entry = format!("{}={}", scheme, desktop_file);
    let mut in_section = false;
    let mut section_found = false;
    let mut insert_at = lines.len();

    for (index, line) in lines.iter_mut().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if in_section {
                insert_at = index;
                break;
            }
            in_section = trimmed == section;
            section_found |= in_section;
            continue;
        }

        if in_section && trimmed.starts_with(&format!("{}=", scheme)) {
            *line = entry;
            return;
        }
    }

    if !section_found {
        if !lines.is_empty() && lines.last().map(|line| !line.is_empty()).unwrap_or(false) {
            lines.push(String::new());
        }
        lines.push(section.to_string());
        lines.push(entry);
    } else {
        lines.insert(insert_at, entry);
    }
}

fn remove_mimeapps_scheme_registration() -> Result<(), Box<dyn Error>> {
    let path = mimeapps_path()?;
    if !path.exists() {
        return Ok(());
    }

    let filtered = fs::read_to_string(&path)?
        .lines()
        .filter(|line| !line.trim_start().starts_with(&format!("{}=", MIME_SCHEME)))
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    fs::write(&path, format!("{}\n", filtered.join("\n")))?;
    Ok(())
}

fn remove_file_if_exists(path: &Path) {
    if path.exists() {
        match fs::remove_file(path) {
            Ok(()) => println!("Linux: Removed {}", path.display()),
            Err(error) => eprintln!(
                "Linux Warning: Failed to remove {}: {}",
                path.display(),
                error
            ),
        }
    }
}

fn mimeapps_contains_scheme() -> Result<bool, Box<dyn Error>> {
    let path = mimeapps_path()?;
    if !path.exists() {
        return Ok(false);
    }

    Ok(fs::read_to_string(path)?
        .lines()
        .any(|line| line.trim() == format!("{}={}", MIME_SCHEME, DESKTOP_FILE_NAME)))
}

fn refresh_desktop_database() {
    if command_available("update-desktop-database") {
        if let Ok(applications_dir) = applications_dir() {
            let _ = Command::new("update-desktop-database")
                .arg(applications_dir)
                .spawn();
        }
    }
}

fn command_available(command: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!(
            "command -v {} >/dev/null 2>&1",
            shell_escape(command)
        ))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn open_generated_html(name: &str, html: &str) -> Result<(), Box<dyn Error>> {
    let app_dir = crate::core::get_app_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine application data directory",
        )
    })?;
    fs::create_dir_all(&app_dir)?;
    let path = app_dir.join(name);
    fs::write(&path, html)?;
    open_url(&path_to_file_url(&path)?)
}

fn path_to_file_url(path: &Path) -> Result<String, Box<dyn Error>> {
    url::Url::from_file_path(path)
        .map(|url| url.to_string())
        .map_err(|_| format!("Failed to convert '{}' to a file URL", path.display()).into())
}

fn config_home() -> Result<PathBuf, Box<dyn Error>> {
    if let Some(value) = std::env::var_os("XDG_CONFIG_HOME") {
        Ok(PathBuf::from(value))
    } else {
        Ok(home_dir()?.join(".config"))
    }
}

fn data_home() -> Result<PathBuf, Box<dyn Error>> {
    if let Some(value) = std::env::var_os("XDG_DATA_HOME") {
        Ok(PathBuf::from(value))
    } else {
        Ok(home_dir()?.join(".local").join("share"))
    }
}

fn home_dir() -> Result<PathBuf, Box<dyn Error>> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set; cannot resolve XDG paths".into())
}

fn applications_dir() -> Result<PathBuf, Box<dyn Error>> {
    Ok(data_home()?.join("applications"))
}

fn application_desktop_path() -> Result<PathBuf, Box<dyn Error>> {
    Ok(applications_dir()?.join(DESKTOP_FILE_NAME))
}

fn autostart_desktop_path() -> Result<PathBuf, Box<dyn Error>> {
    Ok(config_home()?.join("autostart").join(DESKTOP_FILE_NAME))
}

fn mimeapps_path() -> Result<PathBuf, Box<dyn Error>> {
    Ok(config_home()?.join("mimeapps.list"))
}

fn current_exe_display() -> String {
    std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "Unavailable".to_string())
}

fn build_issues_url(repository_url: &str) -> String {
    format!("{}/issues", repository_url.trim_end_matches('/'))
}

fn build_feature_request_url(repository_url: &str) -> String {
    format!("{}/issues/new/choose", repository_url.trim_end_matches('/'))
}

fn shell_escape(value: &str) -> String {
    value.replace('\'', "'\\''")
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn js_string(input: &str) -> String {
    serde_json::to_string(input).unwrap_or_else(|_| "\"\"".to_string())
}

fn about_html(
    repository_url: &str,
    issues_url: &str,
    feature_request_url: &str,
    executable_path: &str,
    storage_path: &str,
    update_status: &str,
    update_version: &str,
    update_checked: &str,
    update_url: &str,
    update_button_text: &str,
) -> String {
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>About FCR Reminder</title>
<style>
:root {{ color-scheme: light dark; --bg:#f6f7f9; --panel:#fff; --text:#1a1a1a; --muted:#606060; --accent:#0b66c3; --border:#dadce0; --button:#f0f4f8; }}
@media (prefers-color-scheme: dark) {{ :root {{ --bg:#18181b; --panel:#242428; --text:#f3f3f3; --muted:#b4b4b8; --accent:#60a5fa; --border:#464649; --button:#343438; }} }}
* {{ box-sizing: border-box; }}
body {{ margin:0; min-height:100vh; background:var(--bg); color:var(--text); font:14px/1.45 system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif; }}
main {{ max-width:900px; margin:0 auto; padding:18px; }}
.hero,.details {{ background:var(--panel); border:1px solid var(--border); }}
.hero {{ min-height:150px; padding:24px; display:grid; grid-template-columns:84px 1fr; gap:24px; align-items:start; }}
.icon {{ width:84px; height:84px; object-fit:contain; }}
h1 {{ margin:0 0 2px; color:var(--accent); font-size:28px; font-weight:650; letter-spacing:0; }}
.sub {{ color:var(--muted); overflow-wrap:anywhere; }}
.badge {{ display:inline-block; margin-top:12px; padding:3px 18px; background:var(--accent); color:white; font-weight:650; }}
.summary {{ margin-top:12px; color:var(--muted); }}
.details {{ margin-top:16px; padding:20px 24px; }}
h2 {{ margin:0 0 8px; font-size:16px; }}
.row {{ display:grid; grid-template-columns:126px 1fr; gap:10px; padding:6px 0; }}
.label {{ color:var(--muted); font-weight:650; }}
.value {{ overflow-wrap:anywhere; }}
.actions {{ margin-top:16px; display:flex; gap:12px; flex-wrap:wrap; align-items:center; }}
a,button {{ border:1px solid var(--border); background:var(--button); color:var(--text); text-decoration:none; padding:8px 16px; font-weight:650; cursor:pointer; }}
.primary {{ margin-left:auto; background:var(--accent); color:white; }}
@media (max-width:640px) {{ .hero {{ grid-template-columns:1fr; }} .row {{ grid-template-columns:1fr; }} .primary {{ margin-left:0; }} }}
</style>
</head>
<body>
<main>
<section class="hero">
<img class="icon" src="icon.png" alt="">
<div>
<h1>FCR Reminder</h1>
<div class="sub">{description}</div>
<div class="badge">Version {version}</div>
<div class="summary">Background companion for Full Calendar Remastered. Native tray app, local reminder storage, and loopback-only integration.</div>
</div>
</section>
<section class="details">
<h2>Application details</h2>
{rows}
</section>
<div class="actions">
<a href="{repository_url}">Repository</a>
<a href="{issues_url}">Issues</a>
<a href="{feature_request_url}">Feature Request</a>
<a href="{update_url}">{update_button_text}</a>
<button class="primary" onclick="window.close()">OK</button>
</div>
</main>
</body>
</html>"##,
        description = html_escape(env!("CARGO_PKG_DESCRIPTION")),
        version = html_escape(env!("CARGO_PKG_VERSION")),
        repository_url = html_escape(repository_url),
        issues_url = html_escape(issues_url),
        feature_request_url = html_escape(feature_request_url),
        update_url = html_escape(update_url),
        update_button_text = html_escape(update_button_text),
        rows = [
            ("Version", env!("CARGO_PKG_VERSION")),
            ("License", env!("CARGO_PKG_LICENSE")),
            ("Runtime", "Rust desktop tray daemon"),
            ("Executable", executable_path),
            ("Storage", storage_path),
            ("Update Status", update_status),
            ("Latest Release", update_version),
            ("Last Check", update_checked),
        ]
        .iter()
        .map(|(label, value)| format!(
            "<div class=\"row\"><div class=\"label\">{}</div><div class=\"value\">{}</div></div>",
            html_escape(label),
            html_escape(value)
        ))
        .collect::<String>()
    )
}

fn events_html(ctx: &GuiContext) -> String {
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>FCR Scheduled Reminders</title>
<style>
:root {{ color-scheme: light dark; --bg:#f4f5f7; --panel:#fff; --text:#18181b; --muted:#71717a; --accent:#2563eb; --accent-soft:#eff6ff; --badge:#1d4ed8; --border:#e4e4e7; --card:#d4d4d8; --danger:#ef4444; }}
@media (prefers-color-scheme: dark) {{ :root {{ --bg:#09090b; --panel:#18181b; --text:#fafafa; --muted:#a1a1aa; --accent:#60a5fa; --accent-soft:#1e293b; --badge:#bfdbfe; --border:#27272a; --card:#3f3f46; --danger:#f87171; }} }}
* {{ box-sizing:border-box; }}
body {{ margin:0; min-height:100vh; background:var(--bg); color:var(--text); font:14px/1.45 system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif; }}
header {{ height:84px; background:var(--panel); border-bottom:1px solid var(--border); display:flex; align-items:center; gap:14px; padding:15px 20px; }}
.icon {{ width:54px; height:54px; object-fit:contain; }}
h1 {{ margin:0; color:var(--accent); font-size:22px; font-weight:650; letter-spacing:0; }}
.subtitle,.status,.time,.body,.storage {{ color:var(--muted); }}
.toolbar {{ min-height:52px; display:flex; gap:10px; align-items:center; padding:12px 20px; flex-wrap:wrap; }}
input {{ width:260px; max-width:100%; padding:7px 9px; border:1px solid var(--border); background:var(--panel); color:var(--text); }}
button,a.button {{ border:1px solid var(--border); background:var(--panel); color:var(--text); padding:7px 12px; font-weight:650; text-decoration:none; cursor:pointer; min-height:32px; }}
.danger {{ color:var(--danger); }}
main {{ padding:10px 20px 54px; }}
.card {{ background:var(--panel); border:1px solid var(--card); margin:0 0 12px; padding:12px 16px; display:grid; grid-template-columns:1fr auto; gap:10px 16px; }}
.title {{ font-size:16px; font-weight:650; overflow-wrap:anywhere; }}
.body {{ min-height:38px; overflow-wrap:anywhere; }}
.badge {{ background:var(--accent-soft); color:var(--badge); font-weight:650; text-align:center; min-width:170px; padding:3px 10px; }}
.actions {{ display:flex; gap:8px; justify-content:flex-end; align-items:center; flex-wrap:wrap; }}
select {{ border:1px solid var(--border); background:var(--panel); color:var(--text); padding:7px 8px; min-height:32px; }}
footer {{ position:fixed; left:0; right:0; bottom:0; min-height:32px; background:var(--panel); border-top:1px solid var(--border); display:flex; gap:12px; justify-content:space-between; align-items:center; padding:7px 20px; }}
.empty,.error {{ min-height:120px; align-content:center; }}
.error .title {{ color:var(--danger); }}
@media (max-width:760px) {{ header {{ height:auto; align-items:flex-start; }} .card {{ grid-template-columns:1fr; }} .badge {{ width:max-content; min-width:0; }} .actions {{ justify-content:flex-start; }} footer {{ position:static; flex-direction:column; align-items:flex-start; }} }}
</style>
</head>
<body>
<header>
<img class="icon" src="icon.png" alt="">
<div>
<h1>Scheduled Calendar Reminders</h1>
<div class="subtitle">Active reminders synchronized from Obsidian. The background service manages desktop notifications.</div>
</div>
</header>
<section class="toolbar">
<input id="search" type="search" placeholder="Search reminders...">
<button id="refresh">Refresh</button>
<button id="test">Trigger Test</button>
</section>
<main id="events"></main>
<footer><div class="status" id="status">Connecting to daemon...</div><div class="storage" id="storage"></div></footer>
<script>
const API_URL = {api_url};
const EXECUTABLE = {executable};
let allReminders = [];
const eventsEl = document.querySelector("#events");
const statusEl = document.querySelector("#status");
const storageEl = document.querySelector("#storage");
const searchEl = document.querySelector("#search");
document.querySelector("#refresh").addEventListener("click", loadEvents);
document.querySelector("#test").addEventListener("click", triggerTestReminder);
searchEl.addEventListener("input", renderCards);

function escapeHtml(value) {{
  return String(value ?? "").replace(/[&<>"']/g, ch => ({{"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}}[ch]));
}}
function relativeTime(seconds) {{
  const abs = Math.abs(seconds);
  const minutes = Math.floor(abs / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);
  const suffix = seconds > 0 ? "" : " ago";
  const prefix = seconds > 0 ? "in " : "";
  if (days > 0) return `${{prefix}}${{days}} day${{days === 1 ? "" : "s"}}${{suffix}}`;
  if (hours > 0) return `${{prefix}}${{hours}} hour${{hours === 1 ? "" : "s"}}${{suffix}}`;
  if (minutes > 0) return `${{prefix}}${{minutes}} min${{minutes === 1 ? "" : "s"}}${{suffix}}`;
  return seconds > 0 ? `in ${{seconds}} sec` : "just now";
}}
async function loadEvents() {{
  eventsEl.innerHTML = "";
  statusEl.textContent = "Loading reminders from background service...";
  try {{
    const response = await fetch(`${{API_URL}}/events`);
    if (!response.ok) throw new Error(`HTTP ${{response.status}}`);
    const payload = await response.json();
    allReminders = payload.events || [];
    storageEl.textContent = payload.storage ? `Database: ${{payload.storage.storage_path}}` : "";
    renderCards();
    statusEl.textContent = `Total active reminders: ${{allReminders.length}}`;
  }} catch (error) {{
    statusEl.textContent = "Error: Companion app daemon is unreachable.";
    storageEl.textContent = "";
    showErrorCard();
  }}
}}
function showErrorCard() {{
  eventsEl.innerHTML = `<article class="card error"><div><div class="title">Daemon Connection Error</div><div class="body">FCR Reminder is a tray companion application. Ensure fcr-reminder is running in your system tray on port 45677.</div></div><div class="actions"><a class="button" href="fcr-reminder://start">Start Daemon</a></div></article>`;
}}
function renderCards() {{
  const filter = searchEl.value.trim().toLowerCase();
  const reminders = allReminders.filter(reminder =>
    String(reminder.title || "").toLowerCase().includes(filter) ||
    String(reminder.body || "").toLowerCase().includes(filter)
  );
  if (!reminders.length) {{
    eventsEl.innerHTML = `<article class="card empty"><div><div class="title">${{filter ? "No Matching Reminders" : "No Active Reminders"}}</div><div class="body">${{filter ? "Try adjusting your search criteria." : "Add new calendar events with reminders in Obsidian to see them synchronized here."}}</div></div></article>`;
    return;
  }}
  eventsEl.innerHTML = reminders.map(reminder => {{
    const openButton = reminder.action_url ? `<a class="button" href="${{escapeHtml(reminder.action_url)}}">Open</a>` : "";
    return `<article class="card">
      <div><div class="title">${{escapeHtml(reminder.title)}}</div><div class="body">${{escapeHtml(reminder.body || "(No description provided)")}}</div><div class="time">Triggers: ${{new Date(reminder.trigger_at_rfc3339).toLocaleString()}}</div></div>
      <div><div class="badge">${{relativeTime(reminder.seconds_until_fire)}}</div><div class="actions">${{openButton}}<select data-snooze="${{escapeHtml(reminder.id)}}"><option value="">Snooze</option><option value="5">5 minutes</option><option value="15">15 minutes</option><option value="30">30 minutes</option><option value="60">1 hour</option><option value="1440">1 day</option></select><button class="danger" data-dismiss="${{escapeHtml(reminder.id)}}">Dismiss</button></div></div>
    </article>`;
  }}).join("");
  eventsEl.querySelectorAll("[data-dismiss]").forEach(button => button.addEventListener("click", () => dismissReminder(button.dataset.dismiss)));
  eventsEl.querySelectorAll("[data-snooze]").forEach(select => select.addEventListener("change", () => {{
    if (select.value) snoozeReminder(select.dataset.snooze, Number(select.value));
  }}));
}}
async function snoozeReminder(id, minutes) {{
  const reminder = allReminders.find(item => item.id === id);
  if (!reminder) return;
  await fetch(`${{API_URL}}/snooze`, {{ method:"POST", headers:{{"Content-Type":"application/json"}}, body:JSON.stringify({{ id:reminder.id, title:reminder.title, body:reminder.body, action_url:reminder.action_url, minutes }}) }});
  await loadEvents();
}}
async function dismissReminder(id) {{
  const payload = allReminders.filter(item => item.id !== id).map(item => ({{ id:item.id, title:item.title, body:item.body, trigger_at_epoch:item.trigger_at_epoch, action_url:item.action_url }}));
  await fetch(`${{API_URL}}/sync`, {{ method:"POST", headers:{{"Content-Type":"application/json"}}, body:JSON.stringify(payload) }});
  await loadEvents();
}}
async function triggerTestReminder() {{
  const nowEpoch = Math.floor(Date.now() / 1000);
  const payload = allReminders
    .filter(item => item.id !== "test-reminder-gui")
    .map(item => ({{ id:item.id, title:item.title, body:item.body, trigger_at_epoch:item.trigger_at_epoch, action_url:item.action_url }}));
  payload.push({{ id:"test-reminder-gui", title:"Test Notification", body:"FCR Reminder is successfully configured and running in the background.", trigger_at_epoch:nowEpoch + 10, action_url:"https://github.com/obsidian-full-calendar-remastered" }});
  await fetch(`${{API_URL}}/sync`, {{ method:"POST", headers:{{"Content-Type":"application/json"}}, body:JSON.stringify(payload) }});
  alert("Test notification scheduled to fire in 10 seconds!");
  await loadEvents();
}}
loadEvents();
</script>
</body>
</html>"##,
        api_url = js_string(&ctx.api_url),
        executable = js_string(&ctx.executable_path)
    )
}
