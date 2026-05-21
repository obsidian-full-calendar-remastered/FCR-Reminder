# Using FCR Reminder

This guide provides instructions on running, configuring, and interacting with the FCR Reminder background daemon.

---

## 1. Running the Daemon

By default, FCR Reminder is designed to run silently in the background with no console window visible. 

### Standard Mode (Headless)
When you launch the daemon (e.g., by double-clicking `desktop.exe` or starting it from a script/shortcut):
1. The terminal/console window is hidden instantly.
2. A premium **FCR Reminder** icon appears in your Windows system tray.
3. The HTTP server binds to `127.0.0.1:45677` and actively listens for updates from Obsidian.

### Debug Mode (Terminal Output)
If you are developing, troubleshooting, or want to actively watch logs:
```powershell
.\desktop.exe --debug
# or using the shortkey:
.\desktop.exe -d
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

---

## 4. Reverting to a Clean Slate (Uninstallation)

We strictly follow the **Clean Slate Philosophy**. If you wish to completely remove the application and all of its assets from your machine:
```powershell
.\desktop.exe --cleanup
```

This self-contained routine will immediately:
1. Delete the custom toast notification registry subkey `HKCU\Software\Classes\AppUserModelId\FCRReminder`.
2. Delete the Windows startup registry entry `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\FCRReminder`.
3. Permanently delete the local application data directory containing your reminder logs, databases, and assets.
