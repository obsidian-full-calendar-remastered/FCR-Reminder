#![allow(dead_code)]
#![allow(unused_imports)]

mod core;
mod platform;

fn main() -> std::process::ExitCode {
    core::run_cli()
}
