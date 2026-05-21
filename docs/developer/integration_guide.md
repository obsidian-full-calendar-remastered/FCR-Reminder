# Developer Integration Guide

This guide describes the current integration contract for host applications that want to synchronize reminders into FCR Reminder.

## 1. Supported Integration Surface

Current supported desktop integration:

* local HTTP calls to `http://127.0.0.1:45677`
* flat JSON reminder arrays pushed to `/sync`
* diagnostic reads from `/status`, `/events`, `/next`, `/storage`, and `/doctor`

Mobile deep-link flows are not implemented in this repository today and should not be treated as current source of truth.

## 2. Base URL And Security Model

All supported desktop requests target loopback only:

* base URL: `http://127.0.0.1:45677`
* network exposure: none beyond localhost

The daemon is intentionally local-only.

## 3. Reminder Payload Contract

`POST /sync` expects a flat JSON array of reminder instances.

Example:

```json
[
  {
    "id": "cal-event-992a7e",
    "title": "Project Review Meeting",
    "body": "Synchronize with the development team regarding the Q3 product roadmap.",
    "trigger_at_epoch": 1779308400,
    "action_url": "obsidian://open?vault=PersonalVault&file=Calendar%2FProjectReview"
  }
]
```

Field definitions:

| Field Name | JSON Type | Required | Description |
| :--- | :--- | :---: | :--- |
| `id` | `String` | Yes | Stable reminder identifier. |
| `title` | `String` | Yes | Notification title. |
| `body` | `String` | Yes | Notification body text. |
| `trigger_at_epoch` | `i64` | Yes | Future Unix epoch timestamp in seconds. |
| `action_url` | `String` | Yes | URL invoked when the notification is activated. |

`/sync` is authoritative for the daemon state. A host should send the full current future reminder set, not partial diffs.

## 4. Recommended Host Workflow

### Step 1: Check Daemon Availability

Before sync, query:

* `GET /status`

If the daemon is unreachable, prompt the user to start FCR Reminder rather than blocking the host application.

### Step 2: Build The Flat Reminder Set

The host should:

1. compute recurrence or reminder instances on its own side
2. filter out past items
3. map each instance to the five-field reminder schema
4. debounce sync calls so the daemon receives a single consolidated update burst

### Step 3: Push The Full Set

Send:

* method: `POST`
* endpoint: `/sync`
* content type: `application/json`

### Step 4: Optionally Inspect

Useful read-only routes after a sync:

* `GET /events`
* `GET /next`
* `GET /storage`
* `GET /doctor`

`/doctor` is especially useful during integration work because it identifies the live daemon instance and confirms the resolved storage and Windows registration state.

## 5. Snooze Contract

The daemon supports a snooze flow through:

* `POST /snooze`

Payload:

```json
{
  "id": "event-123",
  "title": "Meeting with John",
  "body": "Discuss the new architecture",
  "action_url": "obsidian://open?vault=MyVault&file=Calendar/event-123",
  "minutes": 5
}
```

The daemon recalculates `trigger_at_epoch`, persists the updated reminder, and wakes the scheduler.

## 6. Operational Guidance For Integrators

Recommended client behavior:

* keep sync timeouts short, around 2 seconds
* debounce event churn before calling `/sync`
* treat daemon unavailability as a recoverable local condition
* avoid assuming file locations; use `/storage` if you need to inspect them
* avoid assuming daemon identity; use `/doctor` if you need to confirm the running instance
