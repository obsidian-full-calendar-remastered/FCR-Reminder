use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const DAEMON_ADDR: &str = "127.0.0.1:45677";
const TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(250);

#[test]
fn daemon_start_and_stop_smoke_test() {
    stop_daemon_best_effort();
    wait_for_daemon_state(false, Duration::from_secs(5));

    let mut daemon = start_daemon_process();

    wait_for_daemon_state(true, TIMEOUT);

    let updates_status = Command::new(cli_exe())
        .arg("--updates")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to run fcr-reminder-cli --updates");
    assert!(
        updates_status.success(),
        "updates command failed with status {:?}",
        updates_status.code()
    );

    let stop_status = Command::new(cli_exe())
        .arg("--stop")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to run fcr-reminder-cli --stop");
    assert!(
        stop_status.success(),
        "stop command failed with status {:?}",
        stop_status.code()
    );

    wait_for_daemon_state(false, TIMEOUT);
    wait_for_child_exit(&mut daemon, TIMEOUT);
}

fn cli_exe() -> &'static str {
    env!("CARGO_BIN_EXE_fcr-reminder-cli")
}

fn gui_exe() -> &'static str {
    env!("CARGO_BIN_EXE_fcr-reminder")
}

fn start_daemon_process() -> Child {
    Command::new(gui_exe())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to launch fcr-reminder daemon")
}

fn daemon_reachable() -> bool {
    TcpStream::connect(DAEMON_ADDR).is_ok()
}

fn stop_daemon_best_effort() {
    let _ = Command::new(cli_exe())
        .arg("--stop")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn wait_for_daemon_state(expected_running: bool, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        if daemon_reachable() == expected_running {
            return;
        }

        assert!(
            Instant::now() < deadline,
            "daemon state did not become {} within {:?}",
            if expected_running {
                "running"
            } else {
                "stopped"
            },
            timeout
        );

        thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_child_exit(child: &mut Child, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        match child
            .try_wait()
            .expect("failed to query daemon child status")
        {
            Some(status) => {
                assert!(
                    status.success(),
                    "daemon exited unsuccessfully: {:?}",
                    status.code()
                );
                return;
            }
            None => {
                assert!(
                    Instant::now() < deadline,
                    "daemon child did not exit within {:?}",
                    timeout
                );
                thread::sleep(POLL_INTERVAL);
            }
        }
    }
}
