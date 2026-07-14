//! PTY-backed terminal session: process lifecycle, async output pump, resize,
//! input, scrollback, and the live `vt100` emulator state.
//!
//! ## Threading model
//! A dedicated reader thread performs the blocking `read()` on the PTY master
//! and forwards raw byte chunks over an `mpsc` channel. The owning (UI) thread
//! calls [`TerminalSession::pump`] once per frame to drain the channel and feed
//! the bytes into the emulator — so the UI thread never blocks on I/O and the
//! emulator/screen state stays single-threaded (no locking on the render path).

use std::io::Write;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread::JoinHandle;

use portable_pty::{Child, ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};

use crate::config::TerminalConfig;

/// Errors that can occur while managing a terminal session.
#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
    #[error("failed to open pty: {0}")]
    OpenPty(String),
    #[error("failed to spawn shell '{program}': {reason}")]
    Spawn { program: String, reason: String },
    #[error("failed to attach pty reader/writer: {0}")]
    Attach(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Messages sent from the reader thread to the owning thread.
enum ReaderMsg {
    Data(Vec<u8>),
    Eof,
}

/// The outcome of a [`TerminalSession::pump`] call.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PumpOutcome {
    /// New output was applied to the emulator (a repaint is warranted).
    pub dirty: bool,
    /// The child process has exited (observed on this or a previous pump).
    pub exited: bool,
}

/// How a session's child process ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalExit {
    pub code: u32,
    pub signal: Option<String>,
    pub success: bool,
}

/// A running terminal session.
pub struct TerminalSession {
    // Fields ordered for drop-safety: `master` must be dropped before the
    // reader thread exits so the cloned reader handle observes EOF/close
    // instead of blocking `read()` indefinitely (critical on Windows/conpty
    // where killing the child may not produce EOF on the PTY master).
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    reader_rx: Receiver<ReaderMsg>,
    parser: vt100::Parser,
    rows: u16,
    cols: u16,
    scroll_offset: usize,
    program: String,
    alive: bool,
    exit: Option<TerminalExit>,
    // Keep JoinHandle last — Rust drops fields in declaration order, so
    // `master` is dropped (closing the PTY) before this handle detaches
    // the reader thread. The reader observes the close and exits cleanly.
    #[allow(dead_code)]
    reader_handle: Option<JoinHandle<()>>,
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.killer.kill();
        // Do NOT join the reader thread here.  On platforms where kill()
        // does not produce an immediate EOF on the PTY master (Windows
        // conpty, some headless CI runners), the reader remains blocked
        // in `read()` and `join()` would hang the calling thread forever.
        // Rust drops struct fields after Drop::drop() returns — the
        // `master` field is dropped before `reader_handle`, which closes
        // the PTY and causes the cloned reader to observe EOF and exit.
        // Threads are reclaimed by the OS on process exit.
    }
}

impl TerminalSession {
    /// Spawn a shell in a fresh PTY sized to `config.rows` × `config.cols`.
    pub fn spawn(config: &TerminalConfig) -> Result<Self, TerminalError> {
        let rows = config.rows.max(1);
        let cols = config.cols.max(1);
        let (program, args) = config.resolved_shell();

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| TerminalError::OpenPty(e.to_string()))?;

        let mut cmd = CommandBuilder::new(&program);
        for a in &args {
            cmd.arg(a);
        }
        if let Some(cwd) = &config.cwd {
            cmd.cwd(cwd);
        }
        // Advertise a capable terminal so programs enable colors / rich output.
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        for (k, v) in &config.env {
            cmd.env(k, v);
        }

        let child = pair.slave.spawn_command(cmd).map_err(|e| TerminalError::Spawn {
            program: program.clone(),
            reason: e.to_string(),
        })?;
        let killer = child.clone_killer();

        // Drop the slave handle: once the child owns it, closing our copy lets
        // the master observe EOF when the child exits (clean shutdown signal).
        drop(pair.slave);

        let reader =
            pair.master.try_clone_reader().map_err(|e| TerminalError::Attach(e.to_string()))?;
        let writer = pair.master.take_writer().map_err(|e| TerminalError::Attach(e.to_string()))?;

        let (tx, reader_rx) = std::sync::mpsc::channel();
        let reader_handle = std::thread::Builder::new()
            .name("zaroxi-terminal-reader".to_string())
            .spawn(move || reader_loop(reader, tx))
            .map_err(TerminalError::Io)?;

        let parser = vt100::Parser::new(rows, cols, config.scrollback);

        Ok(Self {
            master: pair.master,
            writer,
            child,
            killer,
            reader_rx,
            reader_handle: Some(reader_handle),
            parser,
            rows,
            cols,
            scroll_offset: 0,
            program,
            alive: true,
            exit: None,
        })
    }

    /// The shell program that was launched.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// Drain any pending PTY output into the emulator. Non-blocking.
    pub fn pump(&mut self) -> PumpOutcome {
        let mut dirty = false;
        loop {
            match self.reader_rx.try_recv() {
                Ok(ReaderMsg::Data(bytes)) => {
                    self.parser.process(&bytes);
                    dirty = true;
                }
                Ok(ReaderMsg::Eof) | Err(TryRecvError::Disconnected) => {
                    // Reader closed: the child's PTY is gone. Confirm exit.
                    self.poll_exit();
                    break;
                }
                Err(TryRecvError::Empty) => break,
            }
        }
        if dirty {
            // Keep the visible scrollback offset consistent after new output.
            self.apply_scroll();
        }
        let newly_exited = self.poll_exit();
        PumpOutcome { dirty: dirty || newly_exited, exited: !self.alive }
    }

    /// Write raw bytes to the shell (keyboard input, pasted text, etc.).
    /// Any active scrollback view snaps back to the live prompt.
    pub fn send_input(&mut self, bytes: &[u8]) -> Result<(), TerminalError> {
        if bytes.is_empty() {
            return Ok(());
        }
        // Run one pump so we detect exit before writing.  Writing to a PTY
        // whose slave has already disappeared can block `write_all` / `flush`
        // forever (especially on Windows conpty), wedging the caller.
        self.pump();
        if !self.alive {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "child process has exited",
            )
            .into());
        }
        self.scroll_to_bottom();
        self.writer.write_all(bytes)?;
        self.writer.flush()?;
        Ok(())
    }

    /// Resize the PTY and emulator to `rows` × `cols`. No-op when unchanged.
    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), TerminalError> {
        let rows = rows.max(1);
        let cols = cols.max(1);
        if rows == self.rows && cols == self.cols {
            return Ok(());
        }
        self.master
            .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| TerminalError::Attach(e.to_string()))?;
        self.parser.screen_mut().set_size(rows, cols);
        self.rows = rows;
        self.cols = cols;
        Ok(())
    }

    /// Current grid size in `(rows, cols)`.
    pub fn size(&self) -> (u16, u16) {
        (self.rows, self.cols)
    }

    /// Borrow the live emulator screen for rendering.
    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    /// Whether full-screen ("application") cursor-key mode is active. The
    /// interface layer uses this to encode arrow keys as SS3 vs CSI.
    pub fn application_cursor(&self) -> bool {
        self.parser.screen().application_cursor()
    }

    /// Whether the program requested bracketed-paste mode.
    pub fn bracketed_paste(&self) -> bool {
        self.parser.screen().bracketed_paste()
    }

    /// The current scrollback offset (0 = live bottom).
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Scroll the view up into history by `lines` (toward older output).
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(lines);
        self.apply_scroll();
    }

    /// Scroll the view down toward the live prompt by `lines`.
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        self.apply_scroll();
    }

    /// Snap the view back to the live prompt (bottom of the buffer).
    pub fn scroll_to_bottom(&mut self) {
        if self.scroll_offset != 0 {
            self.scroll_offset = 0;
            self.apply_scroll();
        }
    }

    fn apply_scroll(&mut self) {
        self.parser.screen_mut().set_scrollback(self.scroll_offset);
        // vt100 clamps internally; mirror the clamp so our reported offset never
        // drifts past the real amount of retained history.
        self.scroll_offset = self.parser.screen().scrollback();
    }

    /// Whether the child process is still running.
    pub fn is_alive(&self) -> bool {
        self.alive
    }

    /// How the child exited, if it has.
    pub fn exit_status(&self) -> Option<&TerminalExit> {
        self.exit.as_ref()
    }

    /// Poll the child for termination. Returns `true` on the transition from
    /// alive → exited (so callers can react exactly once).
    fn poll_exit(&mut self) -> bool {
        if !self.alive {
            return false;
        }
        match self.child.try_wait() {
            Ok(Some(status)) => {
                self.alive = false;
                self.exit = Some(TerminalExit {
                    code: status.exit_code(),
                    signal: status.signal().map(|s| s.to_string()),
                    success: status.success(),
                });
                true
            }
            Ok(None) => false,
            Err(_) => {
                // Treat an errored wait as termination to avoid a wedged state.
                self.alive = false;
                self.exit = Some(TerminalExit { code: 1, signal: None, success: false });
                true
            }
        }
    }

    /// Terminate the child process (best-effort). Idempotent.
    pub fn shutdown(&mut self) {
        let _ = self.killer.kill();
        self.alive = false;
    }
}

/// The reader thread body: blocking reads forwarded as channel messages.
fn reader_loop(mut reader: Box<dyn std::io::Read + Send>, tx: std::sync::mpsc::Sender<ReaderMsg>) {
    let mut buf = [0u8; 8192];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => {
                let _ = tx.send(ReaderMsg::Eof);
                break;
            }
            Ok(n) => {
                if tx.send(ReaderMsg::Data(buf[..n].to_vec())).is_err() {
                    // Receiver dropped: session is gone.
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => {
                let _ = tx.send(ReaderMsg::Eof);
                break;
            }
        }
    }
}
