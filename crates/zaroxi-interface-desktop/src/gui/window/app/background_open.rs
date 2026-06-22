//! Background file-open worker: off-thread buffer (rope) materialization.
//!
//! Building the rope for a large file (`Rope::from_lines`) costs ~100 ms for a
//! 300k-line file. Doing that on the UI thread monopolizes input/render for the
//! whole duration. This worker moves that work onto a dedicated thread, mirrored
//! on the existing [`super::background_parse::BackgroundParseWorker`] design:
//!
//! 1. `spawn()` starts the worker thread.
//! 2. `schedule_open(job)` sends a tokened job (stale tokens are dropped).
//! 3. `take_result()` collects the completed rope (stale tokens discarded).
//!
//! Token awareness: every job carries the open token assigned by
//! `GuiApp::request_open`. The worker drains its queue to the newest pending job
//! before building (so superseded *queued* opens are skipped entirely), and the
//! UI side drops any result whose token is no longer the winning one — so a
//! stale open can never flash old content into the editor.

use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use zaroxi_core_editor_rope::Rope;

/// A tokened request to materialize a buffer off-thread.
pub struct OpenJob {
    pub token: u64,
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
}

/// The materialized result for a winning (or to-be-validated) open token.
pub struct OpenResult {
    pub token: u64,
    pub rope: Rope,
    pub cursor_line: usize,
    pub cursor_col: usize,
    /// Wall time spent building the rope on the worker thread.
    pub build_us: u64,
    /// Number of materialization chunks (currently one fused pass).
    pub chunks: u32,
}

/// Background worker owning expensive rope materialization for the winning open
/// token. The UI thread only schedules jobs and installs completed results.
pub struct BackgroundOpenWorker {
    tx: mpsc::Sender<OpenJob>,
    rx: mpsc::Receiver<OpenResult>,
    _handle: Option<thread::JoinHandle<()>>,
    last_sent_token: u64,
    completed_result: Option<OpenResult>,
}

impl BackgroundOpenWorker {
    /// Spawn the background open worker thread.
    pub fn spawn() -> Self {
        let (job_tx, job_rx) = mpsc::channel::<OpenJob>();
        let (result_tx, result_rx) = mpsc::channel::<OpenResult>();

        let handle = thread::Builder::new()
            .name("zaroxi-open-worker".into())
            .spawn(move || {
                Self::worker_loop(job_rx, result_tx);
            })
            .ok();

        Self {
            tx: job_tx,
            rx: result_rx,
            _handle: handle,
            last_sent_token: 0,
            completed_result: None,
        }
    }

    /// Send a new open job. Drops jobs whose token is not newer than the last
    /// one already scheduled (no stale work queued).
    pub fn schedule_open(&mut self, job: OpenJob) {
        if job.token <= self.last_sent_token {
            return;
        }
        self.last_sent_token = job.token;
        let _ = self.tx.send(job);
    }

    /// The newest token scheduled to the worker.
    pub fn latest_token(&self) -> u64 {
        self.last_sent_token
    }

    /// Drain completed results, keeping only the newest (>= last scheduled
    /// token) and discarding stale ones. Returns and takes ownership of the
    /// pending completed result, if any.
    pub fn take_result(&mut self) -> Option<OpenResult> {
        while let Ok(result) = self.rx.try_recv() {
            if result.token >= self.last_sent_token {
                self.completed_result = Some(result);
            }
            // else: a superseded job's result — discard.
        }
        self.completed_result.take()
    }

    fn worker_loop(job_rx: mpsc::Receiver<OpenJob>, result_tx: mpsc::Sender<OpenResult>) {
        while let Ok(mut job) = job_rx.recv() {
            // Coalesce: if newer jobs are already queued, skip straight to the
            // latest so a superseded large file is never materialized.
            while let Ok(newer) = job_rx.try_recv() {
                job = newer;
            }
            let start = Instant::now();
            let rope = Rope::from_lines(&job.lines);
            let _ = result_tx.send(OpenResult {
                token: job.token,
                rope,
                cursor_line: job.cursor_line,
                cursor_col: job.cursor_col,
                build_us: start.elapsed().as_micros() as u64,
                chunks: 1,
            });
        }
    }
}
