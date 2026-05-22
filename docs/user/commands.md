# Commands and Diagnostics

!!! abstract "CLI Surface"
    This page is the command router for terminal-friendly lifecycle control, structured diagnostics, and live daemon inspection.

## CLI Reference

| Flag / Option | Shortkey | Description |
|---|---|---|
| `--help` | `-h` | Show the full CLI help menu. |
| `--debug` | `-d` | Run the daemon with a visible console. |
| `--cleanup` / `--uninstall` | `-c` | Stop the daemon if needed, then remove Windows registry entries and local app data. |
| `--health` |  | Print daemon status and next-event summary as JSON. |
| `--next` |  | Print the next reminder as JSON. |
| `--events` |  | Print all stored reminders as JSON. |
| `--storage` |  | Print resolved app and storage paths as JSON. |
| `--doctor` |  | Print PID, executable path, storage, and registration checks as JSON. |
| `--updates` |  | Print GitHub release update status and latest-release metadata as JSON. |
| `--start` |  | Start the daemon if it is not already running. |
| `--stop` |  | Ask the daemon to shut down cleanly. |
| `--restart` |  | Ask the daemon to restart cleanly. |
| `--inspect <target>` |  | Alias for `health`, `next`, `events`, `storage`, `doctor`, or `updates`. |

## Common Commands

```powershell
.\fcr-reminder-cli.exe --start
.\fcr-reminder-cli.exe --doctor
.\fcr-reminder-cli.exe --updates
.\fcr-reminder-cli.exe --events
.\fcr-reminder-cli.exe --stop
```

## Inspection and Health

These commands talk to the live daemon over localhost and return structured JSON:

```powershell
.\fcr-reminder-cli.exe --health
.\fcr-reminder-cli.exe --next
.\fcr-reminder-cli.exe --events
.\fcr-reminder-cli.exe --storage
.\fcr-reminder-cli.exe --doctor
.\fcr-reminder-cli.exe --updates
```

`--updates` returns the daemon's current release-check snapshot, including whether an update is available, what the latest release version is, when the last check ran, and which GitHub release page the tray and About dialog will open.

!!! example "Best Single Command"
    `--doctor` is the strongest first-line operational check because it confirms the active PID, executable path, storage resolution, and Windows registration state in one response.

!!! tip "Best Update Check"
    `--updates` is the direct runtime check for release awareness. Use it when you want to confirm whether the daemon already saw a newer GitHub release without opening the tray menu or About dialog.

What `--doctor` verifies:

- active daemon PID
- executable path of the running instance
- resolved storage paths
- Windows registration checks for AppUserModelId, startup entry, and custom protocol handler

What `--updates` reports:

- current daemon version
- whether a newer GitHub release is available
- latest seen release version and publication timestamp
- last successful or failed check time
- release page URL used by the tray menu and About dialog

Compact index: [User Docs](index.md) · [Daily Operation](operations.md) · [Cleanup and Registration](cleanup.md)