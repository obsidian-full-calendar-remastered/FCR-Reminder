# 🔔 FCR Reminder Daemon

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE.md)
[![Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/OS-Windows-success.svg)]()
<!-- [![Status](https://img.shields.io/badge/OS-Windows%20%7C%20Linux%20%7C%20macOS-success.svg)]() -->

A lightweight, premium, and resource-efficient background reminder daemon built in Rust. It serves as the companion service for the **Obsidian Full Calendar Remastered** plugin to guarantee that you never miss a calendar event notification, even when Obsidian is completely closed.

---

## 📖 Motivation

**The Challenge:** Obsidian is a heavy electron desktop application and cannot run persistently as a background daemon. When you close Obsidian, all internal timer loops and alert notifications terminate. 

**The Solution:** **FCR Reminder** is a dedicated background service written in pure, asynchronous Rust with Tokio. It runs silently in your system tray, listens for flat event synchronization payloads from the Obsidian plugin on port `45677`, schedules highly accurate timers, and triggers  OS-native toast notifications.

➡️ See Full documentation [here](https://obsidian-full-calendar-remastered.github.io/FCR-Reminder-Companion-App/).

---

## ⚙️ Core Design Choices & Philosophy

### 🛡️ 1. The Dumb Client Principle
The daemon has no parsing engine for complex `.ics` or `RRule` rules. Obsidian is the single source of truth. It computes recurrence rules, parses events, and performs an HTTP POST request to FCR Reminder with a flat JSON array of pre-calculated unix-epoch reminder instances.

### 🔌 2. Security
- No vault wide reads or writes. Completely dumb with bare **minimum user level access**.
- The HTTP sync server binds exclusively to `127.0.0.1:45677` (localhost). It is entirely inaccessible from the external network or other devices, ensuring complete network sandboxing.
- For added security you may block the inbound and outbound access at OS Firewall level. But this would also mean you won't be notified of the new releases.

### 💾 3. Dev Directory Routing (OS Hygiene)
* **Debug builds (`cargo run` / `cargo test`):** Appends logs and stores the SQLite/JSON databases in the workspace-local `dev/` directory, keeping the developer's operating system 100% pristine.
* **Release builds (`cargo build --release`):** Stores reminders persistently under the standard system directory `AppData/Local/fullcalendar/ReminderApp/` so reminders survive disk cleanups.

### 🧹 4. Clean Slate Philosophy
We enforce a strict **Zero Unmanaged Leftovers**. Running the daemon with the cleanup command:
```powershell
.\fcr-reminder.exe --cleanup
```
instantly purges:
1. Custom Windows Toast notification app branding registries (`HKCU\Software\Classes\AppUserModelId\FCRReminder`).
2. Auto-start startup registry configurations (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run\FCRReminder`).
3. Recursive deletion of all persistent `AppData` files and database assets.
4. This ensures a 0 leftover uninstall

---

## 🚀 Getting Started

### 📋 Prerequisites (Windows Compilation)
1. Install **Visual Studio Build Tools 2022** with the **Desktop development with C++** workload selected.
2. Install the **Windows 10/11 SDK** component.
3. Install **Rustup** from [https://rustup.rs](https://rustup.rs).

### 🛠️ Compiling & Running

Clone the repository and build the binary:
```powershell
# Navigate into the project root
cd d:\Codes\full-calendar-remastered-ReminderApp

# Compile in release mode
cargo build --release
```

Run in **Debug Mode** (keeps the console visible with active logging):
```powershell
.\target\release\fcr-reminder.exe --debug
```

Run in **Standard Mode** (instantly hides console window and sits silently in the system tray):
```powershell
.\target\release\fcr-reminder.exe
```

On Windows release builds, `fcr-reminder.exe` starts as a tray-first background app. Double-clicking it should place the daemon in the system tray without opening a terminal window. When you want live logs, launch it from an existing terminal and pass `--debug`.

---

## 🕹️ Command-Line Reference

| Flag / Option | Shortkey | Behavior |
| :--- | :--- | :--- |
| `--help` | `-h` | Prints detailed options, usage metadata, and exits. |
| `--debug` | `-d` | Forces the console window to stay open and prints active runtime logs. |
| `--cleanup` / `--uninstall` | `-c` | Completely wipes all app database files, logs, and system registries. |
| `--health` |  | Queries the running daemon for health, storage details, and the next scheduled reminder. |
| `--next` |  | Queries the running daemon for the next reminder that will fire. |
| `--events` |  | Queries the running daemon for the full list of reminders currently stored on disk. |
| `--storage` |  | Queries the running daemon for its resolved app directory, reminder database path, and file URLs. |
| `--doctor` |  | Runs a live diagnostic against the active daemon and reports the PID, executable path, storage, and platform registration checks. |
| `--start` |  | Starts the daemon if it is not already running. |
| `--stop` |  | Asks the running daemon to shut itself down cleanly. |
| `--restart` |  | Asks the running daemon to restart itself cleanly. |
| `--inspect <target>` |  | Alias for `health`, `next`, `events`, or `storage`. |

### Terminal Daemon Inspection

These commands talk to the live daemon over `127.0.0.1:45677` and print structured JSON in your terminal. Use the console companion binary so PowerShell waits correctly and restores the prompt at the end of the output:

```powershell
.\target\release\fcr-reminder-cli.exe --health
.\target\release\fcr-reminder-cli.exe --next
.\target\release\fcr-reminder-cli.exe --events
.\target\release\fcr-reminder-cli.exe --storage
.\target\release\fcr-reminder-cli.exe --doctor
.\target\release\fcr-reminder-cli.exe --stop
.\target\release\fcr-reminder-cli.exe --restart
```

The storage paths and file URLs are resolved by the daemon at runtime, so the reported locations always match the active machine and build mode.

All of these commands talk to the one daemon bound to `127.0.0.1:45677`. If `fcr-reminder-cli.exe` is on your `PATH`, you can run `fcr-reminder-cli --doctor` from any terminal. If it is not on your `PATH`, use the full or relative executable path instead.

---

## 🧪 Developer Verification & Tests

To execute cargo tests, formatting checks, and strict clippy lints, run the PowerShell validation script:
```powershell
powershell -File .\src\tests\dev-check.ps1
```

To simulate a JSON sync payload from Obsidian and trigger a dynamic native notification 15 seconds in the future:
```powershell
powershell -File .\src\tests\windows-test.ps1
```

To start a debug daemon if needed, inspect its health and storage, and optionally seed a reminder for scheduler testing:

```powershell
powershell -File .\src\tests\windows-test.ps1 -StartDaemon -SeedReminder
```

---

## 📄 License

This project is licensed under the GPL v3.0 License - see the [LICENSE.md](LICENSE.md) file for details.