# Implementation Blueprint

!!! abstract "Implementation Index"
    This page maps the current repository structure to ownership and runtime responsibility. Use it as the source map for the existing codebase, not as a feature roadmap.

!!! info "How To Use This Page"
    If you already know the runtime concept you care about, this page tells you where that behavior lives in source. If you need the higher-level contract first, start with [Architecture Docs](index.md).

## Quick Router

| If you need to find... | Start here |
|---|---|
| shared reminder storage and logging | [`src/reminder_core`](#1-crate-ownership) |
| the daemon entrypoint and control plane | [`src/desktop/src/main.rs`](#2-desktop-source-map) |
| CLI forwarding behavior | [`src/desktop/src/cli/mod.rs`](#2-desktop-source-map) |
| Windows registry and About dialog code | [Windows-Specific Ownership](#4-windows-specific-ownership) |
| lifecycle smoke-test coverage | [Verification Inventory](#5-verification-inventory) |

## 1. Crate Ownership

### `src/reminder_core`

Shared library used by the desktop crate.

Owned responsibilities:

- reminder model definitions
- app-directory and storage-path resolution
- reminder load/save logic
- file-backed logging

### `src/desktop`

Desktop application crate.

Owned responsibilities:

- GUI daemon entry point
- console CLI companion entry point
- loopback HTTP API
- scheduler task
- tray bootstrap
- platform-specific integration layer
- smoke-test coverage for daemon lifecycle

## 2. Desktop Source Map

```text
src/desktop/
├── build.rs                     # Thin build-script shim
├── Cargo.toml                   # desktop crate metadata and binaries
├── src/
│   ├── cli/
│   │   └── mod.rs               # CLI companion forwarding logic
│   ├── cli_main.rs              # console binary entry point
│   ├── main.rs                  # daemon entry point, router, scheduler, tray wiring
│   └── platform/
│       ├── default.rs           # fallback platform no-op implementation
│       ├── linux.rs             # Linux platform integration stub
│       ├── macos.rs             # macOS platform integration stub
│       ├── mod.rs               # cross-platform platform surface
│       └── windows/
│           ├── build_support.rs # icon/resource embedding and Windows metadata
│           ├── console.rs       # console attach/alloc behavior for CLI modes
│           ├── mod.rs           # Windows module exports
│           ├── notification.rs  # Windows notification logic
│           └── registry.rs      # registration, cleanup, and About dialog
└── tests/
    └── lifecycle_smoke.rs       # code-backed start/stop smoke test
```

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

## 4. Windows-Specific Ownership

`platform/windows/registry.rs` owns:

- startup registration checks and creation
- protocol handler registration
- AppUserModelId registration
- cleanup and uninstall registry removal
- Windows About dialog launch

`platform/windows/notification.rs` owns:

- Windows native notification construction and dispatch
- action and snooze handling glue for Windows notifications

`platform/windows/console.rs` owns:

- console attachment behavior required for terminal-friendly CLI execution

## 5. Verification Inventory

Automated verification:

- `src/desktop/tests/lifecycle_smoke.rs`: daemon start/stop smoke test

Scripted verification:

- `src/tests/dev-check.ps1`
- `src/tests/dev-check.bash`
- `src/tests/windows-test.ps1`
- `src/tests/windows-test.bash`

Preferred order:

1. `cargo test -p desktop -- --test-threads=1`
2. `powershell -File .\src\tests\dev-check.ps1`
3. `powershell -File .\src\tests\windows-test.ps1 -StartDaemon -SeedReminder`

## 6. Extension Seams

The intended extension points in the current codebase are:

- new platform logic behind the `platform` module boundary
- additional CLI commands through `cli/mod.rs` and `main.rs` argument routing
- new loopback API endpoints in the Axum router in `main.rs`
- new storage-backed behavior in `reminder_core`

!!! warning "Boundary Rule"
    Any new shared behavior should stay out of Windows-specific modules unless it truly depends on Windows runtime, registry, or notification APIs.

Compact index: [Architecture Docs](index.md) · [Runtime Overview](architecture.md) · [Control API and Lifecycle](control_api.md) · [Windows Runtime](windows_runtime.md) · [Verification Strategy](verification.md)
