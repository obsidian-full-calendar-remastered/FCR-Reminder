# Windows Setup And Verification

This guide describes the supported Windows workflow for building, packaging, and verifying FCR Reminder.

## 1. Toolchain Setup

Install the native Windows Rust toolchain requirements:

1. Install Visual Studio Build Tools 2022 or Visual Studio Community.
2. Select Desktop development with C++.
3. Make sure the Windows 10/11 SDK is installed.
4. Install Rust with `rustup` from `https://rustup.rs`.

Verify the toolchain:

```powershell
rustc --version
cargo --version
```

## 2. Build The Release Artifacts

From the repository root:

```powershell
cd d:\Codes\full-calendar-remastered-ReminderApp
cargo build --release
```

Release outputs:

* `target\release\fcr-reminder.exe`
* `target\release\fcr-reminder-cli.exe`

Use `fcr-reminder.exe` for normal launches and `fcr-reminder-cli.exe` for terminal commands.

## 3. Launch Model

Windows release behavior:

* `fcr-reminder.exe` is built as a GUI-subsystem app, so double-clicking it should not open a console window
* the daemon starts in the tray
* the tray menu exposes `Status: Running`, `Info`, and `Quit`
* if another instance is already active, a duplicate launch exits and reuses the running daemon

For visible logs:

```powershell
.\target\release\fcr-reminder.exe --debug
```

## 4. Preferred Verification Flow

Use code-backed verification first:

```powershell
cargo test -p desktop -- --test-threads=1
```

This runs the desktop lifecycle smoke test in `src/desktop/tests/lifecycle_smoke.rs`, which verifies daemon start and clean stop behavior.

Run the broader repo checks when needed:

```powershell
powershell -File .\src\tests\dev-check.ps1
```

## 5. Daemon Diagnostics

Use the CLI companion to inspect the active daemon:

```powershell
.\target\release\fcr-reminder-cli.exe --health
.\target\release\fcr-reminder-cli.exe --storage
.\target\release\fcr-reminder-cli.exe --events
.\target\release\fcr-reminder-cli.exe --next
.\target\release\fcr-reminder-cli.exe --doctor
```

`--doctor` is the strongest single command for confirming the live instance because it returns the PID, executable path, storage paths, and registration checks.

## 6. End-To-End Reminder Seeding

For an end-to-end sync and notification check, use the scripted Windows harness:

```powershell
powershell -File .\src\tests\windows-test.ps1 -StartDaemon -SeedReminder
```

That script can:

* start a daemon if needed
* push a synthetic sync payload to `/sync`
* inspect health, storage, events, and next reminder data
* optionally seed a reminder a few seconds into the future for scheduler validation

## 7. Cleanup

Cleanup should be run through the CLI companion in a terminal:

```powershell
.\target\release\fcr-reminder-cli.exe --cleanup
```

The cleanup path now stops the daemon first, waits for shutdown, and only then removes registry entries and local app data.
