mod console;
mod notification;
mod registry;

pub use console::prepare_console_for_cli;
pub use notification::trigger_notification;
pub use registry::{cleanup, doctor_checks, init};

/// Dedicated Windows background thread setup to run the Win32 message loop.
pub fn run_event_loop() {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, TranslateMessage, MSG,
        };
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}