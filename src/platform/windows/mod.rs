mod console;
mod events_gui;
mod notification;
mod registry;

pub use console::prepare_console_for_cli;
pub use events_gui::show_events_dialog;
pub use notification::{trigger_notification, trigger_update_notification};
pub use registry::{cleanup, doctor_checks, init, open_url, show_about_dialog};

/// Pumps the Windows message loop once, blocking up to the provided timeout.
pub fn run_event_loop_once(timeout: std::time::Duration) {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, MsgWaitForMultipleObjectsEx, PeekMessageW, TranslateMessage, MSG,
            MWMO_INPUTAVAILABLE, PM_REMOVE, QS_ALLINPUT,
        };

        let timeout_ms = timeout.as_millis().min(u32::MAX as u128) as u32;
        let _ = MsgWaitForMultipleObjectsEx(
            0,
            std::ptr::null(),
            timeout_ms,
            QS_ALLINPUT,
            MWMO_INPUTAVAILABLE,
        );

        let mut msg: MSG = std::mem::zeroed();
        while PeekMessageW(&mut msg, 0, 0, 0, PM_REMOVE) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
