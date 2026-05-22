param(
    [string]$DesktopExe = ".\target\debug\fcr-reminder.exe",
    [string]$CliExe = ".\target\debug\fcr-reminder-cli.exe",
    [switch]$StartDaemon,
    [switch]$SeedReminder,
    [int]$DelaySeconds = 15,
    [switch]$KeepRunning
)

$ErrorActionPreference = "Stop"

function Test-DaemonAvailable {
    try {
        Invoke-RestMethod -Uri "http://127.0.0.1:45677/status" -Method Get | Out-Null
        return $true
    }
    catch {
        return $false
    }
}

function Invoke-DaemonCli {
    param([string[]]$Arguments)

    & $ResolvedCliExe @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Daemon CLI command failed: $($Arguments -join ' ')"
    }
}

function Write-Section {
    param([string]$Title)

    Write-Host "`n=== $Title ===" -ForegroundColor Cyan
}

function Resolve-BinaryPath {
    param(
        [string]$PreferredPath,
        [string[]]$FallbackPaths
    )

    $Candidates = @($PreferredPath) + $FallbackPaths
    foreach ($Candidate in $Candidates) {
        if ($Candidate -and (Test-Path -LiteralPath $Candidate)) {
            return (Resolve-Path -LiteralPath $Candidate).Path
        }
    }

    throw "Could not find a binary at any of these paths: $($Candidates -join ', ')"
}

$ResolvedDesktopExe = Resolve-BinaryPath -PreferredPath $DesktopExe -FallbackPaths @(
    ".\target\release\fcr-reminder.exe"
)
$ResolvedCliExe = Resolve-BinaryPath -PreferredPath $CliExe -FallbackPaths @(
    ".\target\release\fcr-reminder-cli.exe"
)
$StartedProcess = $null

if (-not (Test-DaemonAvailable)) {
    if (-not $StartDaemon) {
        throw "The daemon is not running. Start it manually or rerun this script with -StartDaemon."
    }

    Write-Section "Starting Daemon"
    $StartedProcess = Start-Process -FilePath $ResolvedDesktopExe -ArgumentList "--debug" -PassThru
    Write-Host "Started daemon process $($StartedProcess.Id) using $ResolvedDesktopExe"

    for ($Attempt = 1; $Attempt -le 20; $Attempt++) {
        Start-Sleep -Milliseconds 500
        if (Test-DaemonAvailable) {
            break
        }
    }

    if (-not (Test-DaemonAvailable)) {
        throw "The daemon did not become reachable on http://127.0.0.1:45677."
    }
}

Write-Section "Daemon Health"
Invoke-DaemonCli -Arguments @("--health")

Write-Section "Doctor Report"
Invoke-DaemonCli -Arguments @("--doctor")

Write-Section "Update Status"
Invoke-DaemonCli -Arguments @("--updates")

Write-Section "Storage Details"
Invoke-DaemonCli -Arguments @("--storage")

Write-Section "Stored Events"
Invoke-DaemonCli -Arguments @("--events")

Write-Section "Next Event"
Invoke-DaemonCli -Arguments @("--next")

if ($SeedReminder) {
    $TriggerTime = [DateTimeOffset]::UtcNow.AddSeconds($DelaySeconds).ToUnixTimeSeconds()
    $Payload = @"
[
    {
        "id": "test-event-999",
        "title": "Hello from Obsidian!",
        "body": "This is a native Windows toast notification triggered from the daemon.",
        "trigger_at_epoch": $TriggerTime,
        "action_url": "obsidian://open"
    }
]
"@

    Write-Section "Seeding Reminder"
    Write-Host "Sending payload. Notification will trigger in $DelaySeconds seconds (epoch: $TriggerTime)."
    Invoke-RestMethod -Uri "http://127.0.0.1:45677/sync" -Method Post -Body $Payload -ContentType "application/json" | Out-Null

    Write-Section "Next Event After Sync"
    Invoke-DaemonCli -Arguments @("--next")

    Write-Section "Stored Events After Sync"
    Invoke-DaemonCli -Arguments @("--events")
}

if ($StartedProcess -and -not $KeepRunning) {
    Write-Section "Stopping Started Daemon"
    Stop-Process -Id $StartedProcess.Id
    Write-Host "Stopped daemon process $($StartedProcess.Id)"
}
