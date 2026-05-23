use crate::core::gui::GuiContext;
use std::error::Error;
use std::os::windows::process::CommandExt;

const EVENTS_DIALOG_SCRIPT: &str = r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

# Determine Light/Dark Theme
$themePath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize'
$lightTheme = $true
try {
    $lightTheme = ((Get-ItemProperty -Path $themePath -Name AppsUseLightTheme -ErrorAction Stop).AppsUseLightTheme -ne 0)
} catch {}

if ($lightTheme) {
    $backColor = [System.Drawing.Color]::FromArgb(244, 245, 247)
    $panelColor = [System.Drawing.Color]::White
    $foreColor = [System.Drawing.Color]::FromArgb(24, 24, 27)
    $mutedColor = [System.Drawing.Color]::FromArgb(113, 113, 122)
    $accentColor = [System.Drawing.Color]::FromArgb(37, 99, 235)
    $accentLight = [System.Drawing.Color]::FromArgb(239, 246, 255)
    $badgeFore = [System.Drawing.Color]::FromArgb(29, 78, 216)
    $secondaryButton = [System.Drawing.Color]::FromArgb(244, 244, 245)
    $borderColor = [System.Drawing.Color]::FromArgb(228, 228, 231)
    $cardBorderColor = [System.Drawing.Color]::FromArgb(212, 212, 216)
    $buttonBorderColor = [System.Drawing.Color]::FromArgb(228, 228, 231)
    $dangerColor = [System.Drawing.Color]::FromArgb(239, 68, 68)
} else {
    $backColor = [System.Drawing.Color]::FromArgb(9, 9, 11)
    $panelColor = [System.Drawing.Color]::FromArgb(24, 24, 27)
    $foreColor = [System.Drawing.Color]::FromArgb(250, 250, 250)
    $mutedColor = [System.Drawing.Color]::FromArgb(161, 161, 170)
    $accentColor = [System.Drawing.Color]::FromArgb(96, 165, 250)
    $accentLight = [System.Drawing.Color]::FromArgb(30, 41, 59)
    $badgeFore = [System.Drawing.Color]::FromArgb(191, 219, 254)
    $secondaryButton = [System.Drawing.Color]::FromArgb(39, 39, 42)
    $borderColor = [System.Drawing.Color]::FromArgb(39, 39, 42)
    $cardBorderColor = [System.Drawing.Color]::FromArgb(63, 63, 70)
    $buttonBorderColor = [System.Drawing.Color]::FromArgb(63, 63, 70)
    $dangerColor = [System.Drawing.Color]::FromArgb(248, 113, 113)
}

# Main Form
$form = New-Object System.Windows.Forms.Form
$form.Text = 'FCR Scheduled Reminders'
$form.StartPosition = 'CenterScreen'
$form.Size = New-Object System.Drawing.Size(950, 680)
$form.MinimumSize = New-Object System.Drawing.Size(800, 500)
$form.BackColor = $backColor
$form.ForeColor = $foreColor
$form.Font = New-Object System.Drawing.Font('Segoe UI', 9)

if ($env:FCR_REMINDER_ICON_PATH -and (Test-Path $env:FCR_REMINDER_ICON_PATH)) {
    try {
        $form.Icon = [System.Drawing.Icon]::ExtractAssociatedIcon($env:FCR_REMINDER_EXECUTABLE)
    } catch {}
}

# Header Panel
$headerPanel = New-Object System.Windows.Forms.Panel
$headerPanel.Dock = [System.Windows.Forms.DockStyle]::Top
$headerPanel.Height = 84
$headerPanel.BackColor = $panelColor
$headerPanel.Paint += {
    param($sender, $e)
    $pen = New-Object System.Drawing.Pen($borderColor, 1)
    $e.Graphics.DrawLine($pen, 0, $sender.Height - 1, $sender.Width, $sender.Height - 1)
}

$iconBox = New-Object System.Windows.Forms.PictureBox
$iconBox.Location = New-Object System.Drawing.Point(20, 15)
$iconBox.Size = New-Object System.Drawing.Size(54, 54)
$iconBox.SizeMode = 'Zoom'
$iconBox.BackColor = $panelColor
if ($env:FCR_REMINDER_ICON_PATH -and (Test-Path $env:FCR_REMINDER_ICON_PATH)) {
    $iconBox.Image = [System.Drawing.Image]::FromFile($env:FCR_REMINDER_ICON_PATH)
}

$titleLabel = New-Object System.Windows.Forms.Label
$titleLabel.Text = 'Scheduled Calendar Reminders'
$titleLabel.Location = New-Object System.Drawing.Point(84, 15)
$titleLabel.Size = New-Object System.Drawing.Size(450, 28)
$titleLabel.Font = New-Object System.Drawing.Font('Segoe UI Semibold', 15)
$titleLabel.ForeColor = $accentColor
$titleLabel.BackColor = $panelColor

$subtitleLabel = New-Object System.Windows.Forms.Label
$subtitleLabel.Text = 'Active reminders synchronized from Obsidian. The background service manages desktop notifications.'
$subtitleLabel.Location = New-Object System.Drawing.Point(86, 44)
$subtitleLabel.Size = New-Object System.Drawing.Size(650, 20)
$subtitleLabel.Font = New-Object System.Drawing.Font('Segoe UI', 9)
$subtitleLabel.ForeColor = $mutedColor
$subtitleLabel.BackColor = $panelColor

$headerPanel.Controls.Add($iconBox)
$headerPanel.Controls.Add($titleLabel)
$headerPanel.Controls.Add($subtitleLabel)
$form.Controls.Add($headerPanel)

# Toolbar Panel
$toolbarPanel = New-Object System.Windows.Forms.Panel
$toolbarPanel.Dock = [System.Windows.Forms.DockStyle]::Top
$toolbarPanel.Height = 52
$toolbarPanel.BackColor = $backColor

$searchBox = New-Object System.Windows.Forms.TextBox
$searchBox.Location = New-Object System.Drawing.Point(20, 14)
$searchBox.Size = New-Object System.Drawing.Size(260, 24)
$searchBox.BackColor = $panelColor
$searchBox.ForeColor = $foreColor
# Adding modern search placeholder / border look
$searchBox.Add_TextChanged({ Filter-Cards })

$placeholderLabel = New-Object System.Windows.Forms.Label
$placeholderLabel.Text = 'Search reminders...'
$placeholderLabel.Location = New-Object System.Drawing.Point(24, 17)
$placeholderLabel.Size = New-Object System.Drawing.Size(180, 18)
$placeholderLabel.ForeColor = $mutedColor
$placeholderLabel.BackColor = $panelColor
$placeholderLabel.Cursor = [System.Windows.Forms.Cursors]::IBeam
$placeholderLabel.Add_Click({ $searchBox.Focus() })

$searchBox.Add_GotFocus({ $placeholderLabel.Visible = $false })
$searchBox.Add_LostFocus({ if ($searchBox.Text -eq '') { $placeholderLabel.Visible = $true } })

$btnStyle = {
    param($btn)
    $btn.FlatStyle = 'Flat'
    $btn.FlatAppearance.BorderColor = $buttonBorderColor
    $btn.FlatAppearance.BorderSize = 1
    $btn.BackColor = $panelColor
    $btn.ForeColor = $foreColor
    $btn.Font = New-Object System.Drawing.Font('Segoe UI Semibold', 9)
}

$refreshBtn = New-Object System.Windows.Forms.Button
$refreshBtn.Text = 'Refresh'
$refreshBtn.Location = New-Object System.Drawing.Point(290, 13)
$refreshBtn.Size = New-Object System.Drawing.Size(90, 26)
&$btnStyle $refreshBtn
$refreshBtn.Add_Click({ Load-Events })

$testBtn = New-Object System.Windows.Forms.Button
$testBtn.Text = 'Trigger Test'
$testBtn.Location = New-Object System.Drawing.Point(390, 13)
$testBtn.Size = New-Object System.Drawing.Size(110, 26)
&$btnStyle $testBtn
$testBtn.Add_Click({ Trigger-TestReminder })

$toolbarPanel.Controls.Add($placeholderLabel)
$toolbarPanel.Controls.Add($searchBox)
$toolbarPanel.Controls.Add($refreshBtn)
$toolbarPanel.Controls.Add($testBtn)
$form.Controls.Add($toolbarPanel)

# Footer / Status Panel
$statusPanel = New-Object System.Windows.Forms.Panel
$statusPanel.Dock = [System.Windows.Forms.DockStyle]::Bottom
$statusPanel.Height = 32
$statusPanel.BackColor = $panelColor
$statusPanel.Paint += {
    param($sender, $e)
    $pen = New-Object System.Drawing.Pen($borderColor, 1)
    $e.Graphics.DrawLine($pen, 0, 0, $sender.Width, 0)
}

$statusLabel = New-Object System.Windows.Forms.Label
$statusLabel.Text = 'Connecting to daemon...'
$statusLabel.Location = New-Object System.Drawing.Point(20, 8)
$statusLabel.Size = New-Object System.Drawing.Size(400, 20)
$statusLabel.ForeColor = $mutedColor
$statusLabel.Font = New-Object System.Drawing.Font('Segoe UI', 8.5)

$storageLabel = New-Object System.Windows.Forms.Label
$storageLabel.Text = ''
$storageLabel.Location = New-Object System.Drawing.Point(450, 8)
$storageLabel.Size = New-Object System.Drawing.Size(480, 20)
$storageLabel.ForeColor = $mutedColor
$storageLabel.TextAlign = 'Right'
$storageLabel.Anchor = [System.Windows.Forms.AnchorStyles]::Top -bor [System.Windows.Forms.AnchorStyles]::Right
$storageLabel.Font = New-Object System.Drawing.Font('Segoe UI', 8.5)

$statusPanel.Controls.Add($statusLabel)
$statusPanel.Controls.Add($storageLabel)
$form.Controls.Add($statusPanel)

# Main Events Container
$eventsFlowPanel = New-Object System.Windows.Forms.FlowLayoutPanel
$eventsFlowPanel.Dock = [System.Windows.Forms.DockStyle]::Fill
$eventsFlowPanel.AutoScroll = $true
$eventsFlowPanel.WrapContents = $false
$eventsFlowPanel.FlowDirection = [System.Windows.Forms.FlowDirection]::TopDown
$eventsFlowPanel.Padding = New-Object System.Windows.Forms.Padding(20, 10, 20, 10)
$form.Controls.Add($eventsFlowPanel)

# Helper function to compute relative time text
function Get-RelativeTimeText($seconds) {
    if ($seconds -gt 0) {
        $minutes = [Math]::Floor($seconds / 60)
        $hours = [Math]::Floor($minutes / 60)
        $days = [Math]::Floor($hours / 24)
        if ($days -gt 0) { return "in $days day" + ($days -gt 1 ? "s" : "") }
        if ($hours -gt 0) { return "in $hours hour" + ($hours -gt 1 ? "s" : "") }
        if ($minutes -gt 0) { return "in $minutes min" + ($minutes -gt 1 ? "s" : "") }
        return "in $seconds sec"
    } else {
        $absSec = [Math]::Abs($seconds)
        $minutes = [Math]::Floor($absSec / 60)
        $hours = [Math]::Floor($minutes / 60)
        $days = [Math]::Floor($hours / 24)
        if ($days -gt 0) { return "$days day" + ($days -gt 1 ? "s" : "") + " ago" }
        if ($hours -gt 0) { return "$hours hour" + ($hours -gt 1 ? "s" : "") + " ago" }
        if ($minutes -gt 0) { return "$minutes min" + ($minutes -gt 1 ? "s" : "") + " ago" }
        return "just now"
    }
}

# Main data loader
$allReminders = @()

function Load-Events {
    $eventsFlowPanel.Controls.Clear()
    $statusLabel.Text = 'Loading reminders from background service...'
    $form.UseWaitCursor = $true

    try {
        $url = "$env:FCR_REMINDER_API_URL/events"
        $response = Invoke-RestMethod -Uri $url -Method Get -TimeoutSec 3
        
        $global:allReminders = $response.events
        
        # Display storage details in status bar
        if ($response.storage) {
            $storageLabel.Text = "Database: " + $response.storage.storage_path
        }

        Render-Cards
        $statusLabel.Text = "Total active reminders: $($global:allReminders.Count)"
    } catch {
        $statusLabel.Text = 'Error: Companion app daemon is unreachable.'
        Show-ErrorCard
    } finally {
        $form.UseWaitCursor = $false
    }
}

function Show-ErrorCard {
    $errorCard = New-Object System.Windows.Forms.Panel
    $errorCard.Size = New-Object System.Drawing.Size($eventsFlowPanel.ClientSize.Width - 46, 120)
    $errorCard.BackColor = $panelColor
    $errorCard.BorderStyle = 'FixedSingle'
    
    $errTitle = New-Object System.Windows.Forms.Label
    $errTitle.Text = 'Daemon Connection Error'
    $errTitle.Location = New-Object System.Drawing.Point(20, 20)
    $errTitle.Size = New-Object System.Drawing.Size(400, 24)
    $errTitle.Font = New-Object System.Drawing.Font('Segoe UI Semibold', 12)
    $errTitle.ForeColor = $dangerColor
    
    $errText = New-Object System.Windows.Forms.Label
    $errText.Text = "FCR Reminder is a tray companion application. Ensure fcr-reminder is running in your system tray on port 45677.`nIf it is closed, click 'Start Daemon' or start it via Obsidian."
    $errText.Location = New-Object System.Drawing.Point(20, 50)
    $errText.Size = New-Object System.Drawing.Size(650, 48)
    $errText.ForeColor = $mutedColor

    $startBtn = New-Object System.Windows.Forms.Button
    $startBtn.Text = 'Start Daemon'
    $startBtn.Location = New-Object System.Drawing.Point(680, 45)
    $startBtn.Size = New-Object System.Drawing.Size(120, 30)
    &$btnStyle $startBtn
    $startBtn.Add_Click({
        Start-Process $env:FCR_REMINDER_EXECUTABLE
        Start-Sleep -Seconds 1
        Load-Events
    })
    
    $errorCard.Controls.Add($errTitle)
    $errorCard.Controls.Add($errText)
    $errorCard.Controls.Add($startBtn)
    $eventsFlowPanel.Controls.Add($errorCard)
}

function Render-Cards {
    $eventsFlowPanel.Controls.Clear()
    $filterText = $searchBox.Text.ToLower().Trim()
    
    $filtered = $global:allReminders | Where-Object {
        $_.title.ToLower().Contains($filterText) -or $_.body.ToLower().Contains($filterText)
    }

    if ($filtered.Count -eq 0 -or $filtered -eq $null) {
        $emptyCard = New-Object System.Windows.Forms.Panel
        $emptyCard.Size = New-Object System.Drawing.Size($eventsFlowPanel.ClientSize.Width - 46, 120)
        $emptyCard.BackColor = $panelColor
        $emptyCard.BorderStyle = 'FixedSingle'
        
        $emptyTitle = New-Object System.Windows.Forms.Label
        $emptyTitle.Text = $searchBox.Text -eq '' ? 'No Active Reminders' : 'No Matching Reminders'
        $emptyTitle.Location = New-Object System.Drawing.Point(20, 30)
        $emptyTitle.Size = New-Object System.Drawing.Size(400, 24)
        $emptyTitle.Font = New-Object System.Drawing.Font('Segoe UI Semibold', 12)
        $emptyTitle.ForeColor = $accentColor
        
        $emptyText = New-Object System.Windows.Forms.Label
        $emptyText.Text = $searchBox.Text -eq '' ? 'Add new calendar events with reminders in Obsidian to see them synchronized here.' : 'Try adjusting your search criteria.'
        $emptyText.Location = New-Object System.Drawing.Point(20, 60)
        $emptyText.Size = New-Object System.Drawing.Size(700, 30)
        $emptyText.ForeColor = $mutedColor
        
        $emptyCard.Controls.Add($emptyTitle)
        $emptyCard.Controls.Add($emptyText)
        $eventsFlowPanel.Controls.Add($emptyCard)
        return
    }

    foreach ($reminder in $filtered) {
        $card = New-Object System.Windows.Forms.Panel
        $card.Size = New-Object System.Drawing.Size($eventsFlowPanel.ClientSize.Width - 46, 114)
        $card.BackColor = $panelColor
        $card.Margin = New-Object System.Windows.Forms.Padding(0, 0, 0, 12)
        $card.Paint += {
            param($sender, $e)
            $rect = New-Object System.Drawing.Rectangle(0, 0, $sender.Width - 1, $sender.Height - 1)
            $pen = New-Object System.Drawing.Pen($cardBorderColor, 1)
            $e.Graphics.DrawRectangle($pen, $rect)
        }

        # Title
        $title = New-Object System.Windows.Forms.Label
        $title.Text = $reminder.title
        $title.Location = New-Object System.Drawing.Point(16, 12)
        $title.Size = New-Object System.Drawing.Size($card.Width - 220, 22)
        $title.Font = New-Object System.Drawing.Font('Segoe UI Semibold', 11)
        $title.ForeColor = $foreColor
        $title.BackColor = $panelColor
        $title.AutoEllipsis = $true

        # Body/Description
        $body = New-Object System.Windows.Forms.Label
        $body.Text = $reminder.body -eq '' ? '(No description provided)' : $reminder.body
        $body.Location = New-Object System.Drawing.Point(16, 38)
        $body.Size = New-Object System.Drawing.Size($card.Width - 220, 40)
        $body.ForeColor = $mutedColor
        $body.BackColor = $panelColor
        $body.AutoEllipsis = $true

        # Countdown Badge
        $badgeText = Get-RelativeTimeText $reminder.seconds_until_fire
        $badge = New-Object System.Windows.Forms.Label
        $badge.Text = $badgeText
        $badge.Location = New-Object System.Drawing.Point($card.Width - 190, 12)
        $badge.Size = New-Object System.Drawing.Size(170, 22)
        $badge.Font = New-Object System.Drawing.Font('Segoe UI Semibold', 8.5)
        $badge.ForeColor = $badgeFore
        $badge.BackColor = $accentLight
        $badge.TextAlign = 'MiddleCenter'
        $badge.Anchor = [System.Windows.Forms.AnchorStyles]::Top -bor [System.Windows.Forms.AnchorStyles]::Right

        # Local absolute trigger time
        $localTime = [DateTime]::Parse($reminder.trigger_at_rfc3339).ToLocalTime().ToString('yyyy-MM-dd HH:mm:ss')
        $timeInfo = New-Object System.Windows.Forms.Label
        $timeInfo.Text = "Triggers: $localTime"
        $timeInfo.Location = New-Object System.Drawing.Point(16, 84)
        $timeInfo.Size = New-Object System.Drawing.Size(400, 18)
        $timeInfo.ForeColor = $mutedColor
        $timeInfo.BackColor = $panelColor
        $timeInfo.Font = New-Object System.Drawing.Font('Segoe UI', 8)

        # Action Buttons container (bottom-right)
        $actionPanel = New-Object System.Windows.Forms.Panel
        $actionPanel.Size = New-Object System.Drawing.Size(350, 32)
        $actionPanel.Location = New-Object System.Drawing.Point($card.Width - 370, 72)
        $actionPanel.Anchor = [System.Windows.Forms.AnchorStyles]::Bottom -bor [System.Windows.Forms.AnchorStyles]::Right
        $actionPanel.BackColor = $panelColor

        $btnX = 350
        
        # Dismiss/Delete
        $dismissBtn = New-Object System.Windows.Forms.Button
        $dismissBtn.Text = 'Dismiss'
        $btnX -= 80
        $dismissBtn.Location = New-Object System.Drawing.Point($btnX, 2)
        $dismissBtn.Size = New-Object System.Drawing.Size(74, 26)
        &$btnStyle $dismissBtn
        $dismissBtn.FlatAppearance.MouseOverBackColor = [System.Drawing.Color]::FromArgb(254, 242, 242)
        $dismissBtn.ForeColor = $dangerColor
        $remId = $reminder.id
        $dismissBtn.Add_Click({ Dismiss-Reminder $remId })

        # Snooze dropdown/context menu trigger
        $snoozeBtn = New-Object System.Windows.Forms.Button
        $snoozeBtn.Text = 'Snooze ▾'
        $btnX -= 90
        $snoozeBtn.Location = New-Object System.Drawing.Point($btnX, 2)
        $snoozeBtn.Size = New-Object System.Drawing.Size(84, 26)
        &$btnStyle $snoozeBtn
        
        # Build Snooze context menu
        $ctxMenu = New-Object System.Windows.Forms.ContextMenuStrip
        $snoozeMinutes = @(5, 15, 30, 60, 1440)
        $snoozeLabels = @('5 minutes', '15 minutes', '30 minutes', '1 hour', '1 day')
        for ($i = 0; $i -lt $snoozeMinutes.Count; $i++) {
            $min = $snoozeMinutes[$i]
            $item = $ctxMenu.Items.Add($snoozeLabels[$i])
            $r = $reminder
            $item.Add_Click({ Snooze-Reminder $r.id $r.title $r.body $r.action_url $min })
        }
        $snoozeBtn.Add_Click({ $ctxMenu.Show($snoozeBtn, 0, $snoozeBtn.Height) })

        $actionPanel.Controls.Add($dismissBtn)
        $actionPanel.Controls.Add($snoozeBtn)

        # Open URL button (if url is set)
        if ($reminder.action_url -and $reminder.action_url.Trim() -ne '') {
            $urlBtn = New-Object System.Windows.Forms.Button
            $urlBtn.Text = 'Open'
            $btnX -= 70
            $urlBtn.Location = New-Object System.Drawing.Point($btnX, 2)
            $urlBtn.Size = New-Object System.Drawing.Size(64, 26)
            &$btnStyle $urlBtn
            $url = $reminder.action_url
            $urlBtn.Add_Click({ Start-Process $url })
            $actionPanel.Controls.Add($urlBtn)
        }

        $card.Controls.Add($title)
        $card.Controls.Add($body)
        $card.Controls.Add($badge)
        $card.Controls.Add($timeInfo)
        $card.Controls.Add($actionPanel)
        $eventsFlowPanel.Controls.Add($card)
    }
}

function Filter-Cards {
    Render-Cards
}

# Snooze Action API call
function Snooze-Reminder($id, $title, $body, $actionUrl, $minutes) {
    try {
        $payload = @{
            id = $id
            title = $title
            body = $body
            action_url = $actionUrl
            minutes = [int64]$minutes
        } | ConvertTo-Json -Compress

        $headers = @{ "Content-Type" = "application/json" }
        Invoke-RestMethod -Uri "$env:FCR_REMINDER_API_URL/snooze" -Method Post -Body $payload -Headers $headers -TimeoutSec 3
        
        Load-Events
    } catch {
        [System.Windows.Forms.MessageBox]::Show("Failed to snooze reminder: $($_.Exception.Message)", "Error", [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Error)
    }
}

# Dismiss/Delete Action API call
function Dismiss-Reminder($id) {
    try {
        # Overwrite list without the dismissed event
        $payloadArray = @()
        foreach ($e in $global:allReminders) {
            if ($e.id -ne $id) {
                $payloadArray += @{
                    id = $e.id
                    title = $e.title
                    body = $e.body
                    trigger_at_epoch = $e.trigger_at_epoch
                    action_url = $e.action_url
                }
            }
        }
        $payloadJson = $payloadArray | ConvertTo-Json -Compress
        if ($payloadArray.Count -eq 0) { $payloadJson = "[]" }
        
        $headers = @{ "Content-Type" = "application/json" }
        Invoke-RestMethod -Uri "$env:FCR_REMINDER_API_URL/sync" -Method Post -Body $payloadJson -Headers $headers -TimeoutSec 3
        
        Load-Events
    } catch {
        [System.Windows.Forms.MessageBox]::Show("Failed to dismiss reminder: $($_.Exception.Message)", "Error", [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Error)
    }
}

# Seed/Test Reminder Trigger API call
function Trigger-TestReminder {
    try {
        $nowEpoch = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
        $testReminder = @{
            id = "test-reminder-gui"
            title = "🔔 Test Notification"
            body = "FCR Reminder is successfully configured and running in the background."
            trigger_at_epoch = $nowEpoch + 10
            action_url = "https://github.com/obsidian-full-calendar-remastered"
        }

        # Retrieve current events and filter out any existing test events
        $payloadArray = @()
        foreach ($e in $global:allReminders) {
            if ($e.id -ne "test-reminder-gui") {
                $payloadArray += @{
                    id = $e.id
                    title = $e.title
                    body = $e.body
                    trigger_at_epoch = $e.trigger_at_epoch
                    action_url = $e.action_url
                }
            }
        }
        $payloadArray += $testReminder
        
        $payloadJson = $payloadArray | ConvertTo-Json -Compress
        $headers = @{ "Content-Type" = "application/json" }
        Invoke-RestMethod -Uri "$env:FCR_REMINDER_API_URL/sync" -Method Post -Body $payloadJson -Headers $headers -TimeoutSec 3

        [System.Windows.Forms.MessageBox]::Show("Test notification scheduled to fire in 10 seconds!", "Test Seeding", [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Information)
        Load-Events
    } catch {
        [System.Windows.Forms.MessageBox]::Show("Failed to trigger test reminder: $($_.Exception.Message)", "Error", [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Error)
    }
}

# Form Resizing logic to adjust card widths dynamically
$form.Add_Resize({
    if ($eventsFlowPanel.Controls.Count -gt 0) {
        $newWidth = $eventsFlowPanel.ClientSize.Width - 46
        foreach ($control in $eventsFlowPanel.Controls) {
            if ($control.Width -ne $newWidth) {
                $control.Width = $newWidth
            }
        }
    }
})

$form.Add_Shown({ Load-Events })

[void]$form.ShowDialog()
"#;

pub fn show_events_dialog(ctx: &GuiContext) -> Result<(), Box<dyn Error>> {
    let mut cmd = std::process::Command::new("powershell");
    cmd.arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-WindowStyle")
        .arg("Hidden")
        .arg("-Command")
        .arg(EVENTS_DIALOG_SCRIPT)
        .env("FCR_REMINDER_VERSION", &ctx.version)
        .env("FCR_REMINDER_DESCRIPTION", &ctx.description)
        .env("FCR_REMINDER_LICENSE", &ctx.license)
        .env("FCR_REMINDER_API_URL", &ctx.api_url)
        .env("FCR_REMINDER_STORAGE", &ctx.storage_path)
        .env("FCR_REMINDER_EXECUTABLE", &ctx.executable_path)
        .env("FCR_REMINDER_ICON_PATH", &ctx.icon_path);

    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW to prevent terminal flashing

    cmd.spawn()
        .map(|_| ())
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}
