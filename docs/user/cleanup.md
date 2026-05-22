# Cleanup and Registration

!!! abstract "Cleanup Contract"
    This page covers safe cleanup, uninstall behavior, and the Windows registrations the daemon owns.

## Cleanup and Uninstallation

!!! warning "Safe Cleanup Order"
    Cleanup is designed to stop the daemon first. This avoids deleting registry or storage state while the process is still active.

Recommended command:

```powershell
.\fcr-reminder-cli.exe --cleanup
```

Cleanup flow:

1. detect whether the daemon is running
2. request a clean shutdown and wait for it to stop
3. remove `HKCU\Software\Classes\AppUserModelId\FCRReminder`
4. remove `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\FCRReminder`
5. remove `HKCU\Software\Classes\fcr-reminder`
6. delete the local app data directory under `AppData/Local/fullcalendar/ReminderApp/data`

## Windows Registration Behavior

!!! info "Registration Contract"
    The daemon owns three Windows integration points and repairs them when needed so notification actions and startup behavior remain aligned with the current executable.

On startup, the daemon manages:

- AppUserModelId metadata for branded notifications
- startup Run entry for user login startup
- `fcr-reminder://` protocol handler for notification actions and snooze flows

Later runs do not blindly rewrite these entries. The daemon checks whether the current registration is present and, for the protocol command path, refreshes stale values when necessary.

Compact index: [User Docs](index.md) · [Daily Operation](operations.md) · [Commands and Diagnostics](commands.md)