# Phase 1 Walkthrough: Rust Setup & Windows Compilation

This walkthrough outlines how to install Rust, compile the desktop daemon, and verify that it is fully operational on Windows.

---

## 1. Rust Installation Guide for Windows

Because Rust compiles to native machine code, it requires a C++ compiler toolchain on Windows. 

Follow these steps to set up your environment:

### Step 1: Install Visual Studio Build Tools
1. Download the [Visual Studio Community Edition or Build Tools](https://visualstudio.microsoft.com/downloads/).
2. During installation, select the **Desktop development with C++** workload.
3. Ensure the following individual components are selected (they are usually selected by default):
   - **MSVC v143 - VS 2022 C++ x64/x86 build tools**
   - **Windows 11 SDK** (or Windows 10 SDK)
4. Click install and wait for the installer to finish.

### Step 2: Install Rustup (The Rust Toolchain Installer)
1. Download **`rustup-init.exe`** from [https://rustup.rs](https://rustup.rs) (select the 64-bit version).
2. Run `rustup-init.exe`.
3. The terminal prompt will ask you to select an installation option. Type `1` (Proceed with standard installation) and press Enter.
4. Once completed, restart your terminal or command prompt to apply the system path variables.
5. Verify your installation by running:
   ```powershell
   rustc --version
   cargo --version
   ```

---

## 2. Compiling the Application

Once Rust is installed, compile the daemon:

1. Open PowerShell or a command prompt and navigate to the project directory:
   ```powershell
   cd d:\Codes\full-calendar-remastered-ReminderApp
   ```
2. Build the application in release mode for optimal performance and small size:
   ```powershell
   cargo build --release
   ```
3. The compiled binary will be generated at:
   `d:\Codes\full-calendar-remastered-ReminderApp\target\release\desktop.exe`

---

## 3. Running the Daemon

Run the daemon directly from your terminal:
```powershell
.\target\release\desktop.exe
```

When started, it will output:
```text
=== starting full-calendar-remastered reminder daemon ===
Storage path: C:\Users\<Username>\AppData\Local\fullcalendar\ReminderApp\reminders.json
Background scheduler started.
No active future reminders. Sleeping until next synchronization.
HTTP Server listening on: http://127.0.0.1:45677
```

---

## 4. Verification and Manual Testing

To confirm that the daemon, local storage, scheduler loop, and Windows native notifications are fully functional, you can run the following test commands in a separate PowerShell window:

### Test 1: Check Daemon Status
Query the status endpoint to verify the health of the daemon:
```powershell
Invoke-RestMethod -Uri "http://127.0.0.1:45677/status" -Method Get
```

**Expected Output:**
```json
{
  "status": "running",
  "active_reminders": 0,
  "database_path": "C:\\Users\\<Username>\\AppData\\Local\\fullcalendar\\ReminderApp\\reminders.json"
}
```

### Test 2: Trigger a Notification Test
We can simulate an Obsidian sync payload. We will construct a reminder scheduled **15 seconds in the future** so we can watch the scheduler wake up and trigger the native Windows Toast.

1. Run this PowerShell script to generate a dynamic Epoch target and POST the payload:
   ```powershell
   # Calculate target epoch 15 seconds from now
   $triggerTime = [DateTimeOffset]::UtcNow.AddSeconds(15).ToUnixTimeSeconds()

   # Define the JSON sync payload
   $payload = @(
       @{
           id = "test-event-999"
           title = "Hello from Obsidian!"
           body = "This is a native Windows toast notification triggered from the daemon."
           trigger_at_epoch = $triggerTime
           action_url = "obsidian://open"
       }
   ) | ConvertTo-Json -Depth 5

   # POST payload to our sync endpoint
   Invoke-RestMethod -Uri "http://127.0.0.1:45677/sync" -Method Post -Body $payload -ContentType "application/json"
   ```

2. **Watch the Daemon console:**
   - It will output: `Received Sync request: 1 reminders provided.`
   - Then: `Next reminder scheduled: "Hello from Obsidian!" in 15 seconds.`
   - After 15 seconds, it will wake up: `Reminder triggered! Firing notification for "Hello from Obsidian!".`
   - A standard, beautiful Windows Toast Notification will pop up in the corner of your screen!
