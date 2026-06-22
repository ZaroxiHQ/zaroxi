//! Background file-open *prep* worker: off-thread disk read / buffer load, with
//! a fast first-screenful **head preview** (Phase 10).
//!
//! Opening a file used to `pollster::block_on(service.open_buffer(..))` on the
//! UI thread; Phase 8 moved that read off-thread. Phase 10 additionally emits a
//! cheap *head preview* (the first ~screenful of lines, read with a bounded
//! `BufReader`) BEFORE the full buffer is decoded, so the visible rows can paint
//! almost immediately on a huge file instead of waiting for the whole read.
//!
//! The head read uses the same UTF-8 + `'\n'` line decode as the service's
//! `read_to_string` open path, so the preview's visible rows are byte-identical
//! to the full buffer's — the swap from preview to full does not flash the
//! visible screenful (only below-the-fold rows fill in).
//!
//! Token awareness mirrors [`super::background_open::BackgroundOpenWorker`]: each
//! job carries a read token, the worker drains its queue to the newest job and
//! skips any read whose token the shared `generation` atomic has already
//! superseded (clicking away from a huge file does not pay for its full read).

use std::io::BufRead;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use zaroxi_application_workspace::ports::{OpenBufferRequest, SessionId};

use crate::ports::{BufferId, WorkspaceService};

/// How many leading lines the head preview reads. A bit more than one screenful
/// + overscan so the first visible rows are covered without reading the file.
const MAX_HEAD_LINES: usize = 80;

/// A tokened request to read/load a file's buffer off-thread.
pub struct ReadJob {
    pub token: u64,
    pub service: Arc<dyn WorkspaceService>,
    pub session_id: SessionId,
    pub path: PathBuf,
}

/// Outcomes the worker streams back for one job: first a cheap `Head` preview,
/// then the `Full` buffer once the whole read completes.
pub enum ReadOutcome {
    /// First-screenful preview lines (display only; not yet the registered buffer).
    Head { token: u64, lines: Vec<String>, complete: bool },
    /// The fully read/registered buffer, ready to activate on the UI thread.
    Full { token: u64, buffer_id: Option<BufferId>, read_ms: f32, cancelled: bool, path: PathBuf },
}

impl ReadOutcome {
    /// The open token this outcome belongs to.
    pub fn token(&self) -> u64 {
        match self {
            ReadOutcome::Head { token, .. } => *token,
            ReadOutcome::Full { token, .. } => *token,
        }
    }
}

/// Background worker owning the blocking disk read / buffer load + head preview
/// for the winning open token. The UI thread only schedules jobs and finalizes
/// ready outcomes.
pub struct BackgroundReadWorker {
    tx: mpsc::Sender<ReadJob>,
    rx: mpsc::Receiver<ReadOutcome>,
    _handle: Option<thread::JoinHandle<()>>,
    last_sent_token: u64,
}

impl BackgroundReadWorker {
    /// Spawn the background read worker thread, sharing the `generation` atomic
    /// (the latest requested read token) so it can skip superseded reads.
    pub fn spawn(generation: Arc<AtomicU64>) -> Self {
        let (job_tx, job_rx) = mpsc::channel::<ReadJob>();
        let (outcome_tx, outcome_rx) = mpsc::channel::<ReadOutcome>();

        let handle = thread::Builder::new()
            .name("zaroxi-read-worker".into())
            .spawn(move || {
                Self::worker_loop(job_rx, outcome_tx, generation);
            })
            .ok();

        Self { tx: job_tx, rx: outcome_rx, _handle: handle, last_sent_token: 0 }
    }

    /// Send a new read job. Drops jobs whose token is not newer than the last
    /// one already scheduled (no stale work queued).
    pub fn schedule_read(&mut self, job: ReadJob) {
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

    /// Drain all pending outcomes (head + full, possibly several jobs) in order.
    /// The caller filters by token: stale-token outcomes are dropped.
    pub fn drain(&mut self) -> Vec<ReadOutcome> {
        let mut out = Vec::new();
        while let Ok(o) = self.rx.try_recv() {
            out.push(o);
        }
        out
    }

    /// Read up to `MAX_HEAD_LINES` leading lines of `path`, decoded as UTF-8 and
    /// split on `'\n'` (a trailing `'\n'` is stripped, any `'\r'` is kept) so the
    /// result matches `content.split('\n')` of the full `read_to_string` open
    /// path. Returns `(lines, complete)` where `complete` means EOF was reached
    /// within the cap (the whole file fit in the preview).
    fn read_head_lines(path: &std::path::Path) -> Option<(Vec<String>, bool)> {
        let file = std::fs::File::open(path).ok()?;
        let mut reader = std::io::BufReader::new(file);
        let mut lines: Vec<String> = Vec::with_capacity(MAX_HEAD_LINES);
        let mut complete = false;
        loop {
            if lines.len() >= MAX_HEAD_LINES {
                break;
            }
            let mut buf = String::new();
            let n = reader.read_line(&mut buf).ok()?;
            if n == 0 {
                // EOF: the whole file fit within the cap.
                complete = true;
                break;
            }
            if buf.ends_with('\n') {
                buf.pop();
            }
            lines.push(buf);
        }
        Some((lines, complete))
    }

    fn worker_loop(
        job_rx: mpsc::Receiver<ReadJob>,
        outcome_tx: mpsc::Sender<ReadOutcome>,
        generation: Arc<AtomicU64>,
    ) {
        while let Ok(mut job) = job_rx.recv() {
            // Coalesce: skip straight to the newest queued job.
            while let Ok(newer) = job_rx.try_recv() {
                job = newer;
            }
            // Cancellation: skip a read the latest click already superseded.
            if job.token < generation.load(Ordering::Relaxed) {
                let _ = outcome_tx.send(ReadOutcome::Full {
                    token: job.token,
                    buffer_id: None,
                    read_ms: 0.0,
                    cancelled: true,
                    path: job.path,
                });
                continue;
            }

            // ── Fast head preview (first screenful) ──
            if let Some((lines, complete)) = Self::read_head_lines(&job.path)
                && !lines.is_empty()
            {
                let _ = outcome_tx.send(ReadOutcome::Head { token: job.token, lines, complete });
            }

            // ── Full buffer read/register (the heavy part) ──
            let start = Instant::now();
            let req = OpenBufferRequest { session_id: job.session_id, path: job.path.clone() };
            let buffer_id = match pollster::block_on(job.service.open_buffer(req)) {
                Ok(resp) => Some(resp.buffer_id),
                Err(e) => {
                    log::warn!("background read open_buffer failed: {:?}", e);
                    None
                }
            };
            let _ = outcome_tx.send(ReadOutcome::Full {
                token: job.token,
                buffer_id,
                read_ms: start.elapsed().as_secs_f32() * 1000.0,
                cancelled: false,
                path: job.path,
            });
        }
    }
}
