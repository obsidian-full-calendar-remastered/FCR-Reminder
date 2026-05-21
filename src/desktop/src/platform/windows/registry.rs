use std::error::Error;

const APP_ID_PATH: &str = "Software\\Classes\\AppUserModelId\\FCRReminder";
const RUN_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const PROTOCOL_PATH: &str = "Software\\Classes\\fcr-reminder";

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

    if hkcu.open_subkey(APP_ID_PATH).is_ok() {
        match hkcu.delete_subkey(APP_ID_PATH) {
            Ok(_) => println!("Registry: Successfully removed 'FCRReminder' AppUserModelId from Windows Registry."),
            Err(e) => eprintln!("Registry Warning: Failed to remove registry subkey: {}", e),
        }
    } else {
        println!("Registry: No 'FCRReminder' AppUserModelId entries found (already clean).");
    }

    if let Ok(key) = hkcu.open_subkey_with_flags(RUN_PATH, winreg::enums::KEY_WRITE) {
        match key.delete_value("FCRReminder") {
            Ok(_) => println!(
                "Registry: Successfully removed 'FCRReminder' from Windows Startup Run entries."
            ),
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("Registry Warning: Failed to delete startup Run value: {}", e);
                } else {
                    println!("Registry: No Startup Run entry found (already clean).");
                }
            }
        }
    }

    if hkcu.open_subkey(PROTOCOL_PATH).is_ok() {
        match hkcu.delete_subkey_all(PROTOCOL_PATH) {
            Ok(_) => println!("Registry: Successfully removed 'fcr-reminder' protocol handler from Windows Registry."),
            Err(e) => eprintln!("Registry Warning: Failed to remove registry protocol handler subkey: {}", e),
        }
    } else {
        println!("Registry: No 'fcr-reminder' protocol handler found (already clean).");
    }

    Ok(())
}

pub fn doctor_checks() -> Vec<(&'static str, bool)> {
    vec![
        ("windows_app_id_registered", is_app_id_registered()),
        ("windows_autostart_registered", is_autostart_registered()),
        ("windows_protocol_registered", is_custom_protocol_registered()),
    ]
}

fn register_autostart() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    if let Ok(current_exe) = std::env::current_exe() {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        match hkcu.open_subkey_with_flags(RUN_PATH, winreg::enums::KEY_WRITE) {
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

fn register_custom_protocol() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    if let Ok(current_exe) = std::env::current_exe() {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        match hkcu.create_subkey(PROTOCOL_PATH) {
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

fn register_custom_app_id() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    if let Some(app_dir) = reminder_core::get_app_dir() {
        let icon_path = app_dir.join("icon.png");
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        match hkcu.create_subkey(APP_ID_PATH) {
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

fn is_app_id_registered() -> bool {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    RegKey::predef(HKEY_CURRENT_USER).open_subkey(APP_ID_PATH).is_ok()
}

fn is_autostart_registered() -> bool {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(RUN_PATH)
        .ok()
        .and_then(|key| key.get_value::<String, _>("FCRReminder").ok())
        .is_some()
}

fn is_custom_protocol_registered() -> bool {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(PROTOCOL_PATH)
        .is_ok()
}