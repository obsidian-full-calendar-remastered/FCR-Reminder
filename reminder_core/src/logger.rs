use std::io::Write;

/// Stateless helper function to print logs to the console and append to a persistent log file.
pub fn log_to_file(level: &str, message: &str) {
    let timestamp = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S%.3f UTC")
        .to_string();
    let formatted = format!("[{}] [{}] {}\n", timestamp, level, message);

    // Write to standard output or standard error
    if level == "ERROR" {
        eprint!("{}", formatted);
    } else {
        print!("{}", formatted);
    }

    // Append to log file in local application directory
    if let Some(app_dir) = crate::storage::get_app_dir() {
        let log_path = app_dir.join("fcr-reminder.log");
        let _ = std::fs::create_dir_all(&app_dir);
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let _ = file.write_all(formatted.as_bytes());
        }
    }
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logger::log_to_file("INFO", &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logger::log_to_file("WARN", &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logger::log_to_file("ERROR", &format!($($arg)*));
    };
}
