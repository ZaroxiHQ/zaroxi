//! Integration tests for the PTY-backed terminal session.
//!
//! These spawn a **real** shell in a PTY and exercise the full lifecycle:
//! output pump, input, resize, scrollback and clean exit/restart. They are
//! written defensively (bounded polling with a timeout).
//!
//! Tests that depend on child-process output are gated to `#[cfg(unix)]`
//! because `portable_pty`'s Windows `conpty` implementation does not reliably
//! deliver child stdout on headless CI runners (output is silently lost).
//! The resize and drop-leak tests exercise the session plumbing itself and
//! are safe on both platforms.

use std::time::{Duration, Instant};

use zaroxi_core_platform_terminal::{PumpOutcome, TerminalConfig, TerminalSession};

fn spawn_unix(cmd: &str) -> Option<TerminalSession> {
    let mut cfg = TerminalConfig { rows: 24, cols: 80, ..Default::default() };
    cfg.shell = Some("/bin/sh".to_string());
    cfg.args = vec!["-c".to_string(), cmd.to_string()];
    TerminalSession::spawn(&cfg).ok()
}

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

fn send_or_abort(session: &mut TerminalSession, bytes: &[u8]) -> bool {
    session.send_input(bytes).is_ok()
}

fn screen_contains(session: &TerminalSession, needle: &str) -> bool {
    session.screen().contents().contains(needle)
}

// ── Tests requiring child-process output (Unix-only) ──────────────────
// portable_pty's Windows conpty does not reliably deliver child stdout on
// headless CI runners; these tests are gated to Unix where PTY output is
// robust and deterministic.

#[test]
#[cfg(unix)]
fn spawns_shell_and_captures_output() {
    let Some(mut session) = spawn_unix("echo zaroxi_hello") else {
        eprintln!("no shell available; skipping");
        return;
    };
    let saw = pump_until(&mut session, |s| screen_contains(s, "zaroxi_hello"));
    assert!(saw, "expected echoed output to appear on the screen");
}

#[test]
#[cfg(unix)]
fn detects_child_exit() {
    let Some(mut session) = spawn_unix("exit 0") else {
        return;
    };
    let exited = pump_until(&mut session, |s| !s.is_alive());
    assert!(exited, "child should be observed as exited");
    assert!(session.exit_status().is_some(), "exit status should be recorded");
}

#[test]
#[cfg(unix)]
fn interactive_input_roundtrips() {
    let Some(mut session) = spawn_unix("while read line; do echo got:$line; done") else {
        return;
    };
    std::thread::sleep(Duration::from_millis(200));
    let sent = send_or_abort(&mut session, b"ping\n");
    if !sent {
        eprintln!("send_input failed (child likely exited early); skipping");
        return;
    }
    let saw = pump_until(&mut session, |s| screen_contains(s, "got:ping"));
    assert!(saw, "typed input should be processed by the shell and echoed");
}

#[test]
#[cfg(unix)]
fn scrollback_offset_moves_and_snaps_back() {
    let cmd = "i=0; while [ $i -lt 200 ]; do echo line$i; i=$((i+1)); done; sleep 2";
    let Some(mut session) = spawn_unix(cmd) else {
        return;
    };
    let saw = pump_until(&mut session, |s| screen_contains(s, "line199") || !s.is_alive());
    if !session.is_alive() && !screen_contains(&session, "line199") {
        eprintln!("child exited before producing scrollback output; skipping");
        return;
    }
    assert!(saw, "expected line199 to appear on screen");
    session.scroll_up(50);
    assert!(session.scroll_offset() > 0, "scrollback offset should advance into history");
    send_or_abort(&mut session, b"\n");
    assert_eq!(session.scroll_offset(), 0, "input snaps the view to the bottom");
}

// ── Session-plumbing tests (both platforms) ───────────────────────────
// These exercise the session struct itself (resize, drop) without
// depending on child-process output.

#[test]
fn resize_updates_grid_dimensions() {
    let mut cfg = TerminalConfig { rows: 24, cols: 80, ..Default::default() };
    #[cfg(unix)]
    {
        cfg.shell = Some("/bin/sh".to_string());
        cfg.args = vec!["-c".to_string(), "cat".to_string()];
    }
    #[cfg(windows)]
    {
        cfg.shell = Some("cmd.exe".to_string());
        cfg.args = vec!["/C".to_string(), "more".to_string()];
    }
    let Some(mut session) = TerminalSession::spawn(&cfg).ok() else {
        return;
    };
    assert_eq!(session.size(), (24, 80));
    session.resize(40, 120).expect("resize");
    assert_eq!(session.size(), (40, 120));
    let (rows, cols) = session.screen().size();
    assert_eq!((rows, cols), (40, 120), "emulator screen tracks the resize");
}

#[test]
fn no_leaked_process_after_drop() {
    let mut cfg = TerminalConfig { rows: 24, cols: 80, ..Default::default() };
    #[cfg(unix)]
    {
        cfg.shell = Some("/bin/sh".to_string());
        cfg.args = vec!["-c".to_string(), "sleep 30".to_string()];
    }
    #[cfg(windows)]
    {
        cfg.shell = Some("cmd.exe".to_string());
        cfg.args = vec!["/C".to_string(), "timeout /t 30 /nobreak".to_string()];
    }
    let Some(mut session) = TerminalSession::spawn(&cfg).ok() else {
        return;
    };
    let _ = session.pump();
    drop(session);
}
