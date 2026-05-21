# Using FCR Reminder

This guide is the user-facing reference for running, inspecting, and removing FCR Reminder on Windows.

## 1. Running The Daemon

Normal Windows usage:

* launch `fcr-reminder.exe`
* the app starts without opening a terminal window in release mode
* the daemon appears in the Windows system tray
* the daemon exposes its local control API on `127.0.0.1:45677`

For active logs, run the daemon from a terminal:

```powershell
.\fcr-reminder.exe --debug
```

Single-instance behavior:

* if the daemon is already running, a second `fcr-reminder.exe` launch exits cleanly and reuses the existing instance
* use `fcr-reminder-cli.exe` for explicit lifecycle commands instead of relaunching the GUI binary repeatedly from scripts

## 2. Tray Interaction

The Windows tray menu currently contains:

* `Status: Running`: non-clickable status indicator
* `Info`: opens the About dialog with the app icon, version, repository actions, and installation details
* `Quit`: exits the daemon

The About dialog follows the current Windows app theme and is resizable.

## 3. CLI Commands

Use `fcr-reminder-cli.exe` for terminal-friendly control and diagnostics.

| Flag / Option | Shortkey | Description |
| :--- | :--- | :--- |
| `--help` | `-h` | Show the full CLI help menu. |
| `--debug` | `-d` | Run the daemon with a visible console. |
| `--cleanup` / `--uninstall` | `-c` | Stop the daemon if needed, then remove Windows registry entries and local app data. |
| `--health` |  | Print daemon status and next-event summary as JSON. |
| `--next` |  | Print the next reminder as JSON. |
| `--events` |  | Print all stored reminders as JSON. |
| `--storage` |  | Print resolved app and storage paths as JSON. |
| `--doctor` |  | Print PID, executable path, storage, and registration checks as JSON. |
| `--start` |  | Start the daemon if it is not already running. |
| `--stop` |  | Ask the daemon to shut down cleanly. |
| `--restart` |  | Ask the daemon to restart cleanly. |
| `--inspect <target>` |  | Alias for `health`, `next`, `events`, or `storage`. |

Examples:

```powershell
.\fcr-reminder-cli.exe --start
.\fcr-reminder-cli.exe --doctor
.\fcr-reminder-cli.exe --events
.\fcr-reminder-cli.exe --stop
```

## 4. Inspection And Health Checks

These commands talk to the live daemon over localhost and return structured JSON:

```powershell
.\fcr-reminder-cli.exe --health
.\fcr-reminder-cli.exe --next
.\fcr-reminder-cli.exe --events
.\fcr-reminder-cli.exe --storage
.\fcr-reminder-cli.exe --doctor
```

What `--doctor` confirms:

* active daemon PID
* executable path of the running instance
* resolved storage paths
* Windows registration checks for AppUserModelId, startup entry, and custom protocol handler

## 5. Cleanup And Uninstallation

Cleanup is designed to be safe even if the daemon is already running.

Recommended command:

```powershell
.\fcr-reminder-cli.exe --cleanup
```

Behavior:

1. Detect a running daemon.
2. Request a clean shutdown and wait for it to stop.
3. Remove `HKCU\Software\Classes\AppUserModelId\FCRReminder`.
4. Remove `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\FCRReminder`.
5. Remove `HKCU\Software\Classes\fcr-reminder`.
6. Delete the local app data directory under `AppData/Local/fullcalendar/ReminderApp/data`.

## 6. Windows Registration Behavior

On first successful Windows startup, the daemon registers:

* AppUserModelId metadata for branded notifications
* startup Run entry for user login startup
* `fcr-reminder://` protocol handler for notification actions and snooze flows

On later runs, the daemon checks whether each registration already exists and skips rewriting it if it is already present.
