use crate::core::models::Reminder;
use crate::core::storage::{load_reminders, save_reminders};
use crate::{log_error, log_info};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

pub struct FiredNotification {
    pub id: String,
    pub fired_at: i64,
}

/// Core background scheduler loop.
/// Sleeps until the next active reminder, woke up instantly on sync updates.
pub async fn run_scheduler(
    rx: &mut watch::Receiver<()>,
    fired_notifications: Arc<Mutex<Vec<FiredNotification>>>,
) {
    log_info!("Background scheduler started.");

    // Load active reminders from disk exactly once on startup to maintain an in-memory cache
    let reminders_loaded = load_reminders().unwrap_or_default();
    let current_time = chrono::Utc::now().timestamp();

    // Partition into missed and future reminders
    let (missed_reminders, future_reminders): (Vec<Reminder>, Vec<Reminder>) = reminders_loaded
        .into_iter()
        .partition(|r| r.trigger_at_epoch <= current_time);

    let mut reminders = future_reminders;

    if !missed_reminders.is_empty() {
        log_info!(
            "AUDIT: Found {} missed reminders on startup. Initiating recovery...",
            missed_reminders.len()
        );

        // Immediately update reminders.json so missed reminders are no longer stored on disk
        if let Err(e) = save_reminders(&reminders) {
            log_error!(
                "Failed to save reminders list after clearing missed ones: {}",
                e
            );
        }

        // Spawn a background worker to fire missed notifications with a 20-second interval
        let fired_clone = Arc::clone(&fired_notifications);
        tokio::spawn(async move {
            for (i, reminder) in missed_reminders.into_iter().enumerate() {
                if i > 0 {
                    tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                }

                // Record as fired to avoid re-triggering if a sync occurs in the meantime
                let now = chrono::Utc::now().timestamp();
                {
                    let mut fired = fired_clone.lock().unwrap();
                    fired.retain(|f| now - f.fired_at < 600);
                    fired.push(FiredNotification {
                        id: reminder.id.clone(),
                        fired_at: now,
                    });
                }

                log_info!(
                    "AUDIT: Firing recovered missed reminder '{}' (ID: {}, originally scheduled for epoch {})",
                    reminder.title,
                    reminder.id,
                    reminder.trigger_at_epoch
                );
                trigger_notification(&reminder);
            }
        });
    }

    loop {
        let current_time = chrono::Utc::now().timestamp();

        // Filter for active future reminders and sort ascending using in-memory list
        let mut active: Vec<Reminder> = reminders
            .iter()
            .filter(|r| r.trigger_at_epoch > current_time)
            .cloned()
            .collect();

        active.sort_by_key(|r| r.trigger_at_epoch);

        if active.is_empty() {
            log_info!("No active future reminders. Sleeping until next synchronization.");
            // Sleep indefinitely until we receive a wakeup signal
            if rx.changed().await.is_err() {
                break; // Watch channel closed, terminate scheduler
            }
            // Reload reminders from disk when woke up by a synchronization event
            reminders = load_reminders().unwrap_or_default();
            continue;
        }

        let next_reminder = active[0].clone();
        let delay_secs = next_reminder.trigger_at_epoch - current_time;

        log_info!(
            "Next reminder scheduled: \"{}\" in {} seconds (at Epoch {}).",
            next_reminder.title,
            delay_secs,
            next_reminder.trigger_at_epoch
        );

        let delay = std::time::Duration::from_secs(delay_secs.max(0) as u64);

        // Sleep until either the timer expires OR a sync signal occurs
        tokio::select! {
            _ = tokio::time::sleep(delay) => {
                log_info!("Reminder triggered! Firing notification for \"{}\".", next_reminder.title);
                trigger_notification(&next_reminder);

                // Record the fired notification
                {
                    let mut fired = fired_notifications.lock().unwrap();
                    let now = chrono::Utc::now().timestamp();
                    // Prune old entries
                    fired.retain(|f| now - f.fired_at < 600);
                    fired.push(FiredNotification {
                        id: next_reminder.id.clone(),
                        fired_at: now,
                    });
                    log_info!(
                        "AUDIT: Recorded fired notification '{}' (ID: {}) at epoch {}.",
                        next_reminder.title,
                        next_reminder.id,
                        now
                    );
                }

                // Update the in-memory cache directly without reading from disk
                reminders.retain(|r| r.id != next_reminder.id);

                // Write remaining reminders to disk for durability (single write, no disk reads)
                if let Err(e) = save_reminders(&reminders) {
                    log_error!("Failed to save reminders list after firing: {}", e);
                }
            }
            res = rx.changed() => {
                if res.is_err() {
                    break; // Watch channel closed, terminate scheduler
                }
                log_info!("Synchronization signal received. Reloading reminders from disk and rescheduling...");
                // Reload reminders from disk since the local database was modified by HTTP routes (/sync or /snooze)
                reminders = load_reminders().unwrap_or_default();
            }
        }
    }
}

/// Dispatches a native operating system notification.
pub fn trigger_notification(reminder: &Reminder) {
    if let Err(e) = crate::platform::trigger_notification(reminder) {
        log_error!("Notification error: {}", e);
    }
}
