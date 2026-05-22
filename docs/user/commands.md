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
| `--start` |  | Start the daemon if it is not already running. |
| `--stop` |  | Ask the daemon to shut down cleanly. |
| `--restart` |  | Ask the daemon to restart cleanly. |
| `--inspect <target>` |  | Alias for `health`, `next`, `events`, or `storage`. |

## Common Commands

```powershell
.\fcr-reminder-cli.exe --start
.\fcr-reminder-cli.exe --doctor
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
```

!!! example "Best Single Command"
    `--doctor` is the strongest first-line operational check because it confirms the active PID, executable path, storage resolution, and Windows registration state in one response.

What `--doctor` verifies:

- active daemon PID
- executable path of the running instance
- resolved storage paths
- Windows registration checks for AppUserModelId, startup entry, and custom protocol handler

Compact index: [User Docs](index.md) · [Daily Operation](operations.md) · [Cleanup and Registration](cleanup.md)