# Linux Runtime

!!! abstract "Ubuntu/Linux Runtime"
    This page covers tray behavior, notification dispatch, XDG registration, and cleanup behavior for Ubuntu/Linux targets.

## Tray And Dialog Experience

The Linux tray menu mirrors the Windows runtime:

- `Status: Running`
- `Active Reminders...`
- `Info`
- update status/action, when an update is available
- `Quit`

`Active Reminders...` opens a local browser-hosted Event Viewer generated into the app data directory. It supports the same operational controls as the Windows viewer: search, refresh, trigger test, open reminder URL, snooze, and dismiss.

`Info` opens a local browser-hosted About page with version, license, executable, storage path, update status, and project links.

## Notifications

Linux notifications are emitted from `platform/linux.rs` using `notify-rust`, which talks to the desktop notification server over D-Bus.

Reminder notifications include actions for:

- snooze for 5, 10, 15, 30, or 60 minutes
- open note, when the reminder has an action URL

Action support depends on the active desktop notification server. Ubuntu/GNOME supports the notification transport, but visible action presentation can vary by shell and notification settings.

## Registration

The daemon manages three Linux/XDG integration points:

- desktop entry: `$XDG_DATA_HOME/applications/fcr-reminder.desktop`
- autostart entry: `$XDG_CONFIG_HOME/autostart/fcr-reminder.desktop`
- custom protocol mapping: `$XDG_CONFIG_HOME/mimeapps.list` entry for `x-scheme-handler/fcr-reminder`

When the daemon starts, it writes or refreshes these files for the current executable path. The protocol command invokes:

```text
fcr-reminder --uri %u
```

Cleanup removes the desktop entry, autostart entry, and protocol mapping, then the shared cleanup path removes local app data.

## Diagnostics

`--doctor` includes Linux-specific checks for:

- desktop entry registration
- autostart registration
- protocol registration
- `xdg-open` availability

Compact index: [Architecture Docs](index.md) · [Runtime Overview](architecture.md) · [Windows Runtime](windows_runtime.md) · [Verification Strategy](verification.md)
