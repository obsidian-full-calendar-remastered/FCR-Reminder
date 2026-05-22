# Control API and Lifecycle

!!! abstract "Control Plane"
    This page documents the daemon-facing HTTP routes and the lifecycle guarantees built on top of them.

## Control API

The daemon exposes a loopback-only HTTP API.

| Route | Method | Purpose |
|---|---|---|
| `/status` | `GET` | health summary and next-event information |
| `/events` | `GET` | full stored reminder list |
| `/next` | `GET` | next scheduled reminder |
| `/storage` | `GET` | resolved storage locations |
| `/doctor` | `GET` | instance, storage, and registration diagnostics |
| `/updates` | `GET` | cached GitHub release-update status and release page target |
| `/sync` | `POST` | replace stored reminder set and wake scheduler |
| `/snooze` | `POST` | reschedule a reminder after a snooze action |
| `/lifecycle/start` | `POST` | daemon start acknowledgement endpoint |
| `/lifecycle/stop` | `POST` | clean daemon shutdown |
| `/lifecycle/restart` | `POST` | clean restart |

!!! warning "Security Boundary"
    The daemon is intentionally loopback-only. The bind address is `127.0.0.1`, and remote network exposure is not part of the supported architecture.

## Read-Only Update State

`GET /updates` exposes the daemon's in-memory release snapshot. It is intended for diagnostics and local companion tooling, not as a general remote update service.

The route reports:

- current daemon version
- whether a newer release has been detected
- the latest seen GitHub release metadata
- last and next check timestamps
- the release URL used by the tray menu and About dialog

The route does not trigger an immediate GitHub fetch. It returns the existing cached state so the control plane remains fast and deterministic.

## Lifecycle Model

Lifecycle entry points:

- `--start`
- `--stop`
- `--restart`
- `--cleanup`

Cleanup behavior is intentionally ordered:

1. detect whether the daemon is running
2. request clean stop
3. wait for the daemon to become unreachable
4. delete Windows registrations
5. remove local app data

!!! example "Why the Order Matters"
    The cleanup path is designed to prevent deleting files or registry state while the daemon is still active. This keeps lifecycle management deterministic and avoids silent background corruption.

Compact index: [Architecture Docs](index.md) · [Windows Runtime](windows_runtime.md) · [Verification Strategy](verification.md)