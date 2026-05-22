# Windows Runtime

!!! abstract "Windows-Specific Runtime"
    This page covers tray behavior, notification dispatch, and registry integration that are specific to the Windows production target.

## Tray and About Experience

The Windows tray menu currently contains:

- `Status: Running`
- `Info`
- `Quit`

`Info` opens the Windows About dialog, which is theme-aware and resizable.

## Notifications

Windows notifications are emitted from `platform/windows/notification.rs` and use the Windows notification APIs exposed by the `windows` crate.

## Registration

The daemon manages three Windows registrations:

- AppUserModelId: `HKCU\Software\Classes\AppUserModelId\FCRReminder`
- startup Run entry: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\FCRReminder`
- custom protocol: `HKCU\Software\Classes\fcr-reminder`

Registration behavior:

- first successful startup creates missing registrations
- later startups verify and skip entries that already match expected state
- the protocol command path is refreshed if the current executable no longer matches the registered command
- cleanup removes all of them

Compact index: [Architecture Docs](index.md) · [Runtime Overview](architecture.md) · [Control API and Lifecycle](control_api.md)