#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]
#![allow(dead_code)]
#![allow(unused_imports)]

mod core;
mod platform;

fn main() {
    core::run_daemon();
}
