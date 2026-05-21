# FCR Reminder Daemon

FCR Reminder is the companion daemon for Full Calendar Remastered. Its job is simple: keep reminder delivery working after Obsidian closes by storing future reminder instances locally and firing them through native operating-system notification APIs.

## Current Source Of Truth

What is implemented today:

* Windows is the production-ready target.
* The daemon runs as a tray-first background app in release builds.
* A separate CLI companion binary exists for lifecycle commands, cleanup, and terminal-safe diagnostics.
* The local control API is bound to `127.0.0.1:45677`.
* Start/stop lifecycle behavior is covered by a Rust smoke test in `src/desktop/tests/lifecycle_smoke.rs`.

## Documentation Map

* [User usage guide](user/usage.md): commands, tray behavior, cleanup, and diagnostics
* [Windows setup guide](user/windows_setup.md): build and verification workflow on Windows
* [Architecture](architecture/architecture.md): runtime components, control API, storage, lifecycle, and registration model
* [Implementation blueprint](architecture/blueprint.md): source tree ownership, release artifacts, and extension seams
* [Developer integration guide](developer/integration_guide.md): sync payload contract and daemon-facing endpoints for plugin authors
