# Daily Operation

!!! abstract "Daily Runtime"
    This page covers the normal Windows runtime: how to launch the daemon, what the tray does, and what behavior to expect from a healthy single-instance session.

## Run the Daemon

!!! tip "Normal Launch"
    Launch `fcr-reminder.exe` for everyday use. In Windows release builds, it starts without opening a console window and moves directly into tray-first operation.

Normal runtime expectations:

- the daemon appears in the Windows system tray
- the local control API becomes available on `127.0.0.1:45677`
- a duplicate launch reuses the existing daemon instead of starting another instance
- **Intelligent Duplicate Prevention**: A sliding 10-minute history of fired notifications is kept, preventing duplicate alarms if Obsidian syncs the same reminder multiple times.
- **Missed Notification Recovery**: If the PC or daemon was offline during a reminder's trigger time, it will automatically detect and fire these missed reminders on startup, spaced out at safe 20-second intervals.

For visible logs during active debugging:

```powershell
.\fcr-reminder.exe --debug
```

!!! note "Single-Instance Rule"
    If the daemon is already active, launching `fcr-reminder.exe` again is not the right control path. Use `fcr-reminder-cli.exe` for explicit lifecycle commands instead.

## Tray Interaction

The Windows tray menu currently exposes:

- `Status: Running`: non-clickable status indicator
- `Info`: opens the About dialog with version, repository actions, and installation details
- `Quit`: shuts down the daemon cleanly

!!! info "About Dialog"
    The About surface is theme-aware, resizable, and intended as a user-facing runtime identity surface rather than a developer diagnostics view.

Compact index: [User Docs](index.md) · [Commands and Diagnostics](commands.md) · [Cleanup and Registration](cleanup.md)