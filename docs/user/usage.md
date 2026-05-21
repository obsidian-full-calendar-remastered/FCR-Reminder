# Using FCR Reminder

This guide provides instructions on running, configuring, and interacting with the FCR Reminder background daemon.

---

## 1. Running the Daemon

By default, FCR Reminder is designed to run silently in the background with no console window visible. 

### Standard Mode (Headless)
When you launch the daemon (e.g., by double-clicking `fcr-reminder.exe` or starting it from a script/shortcut):
1. On Windows release builds, no daemon console window is created.
2. A premium **FCR Reminder** icon appears in your Windows system tray.
3. The HTTP server binds to `127.0.0.1:45677` and actively listens for updates from Obsidian.

### Debug Mode (Terminal Output)
If you are developing, troubleshooting, or want to actively watch logs:
```powershell
.\fcr-reminder.exe --debug
# or using the shortkey:
.\fcr-reminder.exe -d
```
In this mode, the console window remains visible, printing color-coded diagnostic logs for every event synchronization and trigger timer event.

---

## 2. System Tray Interaction

Once active, the daemon places a sleek clock/calendar icon in your system tray:
* **Hover:** Hovering over the icon displays the tooltip `"FCR Reminder"`.
* **Context Menu:** Right-clicking the tray icon opens a context menu with options:
  * `Status: Running` (Indicator)
  * `Quit` (Stops the daemon completely)

---

## 3. Command-Line Arguments

The application accepts a variety of CLI arguments for setup, debugging, and cleanup:

| Flag / Option | Shortkey | Description |
| :--- | :--- | :--- |
| `--help` | `-h` | Renders a detailed help menu listing all available arguments and exits. |
| `--debug` | `-d` | Forces the console window to stay open and prints active runtime logs. |
| `--cleanup` / `--uninstall` | `-c` | Completely wipes all database entries, files, icons, and Registry keys. |
| `--health` |  | Prints daemon health, active reminder count, storage details, and the next scheduled reminder as JSON. |
| `--next` |  | Prints the next reminder that will fire. |
| `--events` |  | Prints every reminder currently stored by the daemon. |
| `--storage` |  | Prints the daemon's resolved storage directory, database path, and file URLs. |
| `--doctor` |  | Prints a live diagnostic report including the daemon PID, executable path, storage details, and platform registration checks. |
| `--start` |  | Starts the daemon if it is not already running. |
| `--stop` |  | Stops the running daemon cleanly through the local control API. |
| `--restart` |  | Restarts the running daemon cleanly through the local control API. |
| `--inspect <target>` |  | Alias for `health`, `next`, `events`, or `storage`. |

---

## 4. Terminal Inspection & Health Checks

Use these commands from PowerShell or Command Prompt while the daemon is already running:

```powershell
.\fcr-reminder-cli.exe --health
.\fcr-reminder-cli.exe --next
.\fcr-reminder-cli.exe --events
.\fcr-reminder-cli.exe --storage
.\fcr-reminder-cli.exe --doctor
.\fcr-reminder-cli.exe --stop
.\fcr-reminder-cli.exe --restart
```

Each command asks the running daemon for live data over localhost, so the reminder count, next fire time, and storage paths always reflect the active instance instead of static assumptions.

If `fcr-reminder-cli.exe` is on your `PATH`, you can type `fcr-reminder-cli --doctor` or `fcr-reminder-cli --health` from any terminal and it will target the exact daemon instance currently bound to `127.0.0.1:45677`. If it is not on your `PATH`, call it with its full path or from the directory that contains the executable.

---

## 5. Reverting to a Clean Slate (Uninstallation)

We strictly follow the **Clean Slate Philosophy**. If you wish to completely remove the application and all of its assets from your machine:
```powershell
.\fcr-reminder.exe --cleanup
```

This self-contained routine will immediately:
1. Delete the custom toast notification registry subkey `HKCU\Software\Classes\AppUserModelId\FCRReminder`.
2. Delete the Windows startup registry entry `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\FCRReminder`.
3. Permanently delete the local application data directory containing your reminder logs, databases, and assets.
