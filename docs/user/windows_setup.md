# Windows Setup And Verification

!!! abstract "Windows Build Router"
	This page is the supported Windows workflow for toolchain setup, release builds, verification, and runtime diagnostics.

!!! note "Audience"
	Use this page if you are building or validating the daemon on Windows. For day-to-day operation after the binaries already exist, go to [Daily Operation](operations.md).

## Action Matrix

| Goal | Start here | Typical follow-up |
|---|---|---|
| Prepare the machine for native builds | [Toolchain Setup](#toolchain-setup) | [Build the Release Artifacts](#build-the-release-artifacts) |
| Produce the shipping binaries | [Build the Release Artifacts](#build-the-release-artifacts) | [Launch Model](#launch-model) |
| Verify lifecycle behavior first | [Preferred Verification Flow](#preferred-verification-flow) | [Daemon Diagnostics](#daemon-diagnostics) |
| Run an end-to-end seeded reminder check | [End-to-End Reminder Seeding](#end-to-end-reminder-seeding) | [Cleanup](#cleanup) |

## Toolchain Setup

Install the native Windows Rust toolchain requirements:

1. Install Visual Studio Build Tools 2022 or Visual Studio Community.
2. Select Desktop development with C++.
3. Install the Windows 10 or Windows 11 SDK.
4. Install Rust with `rustup` from `https://rustup.rs`.

Verify the toolchain:

```powershell
rustc --version
cargo --version
```

!!! tip "Why This Matters"
	The platform module depends on native Windows APIs for the tray app, toast notifications, resource embedding, and registry integration. A partial toolchain setup usually fails late, so verify the compiler first.

## Build the Release Artifacts

From the repository root:

```powershell
cd d:\Codes\full-calendar-remastered-ReminderApp
cargo build --release
```

Release outputs:

- `target\release\fcr-reminder.exe`
- `target\release\fcr-reminder-cli.exe`

!!! info "Binary Roles"
	Use `fcr-reminder.exe` for the normal GUI/tray runtime and `fcr-reminder-cli.exe` for terminal commands, diagnostics, and scripted lifecycle control.

## Launch Model

Windows release behavior:

- `fcr-reminder.exe` is built as a GUI-subsystem app, so double-clicking it should not open a console window
- the daemon starts in the tray
- the tray menu exposes `Status: Running`, `Info`, and `Quit`
- if another instance is already active, a duplicate launch exits and reuses the running daemon

For visible logs:

```powershell
.\target\release\fcr-reminder.exe --debug
```

## Preferred Verification Flow

!!! success "Use Code-Backed Checks First"
	Start with executable verification that can fail deterministically before relying on manual tray interaction.

Recommended order:

1. `cargo test -- --test-threads=1`
2. `powershell -File .\src\tests\dev-check.ps1`
3. `powershell -File .\src\tests\windows-test.ps1 -StartDaemon -SeedReminder`

The first command runs the lifecycle smoke test in [`tests/lifecycle_smoke.rs`](file:///d:/Codes/full-calendar-remastered-ReminderApp/tests/lifecycle_smoke.rs), which verifies daemon start and clean stop behavior.

## Daemon Diagnostics

Use the CLI companion to inspect the active daemon:

```powershell
.\target\release\fcr-reminder-cli.exe --health
.\target\release\fcr-reminder-cli.exe --storage
.\target\release\fcr-reminder-cli.exe --events
.\target\release\fcr-reminder-cli.exe --next
.\target\release\fcr-reminder-cli.exe --doctor
```

!!! example "Best Runtime Verification Command"
	`--doctor` is the strongest single command for confirming the live instance because it returns the PID, executable path, storage paths, and registration checks.

## End-to-End Reminder Seeding

For an end-to-end sync and notification check, use the scripted Windows harness:

```powershell
powershell -File .\src\tests\windows-test.ps1 -StartDaemon -SeedReminder
```

That script can:

- start a daemon if needed
- push a synthetic sync payload to `/sync`
- inspect health, storage, events, and next reminder data
- optionally seed a reminder a few seconds into the future for scheduler validation

## Cleanup

Cleanup should be run through the CLI companion in a terminal:

```powershell
.\target\release\fcr-reminder-cli.exe --cleanup
```

!!! warning "Do Not Delete State Manually"
	The supported cleanup path stops the daemon first, waits for shutdown, and only then removes registry entries and local app data. Manual deletion can leave registration and process state out of sync.

Compact index: [User Docs](index.md) · [Daily Operation](operations.md) · [Commands and Diagnostics](commands.md) · [Cleanup and Registration](cleanup.md)
