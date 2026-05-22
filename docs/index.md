# FCR Reminder Daemon

!!! abstract "What This Project Is"
	FCR Reminder is the desktop companion daemon for Full Calendar Remastered. It keeps reminder delivery alive after Obsidian closes by storing future reminder instances locally, scheduling the next wake-up, and dispatching native operating-system notifications.

!!! info "Current Platform Reality"
	Windows is the fully implemented production target in this repository today. The release runtime is tray-first, single-instance, and paired with a separate CLI companion for lifecycle control and diagnostics.

!!! warning "Source-of-Truth Rule"
	The pages in this `docs/` tree describe the current implementation contract. If runtime behavior diverges from these pages, treat that as a defect and fix either the code or the affected documentation deliberately.

## Quick Router

| If you want to... | Start here | Typical follow-up |
|---|---|---|
| Build and verify the Windows app | [Windows Setup](user/windows_setup.md) | [Daily Operation](user/operations.md) |
| Operate the daemon day to day | [User Docs](user/index.md) | [Commands and Diagnostics](user/commands.md) |
| Understand runtime boundaries and invariants | [Architecture Docs](architecture/index.md) | [Implementation Blueprint](architecture/blueprint.md) |
| Integrate a host application or plugin | [Developer Integration Guide](developer/integration_guide.md) | [Control API and Lifecycle](architecture/control_api.md) |
| Inspect source ownership and extension seams | [Implementation Blueprint](architecture/blueprint.md) | [Runtime Overview](architecture/architecture.md) |

## Runtime Snapshot

!!! success "Implemented Today"
	- Windows is the supported production target.
	- `fcr-reminder.exe` is the tray-first GUI daemon.
	- `fcr-reminder-cli.exe` is the terminal-safe control surface.
	- The local control API binds to `127.0.0.1:45677`.
	- Lifecycle start/stop behavior is covered by the Rust smoke test in `src/desktop/tests/lifecycle_smoke.rs`.

## Documentation Map

- User router: [User Docs](user/index.md)
- Build and verification: [Windows Setup](user/windows_setup.md)
- Runtime contract: [Architecture Docs](architecture/index.md)
- Source ownership: [Implementation Blueprint](architecture/blueprint.md)
- Host integration: [Developer Integration Guide](developer/integration_guide.md)

---

[User Docs](user/index.md) · [Windows Setup](user/windows_setup.md) · [Architecture Docs](architecture/index.md) · [Blueprint](architecture/blueprint.md) · [Integration Guide](developer/integration_guide.md)
