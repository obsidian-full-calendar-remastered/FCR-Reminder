# Calculate target epoch 15 seconds from now
$triggerTime = [DateTimeOffset]::UtcNow.AddSeconds(15).ToUnixTimeSeconds()

Write-Host "Sending payload... Notification will trigger in 15 seconds (at Epoch: $triggerTime)"

# Define the JSON sync payload as a standard here-string to prevent PowerShell array flattening bugs
$payload = @"
[
    {
        "id": "test-event-999",
        "title": "Hello from Obsidian!",
        "body": "This is a native Windows toast notification triggered from the daemon.",
        "trigger_at_epoch": $triggerTime,
        "action_url": "obsidian://open"
    }
]
"@

# POST payload to our sync endpoint
$response = Invoke-RestMethod -Uri "http://127.0.0.1:45677/sync" -Method Post -Body $payload -ContentType "application/json"
Write-Host "Response received: $response"
