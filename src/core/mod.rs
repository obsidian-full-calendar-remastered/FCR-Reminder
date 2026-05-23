pub mod api;
pub mod cli;
pub mod commands;
pub mod daemon;
pub mod gui;
pub mod logger;
pub mod models;
pub mod release_updates;
pub mod scheduler;
pub mod storage;

pub use cli::run_main as run_cli;
pub use daemon::run_daemon;
pub use gui::open_event_viewer;
pub use models::Reminder;
pub use storage::{get_app_dir, get_storage_path, load_reminders, save_reminders};
