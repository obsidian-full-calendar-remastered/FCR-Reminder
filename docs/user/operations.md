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
- `Update available: <version>`: appears when the daemon has detected a newer GitHub release and opens the release page in the browser
- `Quit`: shuts down the daemon cleanly

If no newer release is currently known, the update entry stays disabled and acts as a passive status indicator instead of a launch action.

!!! info "About Dialog"
    The About surface is theme-aware, resizable, and intended as a user-facing runtime identity surface rather than a developer diagnostics view.

## Release Awareness

The daemon checks GitHub releases on a weekly cadence and keeps the last known update state in local app data.

Normal expectations:

- update checks do not block tray startup or reminder scheduling
- update notifications fire once per newly detected release version
- the tray menu and About dialog read the same cached update snapshot
- cleanup removes this release-check cache along with the rest of the app's local data

Use [`Commands and Diagnostics`](commands.md) and run `--updates` when you want the raw JSON view of that same state.

Compact index: [User Docs](index.md) · [Commands and Diagnostics](commands.md) · [Cleanup and Registration](cleanup.md)