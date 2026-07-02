//! Apply syntax-highlight spans to editor source lines.
//!
//! Phase 1 syntax-highlighting source of truth:
//! - Tree-sitter parsing happens off the main thread in
//!   `app::background_parse::BackgroundParseWorker` using the language
//!   detected from the file path (`LanguageId::from_path`).
//! - The accepted `ParseResult` is stored on `GuiApp` as
//!   `latest_spans` (full-document byte-offset `HighlightSpan`s).
//! - This module is pure presentation: it maps those stored spans onto the
//!   editor's source lines.  It performs NO parsing and is language-agnostic
//!   (it never references a concrete `LanguageId`).
//!
//! Byte-offset contract: `lines.join("\n")` here is byte-identical to the
//! text the worker parsed (`EditorBufferState::to_string()`), because the
//! buffer is populated from the same lines joined by `"\n"`.  Therefore the
//! absolute byte offsets carried by `HighlightSpan` line up directly with the
//! per-line byte offsets computed below.

use std::collections::HashMap;

/// A run of colored text segments `(text, rgba)` for a single line.
pub type ColoredSpans = Vec<(String, [f32; 4])>;

/// Per-line syntax-colored span cache keyed by `(line_index, content_fnv_hash)`.
pub type LineSyntaxCache = HashMap<(usize, u64), ColoredSpans>;

use zaroxi_core_platform_syntax::highlight::{Highlight, HighlightSpan};
use zaroxi_interface_theme::theme::SemanticColors;

fn highlight_color(h: Highlight, sem: &SemanticColors, default: [f32; 4]) -> [f32; 4] {
    let to_f32 = |c: &zaroxi_interface_theme::Color| -> [f32; 4] { [c.r, c.g, c.b, c.a] };
    match h {
        Highlight::Comment => to_f32(&sem.syntax_comment),
        Highlight::String => to_f32(&sem.syntax_string),
        Highlight::Keyword => to_f32(&sem.syntax_keyword),
        Highlight::Function => to_f32(&sem.syntax_function),
        Highlight::Type => to_f32(&sem.syntax_type),
        Highlight::Number => to_f32(&sem.syntax_number),
        Highlight::Constant => to_f32(&sem.syntax_constant),
        Highlight::Variable => to_f32(&sem.syntax_variable),
        Highlight::Operator => to_f32(&sem.syntax_operator),
        Highlight::Punctuation => to_f32(&sem.syntax_punctuation),
        Highlight::Attribute => to_f32(&sem.syntax_attribute),
        Highlight::Property => to_f32(&sem.syntax_property),
        Highlight::Namespace => to_f32(&sem.syntax_namespace),
        Highlight::Plain => default,
    }
}

/// Whether the decoration trace is enabled, evaluated once per process so the
/// per-token tracer below is a single atomic load (zero-cost) when disabled.
fn decoration_trace_on() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("ZAROXI_DEBUG_DECORATION").as_deref() == Ok("1"))
}

/// Whether the span-application trace is enabled (`ZAROXI_DEBUG_SYNTAX_APPLY=1`).
/// Evaluated once per process so the hot path is a single atomic load when off.
///
/// When enabled it proves — per colorize pass — exactly which spans were
/// applied, which were rejected (and why), and how many bytes fell back to
/// plain because no valid current span covered them.
fn span_apply_trace_on() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("ZAROXI_DEBUG_SYNTAX_APPLY").as_deref() == Ok("1"))
}

/// Slice `s[start..end]` **only** when the range is well-formed against the
/// CURRENT text: ordered, in-bounds, and landing on UTF-8 char boundaries.
/// Any ill-formed range yields `""` instead of panicking — the strict
/// span-ownership invariant means text that a valid current span does not
/// cleanly cover must never be colored (and must never crash the renderer).
fn safe_slice(s: &str, start: usize, end: usize) -> &str {
    if start <= end && end <= s.len() && s.is_char_boundary(start) && s.is_char_boundary(end) {
        &s[start..end]
    } else {
        ""
    }
}

/// The highest `span.end` in a set — how far the highlighting reaches, in
/// full-buffer byte offsets. `0` for an empty set.
pub fn coverage_end(spans: &[HighlightSpan]) -> usize {
    spans.iter().map(|s| s.end).max().unwrap_or(0)
}

/// Byte length of the longest common prefix of `a` and `b`, snapped DOWN to a
/// UTF-8 char boundary of both strings so callers never split a codepoint.
pub fn common_prefix_len(a: &str, b: &str) -> usize {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let max = ab.len().min(bb.len());
    let mut i = 0;
    while i < max && ab[i] == bb[i] {
        i += 1;
    }
    // Snap back to a boundary valid in both strings.
    while i > 0 && (!a.is_char_boundary(i) || !b.is_char_boundary(i)) {
        i -= 1;
    }
    i
}

/// Byte length of the longest common suffix of `a` and `b` that does not overlap
/// the already-matched `prefix`, snapped UP to a char boundary of both strings.
pub fn common_suffix_len(a: &str, b: &str, prefix: usize) -> usize {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let max = ab.len().min(bb.len()).saturating_sub(prefix);
    let mut i = 0;
    while i < max && ab[ab.len() - 1 - i] == bb[bb.len() - 1 - i] {
        i += 1;
    }
    // Snap DOWN (shrink the suffix) until both suffix-start offsets land on a
    // char boundary — never split a multi-byte codepoint.
    while i > 0 && (!a.is_char_boundary(a.len() - i) || !b.is_char_boundary(b.len() - i)) {
        i -= 1;
    }
    i
}

/// Byte offset of the first character of the line containing `pos` (i.e. one
/// past the previous `'\n'`, or `0`). `pos` must be a char boundary.
fn line_start_byte(text: &str, pos: usize) -> usize {
    let pos = pos.min(text.len());
    match text[..pos].rfind('\n') {
        Some(i) => i + 1,
        None => 0,
    }
}

/// Byte offset of the end of the line containing `pos` — the index of the next
/// `'\n'` at/after `pos` (exclusive of that newline), or `text.len()`. `pos`
/// must be a char boundary.
fn line_end_byte(text: &str, pos: usize) -> usize {
    let pos = pos.min(text.len());
    match text[pos..].find('\n') {
        Some(i) => pos + i,
        None => text.len(),
    }
}

/// Subtract the half-open byte range `[hole_start, hole_end)` from `span`,
/// returning the 0, 1, or 2 pieces that remain strictly OUTSIDE the hole. This
/// is the hard guarantee that no retained span can bleed into the degraded edit
/// hole: a span entirely inside the hole disappears; one straddling a boundary
/// is trimmed; one spanning the whole hole is split in two. UTF-8 safety is
/// preserved because `span`'s own offsets are already char boundaries and the
/// hole bounds are computed on char boundaries.
fn subtract_hole(span: &HighlightSpan, hole_start: usize, hole_end: usize) -> Vec<HighlightSpan> {
    let mut pieces = Vec::with_capacity(2);
    // Left remnant: the part before the hole.
    if span.start < hole_start {
        let end = span.end.min(hole_start);
        if span.start < end {
            pieces.push(HighlightSpan { start: span.start, end, highlight: span.highlight });
        }
    }
    // Right remnant: the part after the hole.
    if span.end > hole_end {
        let start = span.start.max(hole_end);
        if start < span.end {
            pieces.push(HighlightSpan { start, end: span.end, highlight: span.highlight });
        }
    }
    pieces
}

/// Remap an error-free, full-coverage span set computed for `old_text` onto
/// `new_text` when a FRESH parse of `new_text` came back DEGRADED (Tree-sitter
/// error recovery collapsed downstream coverage).
///
/// The text change is derived purely from a prefix/suffix diff of the two
/// buffers (no per-edit byte tracking needed, so it is correct for insert,
/// delete, replace, paste, multi-line, and undo alike). The result is:
///   - retained unchanged PREFIX spans,
///   - retained unchanged SUFFIX spans, shifted by the byte delta,
///   - an explicit PLAIN HOLE over the actively edited line(s): NO span — neither
///     retained nor fresh — may color a single byte inside it.
///
/// The hole is expanded from the exact changed byte range out to the full
/// touched line(s) (`line_start_byte(prefix)` .. `line_end_byte(last changed
/// byte)`), so the edited line stays uniformly plain during an invalid moment
/// instead of showing a confusing partial coloring. Fresh degraded spans are
/// intentionally NOT overlaid — the hole is authoritative until a clean parse
/// lands and replaces this result wholesale.
///
/// The suffix is byte-identical text, so its shifted spans land on exactly the
/// same characters: an exact translation of a previously-correct parse, not a
/// guess. Result is in `new_text` full-buffer coordinates, sorted by start;
/// final bounds/char-boundary safety is enforced later by
/// [`validate_spans_for_source`].
pub fn remap_good_spans_across_edit(
    old_text: &str,
    new_text: &str,
    good_spans: &[HighlightSpan],
) -> Vec<HighlightSpan> {
    let old_len = old_text.len();
    let new_len = new_text.len();
    let prefix = common_prefix_len(old_text, new_text);
    let suffix = common_suffix_len(old_text, new_text, prefix);
    let old_change_end = old_len - suffix; // exclusive end of change (old coords)
    let new_change_end = new_len - suffix; // exclusive end of change (new coords)
    let delta = new_len as isize - old_len as isize;

    // ── Full-line edit hole (new-text coordinates) ──
    // Expand the exact changed byte range to whole touched line(s) so the edited
    // line renders uniformly plain. `last_changed` is the last byte the change
    // actually touched (exclusive→inclusive); for a pure deletion the changed
    // region is empty, so we anchor on the line at `prefix`.
    let hole_start = line_start_byte(new_text, prefix);
    let last_changed = if new_change_end > prefix { new_change_end - 1 } else { prefix };
    let hole_end = line_end_byte(new_text, last_changed);

    let trace = span_apply_trace_on();
    if trace {
        let line_start =
            new_text[..hole_start.min(new_len)].bytes().filter(|&b| b == b'\n').count();
        let line_end = new_text[..hole_end.min(new_len)].bytes().filter(|&b| b == b'\n').count();
        eprintln!(
            "ZAROXI_DEBUG_SYNTAX_APPLY: syntax_edit_hole start={hole_start} end={hole_end} line_start={line_start} line_end={line_end} mode=full_lines",
        );
    }

    let mut out: Vec<HighlightSpan> = Vec::with_capacity(good_spans.len());

    let mut push_outside_hole = |span: HighlightSpan, source: &'static str| {
        // Fast path: entirely outside the hole.
        if span.end <= hole_start || span.start >= hole_end {
            out.push(span);
            return;
        }
        // Straddles or is inside the hole → keep only the outside remnants.
        let pieces = subtract_hole(&span, hole_start, hole_end);
        if trace {
            eprintln!(
                "ZAROXI_DEBUG_SYNTAX_APPLY: syntax_span_dropped reason=intersects_edit_hole source={source} start={} end={} kept_pieces={}",
                span.start,
                span.end,
                pieces.len(),
            );
        }
        out.extend(pieces);
    };

    for s in good_spans {
        if s.end <= prefix {
            // Unchanged prefix span (verbatim) — still trimmed against the hole,
            // because a same-line span before the edit point lies inside the
            // full-line hole and must not color the edited line.
            push_outside_hole(s.clone(), "retained_prefix");
        } else if s.start >= old_change_end {
            // Unchanged suffix span, shifted by the edit delta — trimmed against
            // the hole so the tail of the edited line is not colored.
            let ns = (s.start as isize + delta).max(0) as usize;
            let ne = (s.end as isize + delta).max(0) as usize;
            push_outside_hole(
                HighlightSpan { start: ns, end: ne, highlight: s.highlight },
                "retained_suffix",
            );
        }
        // else: overlaps the changed region in old coords → dropped entirely.
    }

    out.sort_by_key(|s| s.start);
    out
}

/// Strict span-ownership validation: keep only spans whose byte range is valid
/// against `source` — non-empty, in-bounds, and on UTF-8 char boundaries.
///
/// This is the hard gate that stops a stale / shifted / out-of-bounds span
/// (e.g. one computed from the pre-edit buffer, or a large-file viewport span
/// left over after a scroll) from coloring post-edit text. A span that fails
/// ANY check is dropped, so the text it would have covered renders plain.
///
/// Rejected spans are counted and — under `ZAROXI_DEBUG_SYNTAX_APPLY=1` —
/// reported individually with a machine-greppable reason.
pub fn validate_spans_for_source(spans: &[HighlightSpan], source: &str) -> Vec<HighlightSpan> {
    let source_len = source.len();
    let trace = span_apply_trace_on();
    let mut valid: Vec<HighlightSpan> = Vec::with_capacity(spans.len());
    let mut rejected = 0usize;
    for s in spans {
        let reason = if s.start >= s.end {
            Some("invalid_range")
        } else if s.end > source_len {
            Some("out_of_bounds")
        } else if !source.is_char_boundary(s.start) || !source.is_char_boundary(s.end) {
            Some("shifted_not_char_boundary")
        } else {
            None
        };
        match reason {
            None => valid.push(s.clone()),
            Some(reason) => {
                rejected += 1;
                if trace {
                    eprintln!(
                        "ZAROXI_DEBUG_SYNTAX_APPLY: syntax_spans_rejected reason={reason} start={} end={} source_len={} highlight={:?}",
                        s.start, s.end, source_len, s.highlight,
                    );
                }
            }
        }
    }
    if trace {
        eprintln!(
            "ZAROXI_DEBUG_SYNTAX_APPLY: syntax_spans_applied span_count={} rejected={} source_len={}",
            valid.len(),
            rejected,
            source_len,
        );
    }
    valid
}

/// Bounded, guarded per-token foreground-color dump (`ZAROXI_DEBUG_DECORATION=1`).
///
/// Proves the color each syntax token receives, with NO diff/row modulation
/// applied — text color comes purely from the highlight→`syntax_*` mapping and is
/// independent of diff state. This is the evidence that, e.g., TOML `@property`
/// keys resolve to `syntax_property` (#d07277, the exact same hue as the diff
/// `error` red) while `@operator` (`=`) is cyan. Capped to a handful of runs so
/// it never floods a whole document.
fn maybe_trace_token_color(highlight: Highlight, text: &str, color: [f32; 4]) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static TRACED: AtomicUsize = AtomicUsize::new(0);
    const MAX: usize = 24;
    if !decoration_trace_on() || TRACED.fetch_add(1, Ordering::Relaxed) >= MAX {
        return;
    }
    let snippet: String = text.chars().take(16).collect();
    eprintln!(
        "ZAROXI_DEBUG_DECORATION: syntax_token highlight={highlight:?} text_modulation=none rgba=({:.3},{:.3},{:.3},{:.3}) text={snippet:?}",
        color[0], color[1], color[2], color[3],
    );
}

/// Colorize editor source lines by applying the supplied (full-document)
/// highlight spans.  Returns per-line colored spans as `(text, [r, g, b, a])`,
/// including `"\n"` separators between lines (matching the renderer's
/// span-emission contract).
pub fn colorize_source(
    lines: &[String],
    sem: &SemanticColors,
    spans: &[HighlightSpan],
) -> Vec<(String, [f32; 4])> {
    let source = lines.join("\n");
    let default_color: [f32; 4] =
        [sem.text_primary.r, sem.text_primary.g, sem.text_primary.b, sem.text_primary.a];

    // Strict ownership: only spans that are valid against the EXACT text being
    // rendered may color it. Everything else falls back to plain.
    let spans = validate_spans_for_source(spans, &source);

    let mut result: Vec<(String, [f32; 4])> = Vec::new();
    let mut byte_offset = 0usize;

    for line in lines {
        extract_line_spans(line, byte_offset, &source, &spans, &default_color, sem, &mut result);
        result.push(("\n".to_string(), default_color));
        byte_offset += line.len() + 1;
    }

    result
}

/// Colorize only the lines in a viewport window, rebasing the full-document
/// highlight spans into window-local byte coordinates.
///
/// `window_lines` are the visible (overscanned) lines; `window_base_byte` is the
/// absolute byte offset of the first window line within the full document. The
/// returned runs cover exactly `window_lines` (with `"\n"` separators), so the
/// renderer emits only viewport rows rather than the whole document — this is
/// the key to bounding per-frame text/clone cost on large files.
pub fn colorize_window(
    window_lines: &[String],
    window_base_byte: usize,
    spans: &[HighlightSpan],
    sem: &SemanticColors,
) -> Vec<(String, [f32; 4])> {
    // Window byte length (lines joined by '\n').
    let window_len: usize = if window_lines.is_empty() {
        0
    } else {
        window_lines.iter().map(|l| l.len()).sum::<usize>() + (window_lines.len() - 1)
    };
    let window_end_byte = window_base_byte + window_len;

    // Rebase + clip spans that intersect the window into window-local coords.
    let mut local_spans: Vec<HighlightSpan> = Vec::new();
    for s in spans {
        if s.end <= window_base_byte || s.start >= window_end_byte {
            continue;
        }
        let start = s.start.saturating_sub(window_base_byte);
        let end = (s.end - window_base_byte).min(window_len);
        if start < end {
            local_spans.push(HighlightSpan { start, end, highlight: s.highlight });
        }
    }

    colorize_source(window_lines, sem, &local_spans)
}

/// Extract colored runs for a single line given byte-offset highlight spans.
fn extract_line_spans(
    line: &str,
    byte_offset: usize,
    source: &str,
    spans: &[HighlightSpan],
    default_color: &[f32; 4],
    sem: &SemanticColors,
    out: &mut Vec<(String, [f32; 4])>,
) {
    let line_end = byte_offset + line.len();
    let line_spans: Vec<HighlightSpan> =
        spans.iter().filter(|s| s.start < line_end && s.end > byte_offset).cloned().collect();

    if line_spans.is_empty() {
        // No current span covers this line: it renders entirely plain. This is
        // the default state — color is opt-in only through a valid current span.
        out.push((line.to_string(), *default_color));
        return;
    }

    // Bytes on this line NOT covered by any valid span; reported so newly typed
    // uncovered text can be proven to render plain by design.
    let mut uncovered_bytes = 0usize;
    let mut pos = byte_offset;
    for span in &line_spans {
        let seg_start = span.start.max(pos);
        let seg_end = span.end.min(line_end);
        // Leading gap: text before this span is uncovered → plain.
        if seg_start > pos {
            let before = safe_slice(source, pos, seg_start);
            if !before.is_empty() {
                uncovered_bytes += before.len();
                out.push((before.to_string(), *default_color));
            }
        }
        // Covered range: color ONLY the exact bytes the current span owns. A
        // range that does not slice cleanly (invalid/shifted) yields "" and is
        // left uncolored rather than "best-effort" stretched across the text.
        if seg_start < seg_end && seg_start >= pos {
            let text = safe_slice(source, seg_start, seg_end);
            if !text.is_empty() {
                let color = highlight_color(span.highlight, sem, *default_color);
                maybe_trace_token_color(span.highlight, text, color);
                out.push((text.to_string(), color));
            }
        }
        pos = seg_end.max(pos);
    }
    // Trailing gap: any remaining bytes are uncovered → plain.
    if pos < line_end {
        let after = safe_slice(source, pos, line_end);
        if !after.is_empty() {
            uncovered_bytes += after.len();
            out.push((after.to_string(), *default_color));
        }
    }

    if uncovered_bytes > 0 && span_apply_trace_on() {
        eprintln!(
            "ZAROXI_DEBUG_SYNTAX_APPLY: syntax_plain_fallback line_byte_offset={byte_offset} uncovered_bytes={uncovered_bytes}",
        );
    }
}

/// Colorize the full source from the supplied spans, reusing a per-line span
/// cache.  Only lines whose `per_line_hashes[i] != cached_line_hashes[i]` are
/// re-extracted; unchanged lines reuse their cached colored spans.
///
/// Note: when new spans arrive from the background worker the caller clears
/// `line_syntax_cache`, so a stale cache never masks fresh highlight colors.
pub fn colorize_source_incremental(
    lines: &[String],
    sem: &SemanticColors,
    spans: &[HighlightSpan],
    line_syntax_cache: &mut LineSyntaxCache,
    per_line_hashes: &[u64],
    cached_line_hashes: &[u64],
) -> ColoredSpans {
    let default_color: [f32; 4] =
        [sem.text_primary.r, sem.text_primary.g, sem.text_primary.b, sem.text_primary.a];

    let n = lines.len();
    let mut result: Vec<(String, [f32; 4])> = Vec::with_capacity(n * 6);

    let source = lines.join("\n");
    // Strict ownership: validate spans against the exact rendered text before
    // any per-line extraction or cache write, so a stale/shifted span can never
    // be baked into the per-line cache.
    let spans = validate_spans_for_source(spans, &source);
    let spans = spans.as_slice();

    // Fingerprint of the CURRENT spans generation, folded into every per-line
    // cache key. Without this, a line whose text is unchanged but whose spans
    // changed (e.g. a multi-line string/comment that now extends over it) could
    // reuse its previously-colored payload. Mixing the fingerprint means "same
    // line text, different spans generation" resolves to a DIFFERENT key, so a
    // colored payload can never be reused across span generations by accident.
    let spans_fingerprint = spans_fingerprint(spans);

    let mut byte_offset = 0usize;
    for (i, line) in lines.iter().enumerate() {
        let cur_hash = per_line_hashes.get(i).copied().unwrap_or(0);
        let prev_hash = cached_line_hashes.get(i).copied().unwrap_or(0);

        if cur_hash == prev_hash && prev_hash != 0 {
            // Reuse cached spans (only when text AND spans generation match).
            let cache_key = (i, cur_hash ^ spans_fingerprint);
            if let Some(cached_spans) = line_syntax_cache.get(&cache_key) {
                result.extend(cached_spans.clone());
            } else {
                let mut line_out = Vec::new();
                extract_line_spans(
                    line,
                    byte_offset,
                    &source,
                    spans,
                    &default_color,
                    sem,
                    &mut line_out,
                );
                line_syntax_cache.insert(cache_key, line_out.clone());
                result.extend(line_out);
            }
        } else {
            let mut line_out: Vec<(String, [f32; 4])> = Vec::new();
            extract_line_spans(
                line,
                byte_offset,
                &source,
                spans,
                &default_color,
                sem,
                &mut line_out,
            );
            let cache_key = (i, cur_hash ^ spans_fingerprint);
            line_syntax_cache.insert(cache_key, line_out.clone());
            result.extend(line_out);
        }

        if i + 1 < n {
            result.push(("\n".to_string(), default_color));
        }
        byte_offset += line.len() + 1;
    }

    result
}

/// A stable fingerprint of a spans generation (order-sensitive FNV-1a over each
/// span's `start`, `end`, and highlight kind). Folded into the per-line syntax
/// cache key so a colored payload can never be reused across span generations.
fn spans_fingerprint(spans: &[HighlightSpan]) -> u64 {
    let mix = |v: u64, h: &mut u64| {
        *h ^= v;
        *h = h.wrapping_mul(0x100000001b3);
    };
    let mut h: u64 = 0xcbf29ce484222325;
    mix(spans.len() as u64, &mut h);
    for s in spans {
        mix(s.start as u64, &mut h);
        mix(s.end as u64, &mut h);
        mix(s.highlight as u64, &mut h);
    }
    h
}
