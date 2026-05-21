# 🔔 FCR Reminder Daemon

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE.md)
[![Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/OS-Windows%20%7C%20Linux%20%7C%20macOS-success.svg)]()

A lightweight, premium, and resource-efficient background reminder daemon built in Rust. It serves as the companion service for the **Obsidian Full Calendar Remastered** plugin to guarantee that you never miss a calendar event notification, even when Obsidian is completely closed.

---

## 📖 The Problem & The Solution

**The Challenge:** Obsidian is a heavy electron desktop application and cannot run persistently as a background daemon. When you close Obsidian, all internal timer loops and alert notifications terminate. 

**The Solution:** **FCR Reminder** is a dedicated background service written in pure, asynchronous Rust with Tokio. It runs silently in your system tray, listens for flat event synchronization payloads from the Obsidian plugin on port `45677`, schedules highly accurate timers, and triggers  OS-native toast notifications.

---

## 🛠️ Architecture & Monorepo Structure

To maintain a pristine project root, all core source code is organized under the `src/` directory:

```text
full-calendar-remastered-ReminderApp/
├── Cargo.toml                  # Workspace root configuration
├── Cargo.lock                  # Dependency lockfile
├── LICENSE.md                  # Project license
├── README.md                   # This document
├── CONTRIBUTING.md             # Guidelines for Rust developers
├── mkdocs.yml                  # Documentation site configuration
├── assets/                     # Workspace assets (icons, etc.)
│   └── icon.png                # Custom premium calendar/clock icon
├── docs/                       # Technical and user guides
│   ├── index.md                # Welcome documentation page
│   ├── user/                   # User-facing guides (windows_setup.md, usage.md)
│   └── architecture/           # Technical blueprints (architecture.md, blueprint.md)
└── src/                        # Root clean source directory
    ├── desktop/                # Windows/Linux/macOS Headless Tray CLI Daemon
    │   ├── Cargo.toml
    │   └── src/main.rs         # Win32 message loop, tray icon, HTTP server, autostart
    ├── reminder_core/          # Shared Core library (compiled for all targets)
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs          # UniFFI interface exports
    │       ├── models.rs       # Shared event structs and serializations
    │       ├── storage.rs      # Local JSON database storage
    │       └── logger.rs       # Stateless file appending logger
    └── tests/                  # Asynchronous script verification tools
        ├── dev-check.ps1       # Formatting/Clippy/cargo test runner (PowerShell)
        ├── dev-check.bash      # Formatting/Clippy/cargo test runner (Bash)
        ├── windows-test.ps1    # Simulates an Obsidian sync call (PowerShell)
        └── windows-test.bash   # Simulates an Obsidian sync call (Bash)
```

---

## ⚙️ Core Design Choices & Philosophy

### 🛡️ 1. The Dumb Client Principle
The daemon has no parsing engine for complex `.ics` or `RRule` rules. Obsidian is the single source of truth. It computes recurrence rules, parses events, and performs an HTTP POST request to FCR Reminder with a flat JSON array of pre-calculated unix-epoch reminder instances.

### 🔌 2. Local-Only Bounded Security
The HTTP sync server binds exclusively to `127.0.0.1:45677` (localhost). It is entirely inaccessible from the external network or other devices, ensuring complete network sandboxing.

### 💾 3. Dev Directory Routing (OS Hygiene)
* **Debug builds (`cargo run` / `cargo test`):** Appends logs and stores the SQLite/JSON databases in the workspace-local `dev/` directory, keeping the developer's operating system 100% pristine.
* **Release builds (`cargo build --release`):** Stores reminders persistently under the standard system directory `AppData/Local/fullcalendar/ReminderApp/` so reminders survive disk cleanups.

### 🧹 4. Clean Slate Philosophy
We enforce a strict law of **Zero Unmanaged Leftovers**. Running the daemon with the cleanup command:
```powershell
.\desktop.exe --cleanup
```
instantly purges:
1. Custom Windows Toast notification app branding registries (`HKCU\Software\Classes\AppUserModelId\FCRReminder`).
2. Auto-start startup registry configurations (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run\FCRReminder`).
3. Recursive deletion of all persistent `AppData` files and database assets.

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
.\target\release\desktop.exe --debug
```

Run in **Standard Mode** (instantly hides console window and sits silently in the system tray):
```powershell
.\target\release\desktop.exe
```

---

## 🕹️ Command-Line Reference

| Flag / Option | Shortkey | Behavior |
| :--- | :--- | :--- |
| `--help` | `-h` | Prints detailed options, usage metadata, and exits. |
| `--debug` | `-d` | Forces the console window to stay open and prints active runtime logs. |
| `--cleanup` / `--uninstall` | `-c` | Completely wipes all app database files, logs, and system registries. |

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

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE.md) file for details.
