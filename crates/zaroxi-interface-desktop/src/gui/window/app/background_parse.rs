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
///     zero spans (a bug path worth flagging)
fn highlight_with_reason(
    pool: &ParserPool,
    language: LanguageId,
    text: &str,
) -> (Vec<HighlightSpan>, &'static str, bool) {
    if language == LanguageId::PlainText {
        return (Vec::new(), "disabled_by_language_detection", false);
    }
    if text.len() > MAX_PARSE_TEXT_BYTES {
        return (Vec::new(), "disabled_by_budget_policy", false);
    }
    let mut parser = match pool.acquire(&language) {
        Some(p) => p,
        None => return (Vec::new(), "parse_failed", true),
    };
    let tree = parser.parse(text, None);
    // `had_error` is true when Tree-sitter could not fully parse the text — the
    // tree contains an ERROR / MISSING node. This is the key signal that the
    // highlight result is a DEGRADED partial (error recovery collapses coverage,
    // dramatically so for whitespace-sensitive grammars like YAML), so the
    // caller can prefer a retained full-coverage baseline instead.
    let had_error = match tree.as_ref() {
        Some(t) => t.root_node().has_error(),
        None => true,
    };
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
    (spans, reason, had_error)
}

/// The full-buffer highlight computation plus the metadata needed to enforce
/// coverage precedence (Part 5): whether the parse was degraded (`had_error`),
/// the byte length of the parsed source, and how far the spans actually reach
/// (`coverage_end`). A degraded result whose coverage falls far short of the
/// source signals that Tree-sitter error recovery collapsed downstream
/// highlighting and a retained full-coverage baseline should be preferred.
#[derive(Debug, Clone)]
pub struct SpanComputation {
    pub spans: Vec<HighlightSpan>,
    /// Tree-sitter reported an ERROR/MISSING node (partial/degraded parse).
    pub had_error: bool,
    /// Byte length of the exact source text that was parsed.
    pub source_len: usize,
    /// Highest `span.end` produced — the last byte the highlighting reaches.
    pub coverage_end: usize,
}

/// Compute highlight spans for `text` and return them alongside coverage
/// metadata (see [`SpanComputation`]). This is the synchronous edit-time /
/// first-paint entry point; the coordinate space is ALWAYS full-buffer byte
/// offsets into `text`.
pub fn compute_spans_detailed(
    pool: &ParserPool,
    language: LanguageId,
    text: &str,
) -> SpanComputation {
    let (spans, reason, had_error) = highlight_with_reason(pool, language, text);
    let coverage_end = spans.iter().map(|s| s.end).max().unwrap_or(0);
    if syntax_trace_enabled() {
        eprintln!(
            "ZAROXI_DEBUG_SYNTAX: syntax={} path=sync lang={:?} text_bytes={} budget={} span_count={} had_error={} coverage_end={}",
            reason,
            language,
            text.len(),
            MAX_PARSE_TEXT_BYTES,
            spans.len(),
            had_error,
            coverage_end,
        );
    }
    SpanComputation { spans, had_error, source_len: text.len(), coverage_end }
}

/// A retained, error-free full-buffer highlight baseline for the active NORMAL
/// file. When a later edit parses to a DEGRADED result (Tree-sitter error
/// recovery collapses coverage), the unchanged suffix of this baseline is
/// remapped across the edit so downstream lines keep their correct highlighting
/// instead of falling back to plain text. Only ever holds a full_buffer result
/// (never a viewport window), and only for normal files.
#[derive(Debug, Clone)]
pub struct GoodHighlight {
    /// Full buffer text this baseline was parsed from.
    pub text: String,
    /// Full-buffer byte-offset spans (error-free coverage).
    pub spans: Vec<HighlightSpan>,
    /// Buffer version the baseline was computed at.
    pub version: u64,
    /// Owning file identity (`committed_active_file`).
    pub owner: Option<String>,
}

/// An immutable snapshot of the buffer at a specific version.
#[derive(Clone)]
pub struct BufferSnapshot {
    pub version: u64,
    pub text: String,
    pub language: LanguageId,
    /// Canonical active-file identity this snapshot was taken from. Echoed back
    /// on the [`ParseResult`] so the UI thread can enforce strict span
    /// ownership: an async result whose owner no longer matches the active file
    /// is dropped rather than painted onto the wrong document.
    pub owner: Option<String>,
}

/// The result of a background parse operation.
#[derive(Clone)]
pub struct ParseResult {
    pub version: u64,
    pub spans: Vec<HighlightSpan>,
    pub incremental: bool,
    pub duration_us: u64,
    /// The file identity the parsed snapshot belonged to (see
    /// [`BufferSnapshot::owner`]).
    pub owner: Option<String>,
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
            let owner = snapshot.owner.clone();

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
            let (spans, reason, _had_error) = highlight_with_reason(&pool, lang, &snapshot.text);
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
                owner,
            });
        }
    }
}
