use reminder_core::Reminder;
use std::error::Error;

/// Windows-specific startup initialization.
pub fn init() -> Result<(), Box<dyn Error>> {
    register_custom_app_id();
    register_autostart();
    register_custom_protocol();
    Ok(())
}

/// Windows-specific cleanup/uninstallation.
pub fn cleanup() -> Result<(), Box<dyn Error>> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 1. Remove Windows Registry AppUserModelId entry
    let subkey_path = "Software\\Classes\\AppUserModelId\\FCRReminder";
    if hkcu.open_subkey(subkey_path).is_ok() {
        match hkcu.delete_subkey(subkey_path) {
            Ok(_) => println!("Registry: Successfully removed 'FCRReminder' AppUserModelId from Windows Registry."),
            Err(e) => eprintln!("Registry Warning: Failed to remove registry subkey: {}", e),
        }
    } else {
        println!("Registry: No 'FCRReminder' AppUserModelId entries found (already clean).");
    }

    // 2. Remove Windows Startup Run entry
    let run_path = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    if let Ok(key) = hkcu.open_subkey_with_flags(run_path, winreg::enums::KEY_WRITE) {
        match key.delete_value("FCRReminder") {
            Ok(_) => println!(
                "Registry: Successfully removed 'FCRReminder' from Windows Startup Run entries."
            ),
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    eprintln!(
                        "Registry Warning: Failed to delete startup Run value: {}",
                        e
                    );
                } else {
                    println!("Registry: No Startup Run entry found (already clean).");
                }
            }
        }
    }

    // 3. Remove Windows Custom Protocol Handler entry
    let protocol_path = "Software\\Classes\\fcr-reminder";
    if hkcu.open_subkey(protocol_path).is_ok() {
        match hkcu.delete_subkey_all(protocol_path) {
            Ok(_) => println!("Registry: Successfully removed 'fcr-reminder' protocol handler from Windows Registry."),
            Err(e) => eprintln!("Registry Warning: Failed to remove registry protocol handler subkey: {}", e),
        }
    } else {
        println!("Registry: No 'fcr-reminder' protocol handler found (already clean).");
    }

    Ok(())
}

/// Dynamically hides the active console window on Windows.
pub fn hide_console() {
    use windows_sys::Win32::System::Console::GetConsoleWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

    let hwnd = unsafe { GetConsoleWindow() };
    if hwnd != 0 {
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}

/// Dispatches a rich interactive Windows Toast notification using Windows Runtime APIs.
pub fn trigger_notification(reminder: &Reminder) -> Result<(), Box<dyn Error>> {
    trigger_windows_toast(reminder)
}

/// Dedicated Windows background thread setup to run the Win32 message loop.
pub fn run_event_loop() {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, TranslateMessage, MSG,
        };
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// Registers the application in the Windows Registry to automatically run on user login.
fn register_autostart() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    if let Ok(current_exe) = std::env::current_exe() {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let subkey_path = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

        match hkcu.open_subkey_with_flags(subkey_path, winreg::enums::KEY_WRITE) {
            Ok(key) => {
                if let Err(e) =
                    key.set_value("FCRReminder", &current_exe.to_string_lossy().to_string())
                {
                    reminder_core::log_error!(
                        "Failed to register FCR Reminder in Startup Run registry key: {}",
                        e
                    );
                } else {
                    reminder_core::log_info!(
                        "Registered FCR Reminder for automatic Windows startup."
                    );
                }
            }
            Err(e) => {
                reminder_core::log_warn!(
                    "Failed to open Run registry key for startup registration: {}",
                    e
                );
            }
        }
    }
}

/// Registers the application as the custom protocol scheme handler for `fcr-reminder://`.
fn register_custom_protocol() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    if let Ok(current_exe) = std::env::current_exe() {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let subkey_path = "Software\\Classes\\fcr-reminder";

        match hkcu.create_subkey(subkey_path) {
            Ok((key, _)) => {
                let _ = key.set_value("", &"URL:FCR Reminder Protocol");
                let _ = key.set_value("URL Protocol", &"");

                match key.create_subkey("shell\\open\\command") {
                    Ok((shell_cmd, _)) => {
                        let cmd = format!("\"{}\" --uri \"%1\"", current_exe.to_string_lossy());
                        let _ = shell_cmd.set_value("", &cmd);
                        reminder_core::log_info!(
                            "Registered FCR Reminder custom protocol scheme handler."
                        );
                    }
                    Err(e) => {
                        reminder_core::log_error!(
                            "Failed to create shell command subkey for protocol handler: {}",
                            e
                        );
                    }
                }
            }
            Err(e) => {
                reminder_core::log_warn!(
                    "Failed to create custom Registry protocol handler subkey: {}",
                    e
                );
            }
        }
    }
}

/// Registers the AppUserModelId in Windows Registry under HKEY_CURRENT_USER.
fn register_custom_app_id() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    if let Some(app_dir) = reminder_core::get_app_dir() {
        let icon_path = app_dir.join("icon.png");
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let subkey_path = "Software\\Classes\\AppUserModelId\\FCRReminder";

        match hkcu.create_subkey(subkey_path) {
            Ok((key, _)) => {
                let _ = key.set_value("DisplayName", &"FCR Reminder");
                let _ = key.set_value("IconUri", &icon_path.to_string_lossy().to_string());
                reminder_core::log_info!(
                    "Registered custom AppUserModelId 'FCRReminder' in Windows Registry."
                );
            }
            Err(e) => {
                reminder_core::log_warn!("Failed to create custom Registry AppId subkey: {}", e);
            }
        }
    }
}

/// Builds and triggers Windows runtime XML Toast.
fn trigger_windows_toast(reminder: &Reminder) -> Result<(), Box<dyn Error>> {
    use windows::core::HSTRING;
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

    let app_id = HSTRING::from("FCRReminder");

    let title_esc = xml_escape(&reminder.title);
    let body_esc = xml_escape(&reminder.body);

    let id_enc = url_encode(&reminder.id);
    let title_enc = url_encode(&reminder.title);
    let body_enc = url_encode(&reminder.body);
    let action_url_enc = url_encode(&reminder.action_url);

    let snooze_args = format!(
        "fcr-reminder://snooze?id={}&title={}&body={}&action_url={}",
        id_enc, title_enc, body_enc, action_url_enc
    );
    let snooze_args_esc = xml_escape(&snooze_args);

    let open_note_action = if !reminder.action_url.is_empty() {
        let action_url_esc = xml_escape(&reminder.action_url);
        format!(
            "<action content=\"Open Note\" activationType=\"protocol\" arguments=\"{}\"/>",
            action_url_esc
        )
    } else {
        String::new()
    };

    let xml_content = format!(
        r#"<toast duration="long">
    <visual>
        <binding template="ToastGeneric">
            <text>{}</text>
            <text>{}</text>
        </binding>
    </visual>
    <audio src="ms-winsoundevent:Notification.Reminder"/>
    <actions>
        <input id="snoozeTime" type="selection" defaultInput="5">
            <selection id="5" content="5 minutes"/>
            <selection id="10" content="10 minutes"/>
            <selection id="15" content="15 minutes"/>
            <selection id="30" content="30 minutes"/>
            <selection id="60" content="1 hour"/>
        </input>
        <action
            content="Snooze"
            activationType="protocol"
            arguments="{}"
            hint-inputId="snoozeTime"/>
        {}
    </actions>
</toast>"#,
        title_esc, body_esc, snooze_args_esc, open_note_action
    );

    let xml_doc = XmlDocument::new()?;
    xml_doc.LoadXml(&HSTRING::from(&xml_content))?;

    let toast = ToastNotification::CreateToastNotification(&xml_doc)?;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&app_id)?;
    notifier.Show(&toast)?;

    Ok(())
}

/// Helper function to escape special XML characters.
fn xml_escape(input: &str) -> String {
    let mut escaped = String::new();
    for c in input.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

/// Helper function to URL-encode strings for protocol URIs.
fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}
