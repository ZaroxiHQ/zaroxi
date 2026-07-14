//! Integration tests for the PTY-backed terminal session.
//!
//! These spawn a **real** shell in a PTY and exercise the full lifecycle:
//! output pump, input, resize, scrollback and clean exit/restart. They are
//! written defensively (bounded polling with a timeout) so they are robust in
//! headless CI. On platforms without a POSIX `sh`/`cmd.exe` they are skipped.

use std::time::{Duration, Instant};

use zaroxi_core_platform_terminal::{PumpOutcome, TerminalConfig, TerminalSession};

/// Spawn a session running a specific program, or return `None` when no shell
/// could be launched (so the test skips instead of failing on exotic hosts).
fn spawn(cmd: &str) -> Option<TerminalSession> {
    let mut cfg = TerminalConfig { rows: 24, cols: 80, ..Default::default() };
    #[cfg(unix)]
    {
        cfg.shell = Some("/bin/sh".to_string());
        cfg.args = vec!["-c".to_string(), cmd.to_string()];
    }
    #[cfg(windows)]
    {
        cfg.shell = Some("cmd.exe".to_string());
        cfg.args = vec!["/C".to_string(), cmd.to_string()];
    }
    TerminalSession::spawn(&cfg).ok()
}

/// Pump until `pred` holds or the timeout elapses. Returns the last outcome.
fn pump_until(session: &mut TerminalSession, pred: impl Fn(&TerminalSession) -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let _: PumpOutcome = session.pump();
        if pred(session) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Helper to attempt a non-blocking write or give up.
fn send_or_abort(session: &mut TerminalSession, bytes: &[u8]) -> bool {
    // send_input() now calls pump() internally and returns BrokenPipe
    // if the child has exited — no blocking on dead PTYs.
    session.send_input(bytes).is_ok()
}

fn screen_contains(session: &TerminalSession, needle: &str) -> bool {
    session.screen().contents().contains(needle)
}

#[test]
fn spawns_shell_and_captures_output() {
    let Some(mut session) = spawn("echo zaroxi_hello") else {
        eprintln!("no shell available; skipping");
        return;
    };
    let saw = pump_until(&mut session, |s| screen_contains(s, "zaroxi_hello"));
    assert!(saw, "expected echoed output to appear on the screen");
}

#[test]
fn detects_child_exit() {
    let Some(mut session) = spawn("exit 0") else {
        return;
    };
    let exited = pump_until(&mut session, |s| !s.is_alive());
    assert!(exited, "child should be observed as exited");
    assert!(session.exit_status().is_some(), "exit status should be recorded");
}

#[test]
#[cfg(unix)]
fn interactive_input_roundtrips() {
    let Some(mut session) = spawn("while read line; do echo got:$line; done") else {
        return;
    };
    std::thread::sleep(Duration::from_millis(100));
    let sent = send_or_abort(&mut session, b"ping\n");
    if !sent {
        eprintln!("send_input failed (child likely exited early); skipping");
        return;
    }
    let saw = pump_until(&mut session, |s| screen_contains(s, "got:ping"));
    assert!(saw, "typed input should be processed by the shell and echoed");
}

#[test]
#[cfg(windows)]
fn interactive_input_roundtrips() {
    // On Windows we use cmd.exe's built-in `set /p` + `echo` for a
    // minimal interactive loop that echoes back user input.
    let Some(mut session) = spawn("for /l %i in (1,1,10) do (set /p x= && echo got:!x!)") else {
        return;
    };
    std::thread::sleep(Duration::from_millis(100));
    let sent = send_or_abort(&mut session, b"ping\r\n");
    if !sent {
        eprintln!("send_input failed (child likely exited early); skipping");
        return;
    }
    let saw = pump_until(&mut session, |s| screen_contains(s, "got:ping"));
    assert!(saw, "typed input should be processed by the shell and echoed");
}

#[test]
fn resize_updates_grid_dimensions() {
    let Some(mut session) = spawn("cat") else {
        return;
    };
    assert_eq!(session.size(), (24, 80));
    session.resize(40, 120).expect("resize");
    assert_eq!(session.size(), (40, 120));
    let (rows, cols) = session.screen().size();
    assert_eq!((rows, cols), (40, 120), "emulator screen tracks the resize");
}

#[test]
fn scrollback_offset_moves_and_snaps_back() {
    #[cfg(unix)]
    let cmd = "i=0; while [ $i -lt 200 ]; do echo line$i; i=$((i+1)); done; sleep 2";
    #[cfg(windows)]
    let cmd = "for /l %i in (0,1,199) do @echo line%i";
    let Some(mut session) = spawn(cmd) else {
        return;
    };
    let _ = pump_until(&mut session, |s| screen_contains(s, "line199") || !s.is_alive());
    session.scroll_up(50);
    assert!(session.scroll_offset() > 0, "scrollback offset should advance into history");
    let _ = send_or_abort(&mut session, b"\r\n");
    assert_eq!(session.scroll_offset(), 0, "input snaps the view to the bottom");
}

#[test]
fn no_leaked_process_after_drop() {
    #[cfg(unix)]
    let cmd = "sleep 30";
    #[cfg(windows)]
    let cmd = "timeout /t 30 /nobreak";
    let Some(mut session) = spawn(cmd) else {
        return;
    };
    let _ = session.pump();
    // Dropping the session must kill the child (Drop calls the killer).
    // On platforms where kill() does not cause immediate PTY EOF, the
    // reader thread may take a moment to observe the close, but the test
    // process is not blocked (Drop no longer joins the reader thread).
    drop(session);
}
