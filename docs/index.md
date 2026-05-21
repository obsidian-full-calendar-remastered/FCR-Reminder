# FCR Reminder Daemon

Welcome to the official documentation for the **FCR Reminder Daemon** (Full Calendar Remastered Reminder Daemon).

This is a lightweight, resource-efficient background service designed specifically to complement the [Full Calendar Remastered Obsidian Plugin](https://github.com/lucasvdh/obsidian-full-calendar). 

---

## The Challenge

Obsidian is a heavy desktop application and is **not built to run as a persistent background daemon**. When you close Obsidian, all internal timer loops and future event alert notifications are terminated. 

If you have a meeting or a highly time-sensitive event scheduled in your calendar, you will **miss the reminder** unless Obsidian remains running on your screen.

## The Solution: FCR Reminder

**FCR Reminder** is a dedicated background service written in Rust. It runs quietly in your system tray (or integrated into your mobile OS system alarms) and provides:

* 🚀 **Extremely Low Memory & CPU Footprint:** Built on pure, asynchronous Rust with Tokio.
* 📦 **Zero-Configuration Native Windows Toasts:** Employs Windows WinRT Toast Notifications with custom branding ("FCR Reminder") and an elegant clock/calendar icon out of the box.
* 🔒 **Local Security Bounded:** Binds strictly to `127.0.0.1:45677` so it is invisible and safe from the external network.
* 📲 **Dynamic Syncing:** The Obsidian plugin sends pre-computed future event epochs directly to FCR Reminder using a simple HTTP POST payload on desktops, or customized deep links on mobile.
* 🛡️ **Self-Healing Timer Queue:** Instantly wakes up and reschedules its entire sleep queue whenever it receives updates from Obsidian.

---

## Quick Navigation

* **[Architecture & Design Details](architecture/architecture.md):** Deep dive into the monorepo design, payload models, custom schemes, and cross-platform native wrapper designs.
* **[Windows Setup & Testing](user/windows_setup.md):** Comprehensive instructions on installing compile toolchains, building the binary, running the daemon, and triggering test notifications with PowerShell.
