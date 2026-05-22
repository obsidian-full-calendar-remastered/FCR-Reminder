#[cfg(target_os = "windows")]
#[path = "src/platform/windows/build_support.rs"]
mod windows_build_support;

fn main() {
    #[cfg(target_os = "windows")]
    {
        if let Err(error) = windows_build_support::embed_windows_resources() {
            panic!("failed to embed Windows resources: {}", error);
        }
    }
}
