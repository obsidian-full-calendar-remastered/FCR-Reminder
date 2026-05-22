# Implementation Blueprint

!!! abstract "Implementation Index"
    This page maps the current repository structure to ownership and runtime responsibility. Use it as the source map for the existing codebase, not as a feature roadmap.

!!! info "How To Use This Page"
    If you already know the runtime concept you care about, this page tells you where that behavior lives in source. If you need the higher-level contract first, start with [Architecture Docs](index.md).

## Quick Router

| If you need to find... | Start here |
|---|---|
| shared reminder storage and logging | [`src/core/storage.rs`](file:///d:/Codes/full-calendar-remastered-ReminderApp/src/core/storage.rs), [`src/core/logger.rs`](file:///d:/Codes/full-calendar-remastered-ReminderApp/src/core/logger.rs) |
| the daemon entrypoint and control plane | [`src/main.rs`](file:///d:/Codes/full-calendar-remastered-ReminderApp/src/main.rs), [`src/core/daemon.rs`](file:///d:/Codes/full-calendar-remastered-ReminderApp/src/core/daemon.rs) |
| CLI forwarding behavior | [`src/cli_main.rs`](file:///d:/Codes/full-calendar-remastered-ReminderApp/src/cli_main.rs), [`src/core/cli.rs`](file:///d:/Codes/full-calendar-remastered-ReminderApp/src/core/cli.rs) |
| Windows registry and About dialog code | [Windows-Specific Ownership](#4-windows-specific-ownership) |
| lifecycle smoke-test coverage | [Verification Inventory](#5-verification-inventory) |

---

## 1. Module Ownership

### `src/core`

The single source of truth for all business, storage, and execution logic.

Owned responsibilities:

- **models.rs**: reminder model and serialization definitions.
- **storage.rs**: app-directory resolution and database storage persistence.
- **logger.rs**: file-backed and console logging macros (`log_info!`, `log_warn!`, `log_error!`).
- **scheduler.rs**: Tokio scheduling loop, duplicate notification prevention (10-minute sliding window), and missed notification startup recovery.
- **api.rs**: loopback Axum API routes and request handlers.
- **commands.rs**: core CLI inspection and lifecycle command executors.
- **cli.rs**: client companion forwarding logic.
- **daemon.rs**: primary tray loop bootstrap, port-binding, single-instance lock, and custom protocol handler logic.

### `src/platform`

Platform-specific wrapper implementations under a uniform cross-platform surface.

Owned responsibilities:

- **mod.rs**: cross-platform surface abstraction interface.
- **windows/**: WinRT-based interactive Toasts, autostart/protocol registry wiring, and GUI console allocation.
- **linux.rs**: Linux D-Bus and system tray integrations.
- **macos.rs**: macOS Cocoa native runtime bindings.
- **default.rs**: Fallback dummy wrapper implementations.

---

## 2. Source Map

```text
src/
├── main.rs                      # Minimal daemon entry point (calls src/core::run_daemon())
├── cli_main.rs                  # Minimal CLI entry point (calls src/core::run_cli())
├── build.rs                     # Unified build script shim
├── core/                        # Single source of truth for implementation logic
│   ├── mod.rs                   # Module declarations and entry orchestrators
│   ├── models.rs                # Reminder definitions and payload schema
│   ├── storage.rs               # DB load/save logic and directory resolution
│   ├── logger.rs                # Logging macros (log_info!, log_warn!, log_error!)
│   ├── scheduler.rs             # Background Tokio loop and missed reminder recovery
│   ├── api.rs                   # HTTP API endpoint configuration and handlers
│   ├── commands.rs              # Client command execution handlers
│   ├── cli.rs                   # Companion CLI forwarded resolver
│   └── daemon.rs                # Core daemon bootstrap, tray thread, and instance binder
└── platform/                    # Platform-specific wrappers
    ├── mod.rs                   # Shared platform traits and methods
    ├── default.rs               # Fallback dummy platform layer
    ├── linux.rs                 # Linux platform integration stub
    ├── macos.rs                 # macOS platform integration stub
    └── windows/                 # Windows-specific system integrations
        ├── mod.rs               # Windows platform trait setup and window loops
        ├── console.rs           # Console attachment utilities for CLI execution
        ├── notification.rs      # WinRT Interactive Toast notification setup
        ├── registry.rs          # Registry autostart, protocol schema, and About dialog
        └── build_support.rs     # Windows compiler support and resource compilation
```

---

## 3. Packaging Model

Windows release packaging should include:

- `fcr-reminder.exe`
- `fcr-reminder-cli.exe`

Expected roles:

- `fcr-reminder.exe`
  - user-facing launch target
  - tray-first runtime
  - embedded Windows icon and metadata
- `fcr-reminder-cli.exe`
  - terminal-safe companion for lifecycle, diagnostics, and cleanup

!!! note "Packaging Principle"
    The runtime intentionally separates tray UX from terminal UX. That boundary keeps double-click launches silent while preserving explicit diagnostics and automation support.

---

## 4. Windows-Specific Ownership

`src/platform/windows/registry.rs` owns:

- startup registration checks and creation
- protocol handler registration
- AppUserModelId registration
- cleanup and uninstall registry removal
- Windows About dialog launch

`src/platform/windows/notification.rs` owns:

- Windows native notification construction and dispatch
- action and snooze handling glue for Windows notifications

`src/platform/windows/console.rs` owns:

- console attachment behavior required for terminal-friendly CLI execution

---

## 5. Verification Inventory

Automated verification:

- [`tests/lifecycle_smoke.rs`](file:///d:/Codes/full-calendar-remastered-ReminderApp/tests/lifecycle_smoke.rs): daemon start/stop smoke test

Scripted verification:

- `src/tests/dev-check.ps1`
- `src/tests/dev-check.bash`
- `src/tests/windows-test.ps1`
- `src/tests/windows-test.bash`

Preferred order:

1. `cargo test -- --test-threads=1`
2. `powershell -File .\src\tests\dev-check.ps1`
3. `powershell -File .\src\tests\windows-test.ps1 -StartDaemon -SeedReminder`

---

## 6. Extension Seams

The intended extension points in the current codebase are:

- new platform logic behind the `src/platform` module boundary
- additional CLI commands through `src/core/commands.rs` and argument routing in `src/core/cli.rs`
- new loopback API endpoints in the Axum router in `src/core/api.rs`
- new storage-backed behavior in `src/core/storage.rs`

!!! warning "Boundary Rule"
    Any new shared behavior should stay out of Windows-specific modules unless it truly depends on Windows runtime, registry, or notification APIs.

Compact index: [Architecture Docs](index.md) · [Runtime Overview](architecture.md) · [Control API and Lifecycle](control_api.md) · [Windows Runtime](windows_runtime.md) · [Verification Strategy](verification.md)
