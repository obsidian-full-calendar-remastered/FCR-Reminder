/// Reattaches the GUI-subsystem process to the parent console when launched from a terminal.
pub fn prepare_console_for_cli() {
    use windows_sys::Win32::System::Console::{
        AttachConsole, AllocConsole, ATTACH_PARENT_PROCESS,
    };

    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            let _ = AllocConsole();
        }
    }
}