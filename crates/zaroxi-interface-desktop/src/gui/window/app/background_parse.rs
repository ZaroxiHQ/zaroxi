use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use zaroxi_core_platform_syntax::highlight::{HighlightEngine, HighlightSpan};
use zaroxi_core_platform_syntax::language::LanguageId;
use zaroxi_core_platform_syntax::parser::ParserPool;

/// Maximum text bytes the syntax parser will accept, **derived from the backend
/// large-file boundary** (`DocumentBuffer::LARGE_THRESHOLD`) so syntax policy
/// and backend selection share ONE threshold and can never disagree:
///   - Rope-backed *normal* files (size < `LARGE_THRESHOLD`) hold their FULL
///     text in the rope, so `to_string()` is always within budget → full-
///     document syntax is enabled by default.
///   - *Large* files (size >= `LARGE_THRESHOLD`) are PieceTable-backed and the
///     rope holds only the viewport window, so `to_string()` is small → the
///     parse is viewport-scoped (the explicit reduced large-file policy).
///
/// There is intentionally NO separate, lower "medium file" cutoff. The previous
/// hard-coded 100 KB constant was exactly such a hidden second threshold: it
/// left rope-backed mid-size files (e.g. a ~124 KB Rust source) rendered as
/// plain text even though `large_file_mode == false` — syntax and backend
/// silently disagreed, which is the bug this unifies away.
const MAX_PARSE_TEXT_BYTES: usize =
    zaroxi_core_editor_largefile::DocumentBuffer::LARGE_THRESHOLD as usize;

/// Whether the syntax-policy decision trace is enabled (`ZAROXI_DEBUG_SYNTAX=1`,
/// also implied by `ZAROXI_DEBUG_PARSE_PIPELINE=1`). Prints exactly one policy/
/// outcome reason per parse so an empty-span result is never silent.
fn syntax_trace_enabled() -> bool {
    std::env::var("ZAROXI_DEBUG_SYNTAX").as_deref() == Ok("1")
        || std::env::var("ZAROXI_DEBUG_PARSE_PIPELINE").as_deref() == Ok("1")
}

/// Compute highlight spans AND the single authoritative policy/outcome reason.
///
/// Returns exactly one of:
///   - `enabled`                        — spans produced for the snapshot
///   - `disabled_by_language_detection` — plain text / no grammar for the language
///   - `disabled_by_budget_policy`      — text exceeds the (large-file) byte budget
///   - `parse_failed`                   — grammar unavailable or tree-sitter parse failed
///   - `empty_result_unexpected`        — a supported language parsed but produced
///                                        zero spans (a bug path worth flagging)
fn highlight_with_reason(
    pool: &ParserPool,
    language: LanguageId,
    text: &str,
) -> (Vec<HighlightSpan>, &'static str) {
    if language == LanguageId::PlainText {
        return (Vec::new(), "disabled_by_language_detection");
    }
    if text.len() > MAX_PARSE_TEXT_BYTES {
        return (Vec::new(), "disabled_by_budget_policy");
    }
    let mut parser = match pool.acquire(&language) {
        Some(p) => p,
        None => return (Vec::new(), "parse_failed"),
    };
    let tree = parser.parse(text, None);
    let spans = match tree.as_ref() {
        Some(t) => HighlightEngine::new().highlight(language, text, t).unwrap_or_default(),
        None => Vec::new(),
    };
    pool.release(&language, parser);
    let reason = if tree.is_none() {
        "parse_failed"
    } else if spans.is_empty() {
        "empty_result_unexpected"
    } else {
        "enabled"
    };
    (spans, reason)
}

/// Parse `text` with `language` and return highlight spans (synchronous path:
/// first-paint / edit-time re-highlight). Empty when the language is plain text,
/// the grammar is unavailable, parsing fails, or the text exceeds the
/// (large-file) parse budget.
///
/// Shared with the background worker. The compiled query is cached process-wide
/// (see `HighlightEngine`), so repeated calls are cheap.
pub fn compute_spans(pool: &ParserPool, language: LanguageId, text: &str) -> Vec<HighlightSpan> {
    let (spans, reason) = highlight_with_reason(pool, language, text);
    if syntax_trace_enabled() {
        eprintln!(
            "ZAROXI_DEBUG_SYNTAX: syntax={} path=sync lang={:?} text_bytes={} budget={} span_count={}",
            reason,
            language,
            text.len(),
            MAX_PARSE_TEXT_BYTES,
            spans.len(),
        );
    }
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
            } else {
                if syntax_trace_enabled() {
                    eprintln!(
                        "ZAROXI_DEBUG_SYNTAX: syntax=cancelled v={} current={} span_count={} reason=superseded_by_newer_version",
                        result.version,
                        self.last_sent_version,
                        result.spans.len(),
                    );
                }
                if debug_pipeline {
                    eprintln!(
                        "ZAROXI_DEBUG_PARSE_PIPELINE: result_rejected v={} (stale, current={})",
                        result.version, self.last_sent_version,
                    );
                }
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

            // Single source of truth for the syntax policy + outcome. The byte
            // budget here is the SAME constant as the backend large-file
            // boundary, so a rope-backed normal file is never rejected and a
            // large file's (already viewport-scoped) text always parses.
            let (spans, reason) = highlight_with_reason(&pool, lang, &snapshot.text);
            let dur = start.elapsed().as_micros() as u64;

            if syntax_trace_enabled() {
                eprintln!(
                    "ZAROXI_DEBUG_SYNTAX: syntax={} path=worker v={} lang={:?} text_bytes={} budget={} span_count={} dur_us={}",
                    reason,
                    snapshot.version,
                    lang,
                    snapshot.text.len(),
                    MAX_PARSE_TEXT_BYTES,
                    spans.len(),
                    dur,
                );
            }
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
