# Developer Integration Guide

!!! abstract "Integration Contract"
    This page describes the current host-to-daemon contract for applications that synchronize reminder instances into FCR Reminder.

!!! info "Current Scope"
    Supported integration in this repository is desktop-local and loopback-only. Mobile deep-link workflows are not the current source of truth here.

## Action Matrix

| If you need to... | Start here | Typical follow-up |
|---|---|---|
| confirm the daemon is available | [Base URL and Security Model](#2-base-url-and-security-model) | [Recommended Host Workflow](#4-recommended-host-workflow) |
| build a valid `/sync` payload | [Reminder Payload Contract](#3-reminder-payload-contract) | [Recommended Host Workflow](#4-recommended-host-workflow) |
| wire snooze behavior back into the daemon | [Snooze Contract](#5-snooze-contract) | [Control API and Lifecycle](../architecture/control_api.md) |
| diagnose a live integration issue | [Recommended Host Workflow](#4-recommended-host-workflow) | [Operational Guidance for Integrators](#6-operational-guidance-for-integrators) |

## 1. Supported Integration Surface

Current supported desktop integration:

- local HTTP calls to `http://127.0.0.1:45677`
- flat JSON reminder arrays pushed to `/sync`
- diagnostic reads from `/status`, `/events`, `/next`, `/storage`, `/doctor`, and `/updates`

## 2. Base URL and Security Model

All supported desktop requests target loopback only:

- base URL: `http://127.0.0.1:45677`
- network exposure: none beyond localhost

!!! warning "Do Not Treat This As a Network Service"
    The daemon is intentionally local-only. It is not designed as a remote API surface, and integrators should not build assumptions around LAN or WAN reachability.

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

| Field Name | JSON Type | Required | Description |
|---|---|:---:|---|
| `id` | `String` | Yes | Stable reminder identifier. |
| `title` | `String` | Yes | Notification title. |
| `body` | `String` | Yes | Notification body text. |
| `trigger_at_epoch` | `i64` | Yes | Future Unix epoch timestamp in seconds. |
| `action_url` | `String` | Yes | URL invoked when the notification is activated. |

!!! note "Authoritative Sync Model"
    `/sync` is authoritative for daemon state. A host should send the full current future reminder set, not partial diffs.

## 4. Recommended Host Workflow

### Step 1: Check Daemon Availability

Before sync, query `GET /status`.

If the daemon is unreachable, prompt the user to start FCR Reminder rather than blocking the host application indefinitely.

### Step 2: Build the Flat Reminder Set

The host should:

1. compute recurrence or reminder instances on its own side
2. filter out past items
3. map each instance to the five-field reminder schema
4. debounce sync calls so the daemon receives a single consolidated update burst

### Step 3: Push the Full Set

Send:

- method: `POST`
- endpoint: `/sync`
- content type: `application/json`

### Step 4: Optionally Inspect

Useful read-only routes after a sync:

- `GET /events`
- `GET /next`
- `GET /storage`
- `GET /doctor`
- `GET /updates`

!!! example "Best Integration Check"
    `/doctor` is especially useful during integration work because it identifies the live daemon instance and confirms storage and Windows registration state in one response.

!!! note "Update State Is Separate From Sync State"
  `/updates` reports the daemon's cached GitHub release-awareness state. It does not affect reminder scheduling, and hosts should treat it as optional read-only diagnostics rather than part of the reminder synchronization contract.

## 5. Snooze Contract

The daemon supports a snooze flow through `POST /snooze`.

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

## 6. Operational Guidance for Integrators

Recommended client behavior:

- keep sync timeouts short, around 2 seconds
- debounce event churn before calling `/sync`
- treat daemon unavailability as a recoverable local condition
- avoid assuming file locations; use `/storage` if you need to inspect them
- avoid assuming daemon identity; use `/doctor` if you need to confirm the running instance
- avoid coupling reminder sync logic to `/updates`; it is a diagnostic/read-only surface for release awareness

Compact index: [User Docs](../user/index.md) · [Architecture Docs](../architecture/index.md) · [Control API and Lifecycle](../architecture/control_api.md) · [Blueprint](../architecture/blueprint.md)
