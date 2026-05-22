use crate::core::release_updates::UpdateStateSnapshot;
use std::error::Error;

const APP_ID_PATH: &str = "Software\\Classes\\AppUserModelId\\FCRReminder";
const RUN_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const PROTOCOL_PATH: &str = "Software\\Classes\\fcr-reminder";
const ABOUT_DIALOG_SCRIPT: &str = r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

function Add-InfoRow {
    param(
        [System.Windows.Forms.Control]$Parent,
        [string]$Label,
        [string]$Value,
        [int]$Top,
        [System.Drawing.Color]$LabelColor,
        [System.Drawing.Color]$ValueColor,
        [System.Drawing.Color]$BackgroundColor
    )

    $labelControl = New-Object System.Windows.Forms.Label
    $labelControl.Text = $Label
    $labelControl.Location = New-Object System.Drawing.Point(24, $Top)
    $labelControl.Size = New-Object System.Drawing.Size(120, 20)
    $labelControl.Font = New-Object System.Drawing.Font('Segoe UI Semibold', 9)
    $labelControl.ForeColor = $LabelColor
    $labelControl.BackColor = $BackgroundColor

    $valueControl = New-Object System.Windows.Forms.Label
    $valueControl.Text = $Value
    $valueControl.Location = New-Object System.Drawing.Point(150, $Top)
    $valueControl.Size = New-Object System.Drawing.Size(470, 34)
    $valueControl.Font = New-Object System.Drawing.Font('Segoe UI', 9)
    $valueControl.ForeColor = $ValueColor
    $valueControl.BackColor = $BackgroundColor
    $valueControl.AutoEllipsis = $true
    $valueControl.Anchor = [System.Windows.Forms.AnchorStyles]::Top -bor [System.Windows.Forms.AnchorStyles]::Left -bor [System.Windows.Forms.AnchorStyles]::Right

    $Parent.Controls.Add($labelControl)
    $Parent.Controls.Add($valueControl)
}

function New-ActionButton {
    param(
        [string]$Text,
        [int]$Left,
        [int]$Top,
        [int]$Width,
        [System.Drawing.Color]$BackColor,
        [System.Drawing.Color]$ForeColor,
        [System.Drawing.Color]$BorderColor
    )

    $button = New-Object System.Windows.Forms.Button
    $button.Text = $Text
    $button.Location = New-Object System.Drawing.Point($Left, $Top)
    $button.Size = New-Object System.Drawing.Size($Width, 34)
    $button.FlatStyle = 'Flat'
    $button.BackColor = $BackColor
    $button.ForeColor = $ForeColor
    $button.Font = New-Object System.Drawing.Font('Segoe UI Semibold', 9)
    $button.FlatAppearance.BorderColor = $BorderColor
    return $button
}

$themePath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize'
$lightTheme = $true
try {
    $lightTheme = ((Get-ItemProperty -Path $themePath -Name AppsUseLightTheme -ErrorAction Stop).AppsUseLightTheme -ne 0)
} catch {}

if ($lightTheme) {
    $backColor = [System.Drawing.Color]::FromArgb(246, 247, 249)
    $panelColor = [System.Drawing.Color]::White
    $foreColor = [System.Drawing.Color]::FromArgb(26, 26, 26)
    $mutedColor = [System.Drawing.Color]::FromArgb(96, 96, 96)
    $accentColor = [System.Drawing.Color]::FromArgb(11, 102, 195)
    $secondaryButton = [System.Drawing.Color]::FromArgb(240, 244, 248)
    $borderColor = [System.Drawing.Color]::FromArgb(218, 220, 224)
} else {
    $backColor = [System.Drawing.Color]::FromArgb(24, 24, 27)
    $panelColor = [System.Drawing.Color]::FromArgb(36, 36, 40)
    $foreColor = [System.Drawing.Color]::FromArgb(243, 243, 243)
    $mutedColor = [System.Drawing.Color]::FromArgb(180, 180, 184)
    $accentColor = [System.Drawing.Color]::FromArgb(96, 165, 250)
    $secondaryButton = [System.Drawing.Color]::FromArgb(52, 52, 56)
    $borderColor = [System.Drawing.Color]::FromArgb(70, 70, 73)
}

$form = New-Object System.Windows.Forms.Form
$form.Text = 'About FCR Reminder'
$form.StartPosition = 'CenterScreen'
$form.Size = New-Object System.Drawing.Size(900, 640)
$form.MinimumSize = New-Object System.Drawing.Size(780, 560)
$form.MaximizeBox = $true
$form.MinimizeBox = $false
$form.FormBorderStyle = 'Sizable'
$form.BackColor = $backColor
$form.ForeColor = $foreColor
$form.TopMost = $true

$heroPanel = New-Object System.Windows.Forms.Panel
$heroPanel.Location = New-Object System.Drawing.Point(18, 18)
$heroPanel.Size = New-Object System.Drawing.Size(848, 150)
$heroPanel.BackColor = $panelColor
$heroPanel.BorderStyle = 'FixedSingle'

$iconBox = New-Object System.Windows.Forms.PictureBox
$iconBox.Location = New-Object System.Drawing.Point(24, 24)
$iconBox.Size = New-Object System.Drawing.Size(84, 84)
$iconBox.SizeMode = 'Zoom'
$iconBox.BackColor = $panelColor
if ($env:FCR_REMINDER_ICON_PATH -and (Test-Path $env:FCR_REMINDER_ICON_PATH)) {
    $iconBox.Image = [System.Drawing.Image]::FromFile($env:FCR_REMINDER_ICON_PATH)
}

$header = New-Object System.Windows.Forms.Label
$header.Text = 'FCR Reminder'
$header.Location = New-Object System.Drawing.Point(114, 24)
$header.Size = New-Object System.Drawing.Size(340, 34)
$header.Font = New-Object System.Drawing.Font('Segoe UI Semibold', 18)
$header.ForeColor = $accentColor
$header.BackColor = $panelColor

$subheader = New-Object System.Windows.Forms.Label
$subheader.Text = $env:FCR_REMINDER_DESCRIPTION
$subheader.Location = New-Object System.Drawing.Point(116, 58)
$subheader.Size = New-Object System.Drawing.Size(690, 26)
$subheader.Font = New-Object System.Drawing.Font('Segoe UI', 9.5)
$subheader.ForeColor = $mutedColor
$subheader.BackColor = $panelColor
$subheader.AutoEllipsis = $true

$versionBadge = New-Object System.Windows.Forms.Label
$versionBadge.Text = "Version $env:FCR_REMINDER_VERSION"
$versionBadge.Location = New-Object System.Drawing.Point(116, 96)
$versionBadge.Size = New-Object System.Drawing.Size(140, 22)
$versionBadge.Font = New-Object System.Drawing.Font('Segoe UI Semibold', 8.5)
$versionBadge.ForeColor = [System.Drawing.Color]::White
$versionBadge.BackColor = $accentColor
$versionBadge.TextAlign = 'MiddleCenter'

$summaryLabel = New-Object System.Windows.Forms.Label
$summaryLabel.Text = 'Background companion for Full Calendar Remastered. Native tray app, local reminder storage, and loopback-only integration.'
$summaryLabel.Location = New-Object System.Drawing.Point(276, 92)
$summaryLabel.Size = New-Object System.Drawing.Size(530, 40)
$summaryLabel.Font = New-Object System.Drawing.Font('Segoe UI', 8.5)
$summaryLabel.ForeColor = $mutedColor
$summaryLabel.BackColor = $panelColor
$summaryLabel.AutoEllipsis = $true

$detailsPanel = New-Object System.Windows.Forms.Panel
$detailsPanel.Location = New-Object System.Drawing.Point(18, 184)
$detailsPanel.Size = New-Object System.Drawing.Size(848, 320)
$detailsPanel.BackColor = $panelColor
$detailsPanel.BorderStyle = 'FixedSingle'

$detailsTitle = New-Object System.Windows.Forms.Label
$detailsTitle.Text = 'Application details'
$detailsTitle.Location = New-Object System.Drawing.Point(24, 16)
$detailsTitle.Size = New-Object System.Drawing.Size(220, 20)
$detailsTitle.Font = New-Object System.Drawing.Font('Segoe UI Semibold', 10)
$detailsTitle.ForeColor = $foreColor
$detailsTitle.BackColor = $panelColor

$actionPanel = New-Object System.Windows.Forms.Panel
$actionPanel.Location = New-Object System.Drawing.Point(18, 520)
$actionPanel.Size = New-Object System.Drawing.Size(848, 54)
$actionPanel.BackColor = $backColor

$repoButton = New-ActionButton -Text 'Repository' -Left 0 -Top 10 -Width 130 -BackColor $secondaryButton -ForeColor $foreColor -BorderColor $borderColor
$issuesButton = New-ActionButton -Text 'Issues' -Left 144 -Top 10 -Width 110 -BackColor $secondaryButton -ForeColor $foreColor -BorderColor $borderColor
$featureButton = New-ActionButton -Text 'Feature Request' -Left 268 -Top 10 -Width 150 -BackColor $secondaryButton -ForeColor $foreColor -BorderColor $borderColor
$updatesButton = New-ActionButton -Text $env:FCR_REMINDER_UPDATE_BUTTON_TEXT -Left 432 -Top 10 -Width 170 -BackColor $secondaryButton -ForeColor $foreColor -BorderColor $borderColor

$okButton = New-ActionButton -Text 'OK' -Left 748 -Top 10 -Width 100 -BackColor $accentColor -ForeColor ([System.Drawing.Color]::White) -BorderColor $borderColor
$okButton.DialogResult = [System.Windows.Forms.DialogResult]::OK

$repoButton.Add_Click({ Start-Process $env:FCR_REMINDER_REPOSITORY_URL })
$issuesButton.Add_Click({ Start-Process $env:FCR_REMINDER_ISSUES_URL })
$featureButton.Add_Click({ Start-Process $env:FCR_REMINDER_FEATURE_URL })
$updatesButton.Add_Click({ if ($env:FCR_REMINDER_UPDATE_URL) { Start-Process $env:FCR_REMINDER_UPDATE_URL } })

function Update-AboutLayout {
    $margin = 18
    $gap = 16
    $actionHeight = 54
    $heroHeight = [Math]::Max(150, [Math]::Min(220, [int]([Math]::Floor($form.ClientSize.Height * 0.30))))
    $contentWidth = $form.ClientSize.Width - ($margin * 2)
    $heroPanel.Location = New-Object System.Drawing.Point($margin, $margin)
    $heroPanel.Size = New-Object System.Drawing.Size($contentWidth, $heroHeight)

    $headerLeft = 132
    $rightInset = 24
    $textWidth = [Math]::Max(320, $heroPanel.Width - $headerLeft - $rightInset)
    $subheader.Size = New-Object System.Drawing.Size($textWidth, 26)
    $summaryWidth = [Math]::Max(280, $heroPanel.Width - 300)
    $summaryLabel.Size = New-Object System.Drawing.Size($summaryWidth, 40)

    $detailsTop = $heroPanel.Bottom + $gap
    $actionTop = $form.ClientSize.Height - $margin - $actionHeight
    $detailsHeight = [Math]::Max(250, $actionTop - $gap - $detailsTop)
    $detailsPanel.Location = New-Object System.Drawing.Point($margin, $detailsTop)
    $detailsPanel.Size = New-Object System.Drawing.Size($contentWidth, $detailsHeight)

    $actionPanel.Location = New-Object System.Drawing.Point($margin, $actionTop)
    $actionPanel.Size = New-Object System.Drawing.Size($contentWidth, $actionHeight)
    $okButton.Location = New-Object System.Drawing.Point(($actionPanel.Width - $okButton.Width), 10)
}

if ($env:FCR_REMINDER_ICON_PATH -and (Test-Path $env:FCR_REMINDER_ICON_PATH)) {
    try {
        $form.Icon = [System.Drawing.Icon]::ExtractAssociatedIcon($env:FCR_REMINDER_EXECUTABLE)
    } catch {}
}

Add-InfoRow -Parent $detailsPanel -Label 'Version' -Value $env:FCR_REMINDER_VERSION -Top 48 -LabelColor $mutedColor -ValueColor $foreColor -BackgroundColor $panelColor
Add-InfoRow -Parent $detailsPanel -Label 'License' -Value $env:FCR_REMINDER_LICENSE -Top 78 -LabelColor $mutedColor -ValueColor $foreColor -BackgroundColor $panelColor
Add-InfoRow -Parent $detailsPanel -Label 'Runtime' -Value 'Rust desktop tray daemon' -Top 108 -LabelColor $mutedColor -ValueColor $foreColor -BackgroundColor $panelColor
Add-InfoRow -Parent $detailsPanel -Label 'Executable' -Value $env:FCR_REMINDER_EXECUTABLE -Top 138 -LabelColor $mutedColor -ValueColor $foreColor -BackgroundColor $panelColor
Add-InfoRow -Parent $detailsPanel -Label 'Storage' -Value $env:FCR_REMINDER_STORAGE -Top 168 -LabelColor $mutedColor -ValueColor $foreColor -BackgroundColor $panelColor
Add-InfoRow -Parent $detailsPanel -Label 'Update Status' -Value $env:FCR_REMINDER_UPDATE_STATUS -Top 198 -LabelColor $mutedColor -ValueColor $foreColor -BackgroundColor $panelColor
Add-InfoRow -Parent $detailsPanel -Label 'Latest Release' -Value $env:FCR_REMINDER_UPDATE_VERSION -Top 228 -LabelColor $mutedColor -ValueColor $foreColor -BackgroundColor $panelColor
Add-InfoRow -Parent $detailsPanel -Label 'Last Check' -Value $env:FCR_REMINDER_UPDATE_CHECKED -Top 258 -LabelColor $mutedColor -ValueColor $foreColor -BackgroundColor $panelColor

$heroPanel.Controls.Add($iconBox)
$heroPanel.Controls.Add($header)
$heroPanel.Controls.Add($subheader)
$heroPanel.Controls.Add($versionBadge)
$heroPanel.Controls.Add($summaryLabel)
$detailsPanel.Controls.Add($detailsTitle)
$actionPanel.Controls.Add($repoButton)
$actionPanel.Controls.Add($issuesButton)
$actionPanel.Controls.Add($featureButton)
$actionPanel.Controls.Add($updatesButton)
$actionPanel.Controls.Add($okButton)
$form.Controls.Add($heroPanel)
$form.Controls.Add($detailsPanel)
$form.Controls.Add($actionPanel)
$form.AcceptButton = $okButton
$form.CancelButton = $okButton
$form.Add_Shown({ Update-AboutLayout })
$form.Add_Resize({ Update-AboutLayout })

[void]$form.ShowDialog()
"#;

/// Windows-specific startup initialization.
pub fn init() -> Result<(), Box<dyn Error>> {
    register_custom_app_id_if_missing();
    register_autostart_if_missing();
    register_custom_protocol_if_missing();
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

pub fn show_about_dialog(update_state: &UpdateStateSnapshot) -> Result<(), Box<dyn Error>> {
    let repository_url = env!("CARGO_PKG_REPOSITORY");
    let issues_url = build_issues_url(repository_url);
    let feature_request_url = build_feature_request_url(repository_url);
    let executable_path = std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "Unavailable".to_string());
    let storage_path = crate::core::get_storage_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "Unavailable".to_string());
    let icon_path = crate::core::get_app_dir()
        .map(|dir| dir.join("icon.png").display().to_string())
        .unwrap_or_default();
    let update_version = update_state
        .latest_release
        .as_ref()
        .map(|release| release.version.clone())
        .unwrap_or_else(|| "Unavailable".to_string());
    let update_checked = if update_state.last_checked_at_epoch > 0 {
        chrono::DateTime::<chrono::Utc>::from_timestamp(update_state.last_checked_at_epoch, 0)
            .map(|timestamp| timestamp.to_rfc3339())
            .unwrap_or_else(|| "Unavailable".to_string())
    } else {
        "Pending".to_string()
    };
    let update_button_text = if update_state.update_available {
        "Update Available"
    } else {
        "Latest Release"
    };

    std::process::Command::new("powershell")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-WindowStyle")
        .arg("Hidden")
        .arg("-Command")
        .arg(about_dialog_script())
        .env("FCR_REMINDER_VERSION", env!("CARGO_PKG_VERSION"))
        .env("FCR_REMINDER_DESCRIPTION", env!("CARGO_PKG_DESCRIPTION"))
        .env("FCR_REMINDER_LICENSE", env!("CARGO_PKG_LICENSE"))
        .env("FCR_REMINDER_REPOSITORY_URL", repository_url)
        .env("FCR_REMINDER_ISSUES_URL", issues_url)
        .env("FCR_REMINDER_FEATURE_URL", feature_request_url)
        .env("FCR_REMINDER_EXECUTABLE", executable_path)
        .env("FCR_REMINDER_STORAGE", storage_path)
        .env("FCR_REMINDER_ICON_PATH", icon_path)
        .env("FCR_REMINDER_UPDATE_STATUS", &update_state.status_label)
        .env("FCR_REMINDER_UPDATE_VERSION", update_version)
        .env("FCR_REMINDER_UPDATE_CHECKED", update_checked)
        .env("FCR_REMINDER_UPDATE_URL", update_state.action_url())
        .env("FCR_REMINDER_UPDATE_BUTTON_TEXT", update_button_text)
        .spawn()
        .map(|_| ())
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

    pub fn open_url(url: &str) -> Result<(), Box<dyn Error>> {
        let escaped = url.replace('\'', "''");
        std::process::Command::new("powershell")
        .arg("-NoProfile")
        .arg("-WindowStyle")
        .arg("Hidden")
        .arg("-Command")
        .arg(format!("Start-Process '{}'", escaped))
        .spawn()
        .map(|_| ())
        .map_err(|error| Box::new(error) as Box<dyn Error>)
    }

fn build_issues_url(repository_url: &str) -> String {
    format!("{}/issues", repository_url.trim_end_matches('/'))
}

fn build_feature_request_url(repository_url: &str) -> String {
    format!(
        "{}/issues/new/choose",
        repository_url.trim_end_matches('/')
    )
}

fn about_dialog_script() -> &'static str {
    ABOUT_DIALOG_SCRIPT
}

fn register_autostart_if_missing() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    if is_autostart_registered() {
        crate::log_info!("FCR Reminder startup Run entry already exists. Skipping registration.");
        return;
    }

    if let Ok(current_exe) = std::env::current_exe() {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        match hkcu.open_subkey_with_flags(RUN_PATH, winreg::enums::KEY_WRITE) {
            Ok(key) => {
                if let Err(e) =
                    key.set_value("FCRReminder", &current_exe.to_string_lossy().to_string())
                {
                    crate::log_error!(
                        "Failed to register FCR Reminder in Startup Run registry key: {}",
                        e
                    );
                } else {
                    crate::log_info!(
                        "Registered FCR Reminder for automatic Windows startup."
                    );
                }
            }
            Err(e) => {
                crate::log_warn!(
                    "Failed to open Run registry key for startup registration: {}",
                    e
                );
            }
        }
    }
}

fn register_custom_protocol_if_missing() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    if let Ok(current_exe) = std::env::current_exe() {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let expected_command = protocol_command_value(&current_exe);

        if current_protocol_command().as_deref() == Some(expected_command.as_str()) {
            crate::log_info!(
                "FCR Reminder protocol handler already matches the current executable. Skipping registration."
            );
            return;
        }

        match hkcu.create_subkey(PROTOCOL_PATH) {
            Ok((key, _)) => {
                let _ = key.set_value("", &"URL:FCR Reminder Protocol");
                let _ = key.set_value("URL Protocol", &"");

                match key.create_subkey("shell\\open\\command") {
                    Ok((shell_cmd, _)) => {
                        let _ = shell_cmd.set_value("", &expected_command);
                        crate::log_info!(
                            "Registered or refreshed the FCR Reminder custom protocol scheme handler."
                        );
                    }
                    Err(e) => {
                        crate::log_error!(
                            "Failed to create shell command subkey for protocol handler: {}",
                            e
                        );
                    }
                }
            }
            Err(e) => {
                crate::log_warn!(
                    "Failed to create custom Registry protocol handler subkey: {}",
                    e
                );
            }
        }
    }
}

fn register_custom_app_id_if_missing() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    if is_app_id_registered() {
        crate::log_info!("FCR Reminder AppUserModelId already exists. Skipping registration.");
        return;
    }

    if let Some(app_dir) = crate::core::get_app_dir() {
        let icon_path = app_dir.join("icon.png");
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        match hkcu.create_subkey(APP_ID_PATH) {
            Ok((key, _)) => {
                let _ = key.set_value("DisplayName", &"FCR Reminder");
                let _ = key.set_value("IconUri", &icon_path.to_string_lossy().to_string());
                crate::log_info!(
                    "Registered custom AppUserModelId 'FCRReminder' in Windows Registry."
                );
            }
            Err(e) => {
                crate::log_warn!("Failed to create custom Registry AppId subkey: {}", e);
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

fn current_protocol_command() -> Option<String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(format!("{}\\shell\\open\\command", PROTOCOL_PATH))
        .ok()
        .and_then(|key| key.get_value::<String, _>("").ok())
}

fn protocol_command_value(current_exe: &std::path::Path) -> String {
    format!("\"{}\" --uri \"%1\"", current_exe.to_string_lossy())
}
