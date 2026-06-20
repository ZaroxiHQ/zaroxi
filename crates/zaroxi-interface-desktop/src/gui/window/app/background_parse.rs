use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use zaroxi_core_platform_syntax::highlight::{HighlightEngine, HighlightSpan};
use zaroxi_core_platform_syntax::language::LanguageId;
use zaroxi_core_platform_syntax::parser::ParserPool;

/// Maximum text bytes to send to the background parse worker.
/// Beyond this threshold, full-document tree-sitter parsing is too
/// slow to be useful and we drop the snapshot without parsing.
const MAX_PARSE_TEXT_BYTES: usize = 100_000;

/// Parse `text` with `language` and return highlight spans, or an empty vector
/// when the language is plain text, the grammar is unavailable, parsing fails,
/// or the text exceeds the parse budget.
///
/// Shared by the background worker and the synchronous first-paint highlight on
/// file open. The compiled query is cached process-wide (see
/// `HighlightEngine`), so repeated calls are cheap.
pub fn compute_spans(pool: &ParserPool, language: LanguageId, text: &str) -> Vec<HighlightSpan> {
    if language == LanguageId::PlainText || text.len() > MAX_PARSE_TEXT_BYTES {
        return Vec::new();
    }
    let mut parser = match pool.acquire(&language) {
        Some(p) => p,
        None => return Vec::new(),
    };
    let spans = match parser.parse(text, None) {
        Some(tree) => HighlightEngine::new().highlight(language, text, &tree).unwrap_or_default(),
        None => Vec::new(),
    };
    pool.release(&language, parser);
    spans
}

/// An immutable snapshot of the buffer at a specific version.
#[derive(Clone)]
pub struct BufferSnapshot {
    pub version: u64,
    pub text: String,
    pub language: LanguageId,
}

/// The result of a background parse operation.
#[derive(Clone)]
pub struct ParseResult {
    pub version: u64,
    pub spans: Vec<HighlightSpan>,
    pub incremental: bool,
    pub duration_us: u64,
}

/// A background worker that receives buffer snapshots and produces
/// tree-sitter parse results off the main thread.
///
/// Usage:
/// 1. `spawn()` creates the worker and starts the thread.
/// 2. `schedule_parse(snapshot)` sends work to the background thread.
/// 3. `poll_result()` collects completed results (stale results are discarded).
pub struct BackgroundParseWorker {
    tx: mpsc::Sender<BufferSnapshot>,
    rx: mpsc::Receiver<ParseResult>,
    _handle: Option<thread::JoinHandle<()>>,
    last_sent_version: u64,
    completed_result: Option<ParseResult>,
}

impl BackgroundParseWorker {
    /// Spawn the background worker thread.  The `ParserPool` is shared with
    /// the main thread via `Arc` so parsers can be acquired/released safely.
    pub fn spawn(pool: Arc<ParserPool>) -> Self {
        let (snap_tx, snap_rx) = mpsc::channel::<BufferSnapshot>();
        let (result_tx, result_rx) = mpsc::channel::<ParseResult>();

        let worker_pool = Arc::clone(&pool);
        let handle = thread::Builder::new()
            .name("zaroxi-parse-worker".into())
            .spawn(move || {
                Self::worker_loop(worker_pool, snap_rx, result_tx);
            })
            .ok();

        Self {
            tx: snap_tx,
            rx: result_rx,
            _handle: handle,
            last_sent_version: 0,
            completed_result: None,
        }
    }

    /// Send a new buffer snapshot to the worker for background parsing.
    /// Silently drops duplicate versions (no work queued).
    pub fn schedule_parse(&mut self, snapshot: BufferSnapshot) {
        if snapshot.version <= self.last_sent_version {
            return;
        }
        self.last_sent_version = snapshot.version;
        let _ = self.tx.send(snapshot);
    }

    /// Return the latest version that was sent to the worker.
    pub fn latest_version(&self) -> u64 {
        self.last_sent_version
    }

    /// Poll for completed parse results from the worker.
    /// Stale results (version < last_sent_version) are discarded.
    pub fn poll_result(&mut self) -> Option<&ParseResult> {
        while let Ok(result) = self.rx.try_recv() {
            let debug_pipeline = std::env::var("ZAROXI_DEBUG_PARSE_PIPELINE").as_deref() == Ok("1");
            if debug_pipeline {
                eprintln!(
                    "ZAROXI_DEBUG_PARSE_PIPELINE: result_received v={} incremental={} dur_us={}",
                    result.version, result.incremental, result.duration_us,
                );
            }
            if result.version >= self.last_sent_version {
                if debug_pipeline {
                    eprintln!(
                        "ZAROXI_DEBUG_PARSE_PIPELINE: result_accepted v={} span_count={}",
                        result.version,
                        result.spans.len(),
                    );
                }
                self.completed_result = Some(result);
            } else if debug_pipeline {
                eprintln!(
                    "ZAROXI_DEBUG_PARSE_PIPELINE: result_rejected v={} (stale, current={})",
                    result.version, self.last_sent_version,
                );
            }
        }
        self.completed_result.as_ref()
    }

    /// Clear the completed result so it isn't reused on the next poll.
    pub fn clear_result(&mut self) {
        self.completed_result = None;
    }

    fn worker_loop(
        pool: Arc<ParserPool>,
        snap_rx: mpsc::Receiver<BufferSnapshot>,
        result_tx: mpsc::Sender<ParseResult>,
    ) {
        let debug_pipeline = std::env::var("ZAROXI_DEBUG_PARSE_PIPELINE").as_deref() == Ok("1");

        while let Ok(snapshot) = snap_rx.recv() {
            let start = Instant::now();
            let lang = snapshot.language;

            if debug_pipeline {
                eprintln!(
                    "ZAROXI_DEBUG_PARSE_PIPELINE: worker_task v={} lang={:?} text_bytes={}",
                    snapshot.version,
                    lang,
                    snapshot.text.len(),
                );
            }

            // Defence-in-depth: reject snapshots whose text payload exceeds
            // the parse budget.  The caller-level guard in
            // `schedule_background_parse` should already prevent these, but
            // a stale snapshot from before the guard was tightened could
            // still be in the channel.
            if snapshot.text.len() > MAX_PARSE_TEXT_BYTES {
                if debug_pipeline {
                    eprintln!(
                        "ZAROXI_DEBUG_PARSE_PIPELINE: worker_task DROPPED text_bytes={} (exceeds max={})",
                        snapshot.text.len(),
                        MAX_PARSE_TEXT_BYTES,
                    );
                }
                let _ = result_tx.send(ParseResult {
                    version: snapshot.version,
                    spans: Vec::new(),
                    incremental: false,
                    duration_us: start.elapsed().as_micros() as u64,
                });
                continue;
            }

            if lang == LanguageId::PlainText {
                let _ = result_tx.send(ParseResult {
                    version: snapshot.version,
                    spans: Vec::new(),
                    incremental: false,
                    duration_us: start.elapsed().as_micros() as u64,
                });
                continue;
            }

            let mut parser = match pool.acquire(&lang) {
                Some(p) => p,
                None => {
                    let _ = result_tx.send(ParseResult {
                        version: snapshot.version,
                        spans: Vec::new(),
                        incremental: false,
                        duration_us: start.elapsed().as_micros() as u64,
                    });
                    continue;
                }
            };

            let tree = parser.parse(&snapshot.text, None);
            let spans: Vec<HighlightSpan> = match tree.as_ref() {
                Some(t) => {
                    let engine = HighlightEngine::new();
                    engine.highlight(lang, &snapshot.text, t).unwrap_or_default()
                }
                None => Vec::new(),
            };

            pool.release(&lang, parser);

            let dur = start.elapsed().as_micros() as u64;
            if debug_pipeline {
                eprintln!(
                    "ZAROXI_DEBUG_PARSE_PIPELINE: worker_done v={} dur_us={} span_count={}",
                    snapshot.version,
                    dur,
                    spans.len(),
                );
            }

            let _ = result_tx.send(ParseResult {
                version: snapshot.version,
                spans,
                incremental: false,
                duration_us: dur,
            });
        }
    }
}
