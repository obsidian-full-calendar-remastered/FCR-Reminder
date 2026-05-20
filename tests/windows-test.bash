#!/bin/bash

# Calculate target epoch 15 seconds from now (compatible with Git Bash and Linux date command)
TRIGGER_TIME=$(($(date +%s) + 15))

echo "Sending payload... Notification will trigger in 15 seconds (at Epoch: $TRIGGER_TIME)"

# Define the JSON sync payload
PAYLOAD="[
  {
    \"id\": \"test-event-999\",
    \"title\": \"Hello from Obsidian!\",
    \"body\": \"This is a native Windows toast notification triggered from the daemon.\",
    \"trigger_at_epoch\": ${TRIGGER_TIME},
    \"action_url\": \"obsidian://open\"
  }
]"

# POST payload to our sync endpoint using curl
curl -i -X POST -H "Content-Type: application/json" -d "$PAYLOAD" http://127.0.0.1:45677/sync
echo -e "\nDone!"