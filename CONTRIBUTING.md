# 🎉 Contributing to FCR Reminder Daemon

Thank you for your interest in contributing to the **FCR Reminder Daemon**! This project is built on pure, asynchronous Rust and designed to be lightweight, cross-platform, and highly secure.

Follow this guide to get set up for local development and to align with our codebase philosophies.

---

## 🧭 Core Development Philosophies

Before writing code, please read and respect our core technical rules:

### 📁 1. Root Clean Slate & Directory Structure
To keep our repository root exceptionally clean:
* **All source code** must reside within the `src/` directory.
* **Do not** add temporary or untracked state directories to the project root.
* **Workspace Crates:**
  * `src/reminder_core`: Compilation target for shared UniFFI bindings, models, logging, and storage engine.
  * `src/desktop`: Entry point for desktop targets, handling network sockets, tray loops, and OS toast integrations.
  * `src/tests`: Contains automated script runners and manual payload triggers.

### 🧹 2. Clean Slate Uninstallation
Any changes you introduce must comply with our **Clean Slate Philosophy**. If a feature registers state or configurations in the operating system (e.g., registries, files, launch agents), you **must** implement a corresponding teardown step in the uninstallation/cleanup routine:
* Windows: Cleans custom toast app associations, auto-start startup keys, and standard AppData files.
* Linux: Cleans `systemd` user service agents and standard share folders.
* macOS: Cleans `launchd` plist configurations and standard Application Support paths.

### 💾 3. Developer Environment Isolation
In debug builds (i.e. when running `cargo run` or `cargo test`), ensure that all created database files, logs, and diagnostic files are stored within the repository-local [dev/](dev/) directory. **Do not** pollute the local user's home directories or system AppData paths during active development.

---

## 🚀 Setting Up Your Development Environment

### 1. Compile Toolchain
1. **Windows:** Ensure you have **Visual Studio Build Tools 2022** installed with the **Desktop development with C++** workload and the **Windows 10/11 SDK** selected.
2. **Rustup:** Ensure you are running the latest stable Rust compiler.
   ```powershell
   rustup update stable
   ```

### 2. Workspace Cargo Commands
You can run standard Cargo commands from the repository root:
```powershell
# Compile the entire workspace
cargo build

# Run formatting checks
cargo fmt --all -- --check

# Run clippy analysis with compiler warnings treated as errors
cargo clippy --all-targets --all-features -- -D warnings

# Execute workspace unit tests
cargo test --all
```

---

## 🧪 Automation Check Script

To make verification easy, we have provided an automated checker script that runs cargo format verification, strict clippy analysis, and the core storage unit test suites in a single command. 

Ensure this script passes with **100% success** before proposing any changes:

* **Windows PowerShell:**
  ```powershell
  powershell -File .\src\tests\dev-check.ps1
  ```
* **Git Bash / Linux Shell:**
  ```bash
  ./src/tests/dev-check.bash
  ```

---

## 📝 Pull Request Checklist

When submitting your improvements, verify that you have completed the following:

1. **Prism-Perfect Formatting:** Run `cargo fmt --all` to format your changes.
2. **Clippy Lints:** Ensure no clippy warnings or compiler complaints are reported.
3. **No Unmanaged Leftovers:** Run `cargo run --release -- --cleanup` to confirm that all newly created assets are completely and cleanly deleted.
4. **Environment Routing:** Confirm that running the debug binary generates state files inside `dev/` and release builds generate them in persistent `AppData`.
5. **No git commands:** Please do not write files or invoke terminal processes that run arbitrary `git` commands inside your changes or automation testing logic.
