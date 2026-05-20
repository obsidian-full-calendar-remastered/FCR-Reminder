use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reminder {
    pub id: String,
    pub title: String,
    pub body: String,
    pub trigger_at_epoch: i64,
    pub action_url: String,
}
