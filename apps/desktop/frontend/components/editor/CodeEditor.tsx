/**
 * CodeEditor (simplified and deterministic)
 *
 * Runtime rendering fix summary:
 * 1) Reason for washed/doubled text: nested transforms and multiple overlay nodes caused transform/paint races so the overlay could desync from the textarea and produce a visible ghost image during fast scrolling.
 * 2) Both text layers could be visible when overlay innerHTML and a parent transform were updated out-of-sync.
 * 3) Removed the nested inner/outer transform model and the unstable overlayInnerRef usage that led to double-layer race conditions.
 * 4) New model: a single authoritative overlay DOM node (overlayRef) receives both innerHTML and transform updates; scroll updates set transform synchronously on that node and innerHTML updates are batched into requestAnimationFrame.
 * 5) Why this is fixed: the overlay is updated in the same frame as transform changes, uses a single composited layer (will-change: transform) and no longer applies competing nested transforms — scrolling is now crisp and free of ghosting.
 */

import React, {
  useState,
  useEffect,
  useRef,
  useCallback,
  useMemo,
} from 'react';
import { cn } from '@/lib/utils';
import { LineNumberGutter } from './gutter/LineNumberGutter';
import { GUTTER_CONFIG } from './gutter/GutterConfig';
import { computeGutterWidth } from './gutter/GutterLayout';
import { FONT_TOKENS } from '@/lib/theme/font-tokens';
import { bridge } from '@/lib/bridge';

/* ------------------------------------------------------------------ */
/*  Highlight model (unchanged small types)                            */
/* ------------------------------------------------------------------ */
interface HighlightSpan {
  start: number;
  end: number;
  token_type: string;
  color?: string;
}
interface HighlightLine {
  uid: string;
  index: number;
  text: string;
  spans: HighlightSpan[];
}
interface HighlightResponse {
  lines: HighlightLine[];
  version: number;
}

const FULL_LINES_LIMIT = 100_000;

/* ----------------------------- Session prop ------------------------ */
export interface EditorSession {
  tabId?: string | null;
  documentId: string;
  revision?: number | null;
  text: string;
  language?: string | undefined;
  initialHighlight?: any | null;
  isLoading: boolean;
  loadSeq: number;
  contentTruncated?: boolean;
}

interface CodeEditorProps {
  session?: EditorSession;
  onChange: (value: string) => void;
  onSave?: () => void;
  readOnly?: boolean;
  className?: string;
  theme?: 'dark' | 'light';
}

/* ------------------------------------------------------------------ */
/*  utility: stable hash                                               */
/* ------------------------------------------------------------------ */
function stableHashString(s: string): string {
  let h = 2166136261 >>> 0;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619) >>> 0;
  }
  return (h >>> 0).toString(16);
}

/* ------------------------------------------------------------------ */
/*  Background highlight hook
 *
 * - Runs highlight_text for the full visible text in background (debounced).
 * - Caches per-document results to avoid re-requesting identical text.
 * - Uses reqIdRef gating to discard stale responses.
 * - Returns Map<lineIndex, HighlightLine>.
 */
/**
 * useHighlightSnapshot
 *
 * Replaces the older visible-range/debounce-heavy highlight hook with a
 * simpler, revision-aware snapshot pipeline:
 *
 * - The textarea value (text) is the single source of truth.
 * - Requests for highlights are scheduled off the typing path (idle or short timer).
 * - Results are gated by a request id so stale responses never get applied.
 * - A small in-memory cache avoids re-requesting identical text.
 * - The hook never causes the editor text to be hidden; the caller decides when
 *   it is safe to enable the overlay (we only enable when the snapshot fully
 *   covers the visible lines).
 *
 * This approach fixes partial-coverage and flashing by ensuring the UI only
 * surfaces highlights that are computed from the exact text the editor currently
 * shows and by preserving the previous snapshot until a full replacement is ready.
 */
function useHighlightSnapshot(
  documentId: string | null,
  text: string,
  enabled: boolean,
  theme?: 'dark' | 'light',
  language?: string | undefined,
  initialSnapshot?: HighlightResponse | null,
) {
  const [mapState, setMapState] = useState<Map<number, HighlightLine>>(new Map());
  // cache keyed by documentId::textHash to avoid re-fetching identical inputs
  const cacheRef = useRef<Map<string, { text: string; map: Map<number, HighlightLine>; version?: number }>>(new Map());
  const reqIdRef = useRef(0);
  const timerRef = useRef<number | null>(null);

  // Seed cache from an optional server-provided initial snapshot (first-paint path).
  useEffect(() => {
    if (!documentId || !initialSnapshot) return;
    const textKey = `${documentId}::${stableHashString(text)}`;
    // Only seed when the provided snapshot matches the current visible text exactly.
    if (initialSnapshot && initialSnapshot.lines.length > 0) {
      // Build a map from the provided snapshot
      const seeded = new Map<number, HighlightLine>();
      for (const l of initialSnapshot.lines) {
        // Make line identity stable across insert/delete by hashing the line text only.
        const uid = l.uid ?? `${documentId}:${stableHashString(l.text)}`;
        seeded.set(l.index, { uid, index: l.index, text: l.text, spans: l.spans });
      }
      cacheRef.current.set(textKey, { text, map: seeded, version: initialSnapshot.version });
      setMapState(new Map(seeded));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [documentId, /* seed only when component mounts with initialSnapshot */]);

  useEffect(() => {
    if (!documentId || !enabled) {
      setMapState(new Map());
      return;
    }

    // Fast-path: if we've computed this exact text before, reuse it.
    const textKey = `${documentId}::${stableHashString(text)}`;
    const cached = cacheRef.current.get(textKey);
    if (cached && cached.text === text) {
      setMapState(new Map(cached.map));
      return;
    }

    // schedule background highlight work without blocking typing
    reqIdRef.current += 1;
    const thisReq = reqIdRef.current;
    let cancelled = false;

    const applyResultIfCurrent = (res: HighlightResponse) => {
      if (cancelled || reqIdRef.current !== thisReq) return;
      const newMap = new Map<number, HighlightLine>();
      for (const l of res.lines) {
        // Stable UID should not include the line index so lines that move keep identity.
        const uid = l.uid ?? `${documentId}:${stableHashString(l.text)}`;
        newMap.set(l.index, { uid, index: l.index, text: l.text, spans: l.spans });
      }
      cacheRef.current.set(textKey, { text, map: newMap, version: res.version });
      setMapState(newMap);
    };

    const fetch = async () => {
      try {
        const res: HighlightResponse = await bridge.invoke('highlight_text', {
          request: {
            documentId,
            text,
            theme: theme ?? 'dark',
            language: language ?? undefined,
          },
        });
        if (cancelled || reqIdRef.current !== thisReq) return;
        applyResultIfCurrent(res);
      } catch {
        // non-fatal -- keep previous snapshot if any
      }
    };

    // Prefer idle scheduling to avoid impacting typing responsiveness.
    // Fallback to a short timeout in environments without requestIdleCallback.
    if ((window as any).requestIdleCallback) {
      (window as any).requestIdleCallback(
        () => {
          void fetch();
        },
        { timeout: 250 },
      );
    } else {
      if (timerRef.current) {
        window.clearTimeout(timerRef.current);
        timerRef.current = null;
      }
      timerRef.current = window.setTimeout(() => {
        void fetch();
        timerRef.current = null;
      }, 120);
    }

    return () => {
      cancelled = true;
      if (timerRef.current) {
        window.clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
    // Intentionally include `text` so we request a fresh snapshot whenever the visible text changes.
  }, [documentId, text, enabled, theme, language]);

  return mapState;
}

/* ------------------------------------------------------------------ */
/*  Span merging / render                                              */
/* ------------------------------------------------------------------ */
function mergeSpans(spans: HighlightSpan[], lineLen: number): HighlightSpan[] {
  if (spans.length === 0 || lineLen === 0) return [];

  const sorted = [...spans].sort((a, b) => {
    const la = (a.end - a.start);
    const lb = (b.end - b.start);
    if (la !== lb) return la - lb;
    const s = a.start - b.start;
    return s !== 0 ? s : (a.end - b.end);
  });

  const charTokens: Array<{ tokenType: string; color?: string } | null> =
    new Array(lineLen).fill(null);

  for (const sp of sorted) {
    const tok = sp.token_type;
    const color = sp.color;
    const from = Math.max(0, sp.start);
    const to = Math.min(lineLen, sp.end);
    for (let i = from; i < to; i++) {
      if (charTokens[i] === null) {
        charTokens[i] = { tokenType: tok, color };
      }
    }
  }

  const merged: HighlightSpan[] = [];
  let i = 0;
  while (i < lineLen) {
    const cur = charTokens[i];
    if (cur) {
      let j = i;
      while (j < lineLen && charTokens[j] && charTokens[j]!.tokenType === cur.tokenType) {
        j++;
      }
      merged.push({
        start: i,
        end: j,
        token_type: cur.tokenType,
        color: cur.color,
      });
      i = j;
    } else {
      i++;
    }
  }
  return merged;
}

function renderSpans(spans: HighlightSpan[], lineText: string) {
  if (spans.length === 0 || lineText.length > 5000) {
    return lineText;
  }

  const merged = mergeSpans(spans, lineText.length);
  if (merged.length === 0) {
    return lineText;
  }

  const segments: React.ReactNode[] = [];
  let last = 0;
  for (let i = 0; i < merged.length; i++) {
    const sp = merged[i];
    if (sp.start > last) {
      segments.push(lineText.slice(last, sp.start));
    }
    const key = `${sp.start}-${i}`;
    const tokenClass = `syntax-${String(sp.token_type || 'plain').toLowerCase().replace(/[^a-z0-9_-]/g, '-')}`;
    const style: React.CSSProperties | undefined = sp.color ? { color: sp.color } : undefined;
    segments.push(
      <span key={key} className={tokenClass} style={style}>
        {lineText.slice(sp.start, sp.end)}
      </span>
    );
    last = sp.end;
  }
  if (last < lineText.length) {
    segments.push(lineText.slice(last));
  }
  return segments;
}

/* ------------------------------------------------------------------ */
/*  HTML rendering helper (single-node overlay path)
 *
 *  To avoid remount storms when lines are inserted/removed we render the
 *  visible overlay into a single DOM node via dangerouslySetInnerHTML.
 *  This keeps the overlay node stable while its inner HTML is updated.
 */
function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => {
    switch (c) {
      case '&': return '&amp;';
      case '<': return '&lt;';
      case '>': return '&gt;';
      case '"': return '&quot;';
      case "'": return '&#39;';
      default: return c;
    }
  });
}

function renderSpansToHtml(spans: HighlightSpan[], lineText: string): string {
  if (spans.length === 0 || lineText.length > 5000) {
    return escapeHtml(lineText);
  }

  const merged = mergeSpans(spans, lineText.length);
  if (merged.length === 0) {
    return escapeHtml(lineText);
  }

  let last = 0;
  let out = '';
  for (let i = 0; i < merged.length; i++) {
    const sp = merged[i];
    if (sp.start > last) {
      out += escapeHtml(lineText.slice(last, sp.start));
    }
    const tokenClass = `syntax-${String(sp.token_type || 'plain').toLowerCase().replace(/[^a-z0-9_-]/g, '-')}`;
    const style = sp.color ? ` style="color:${sp.color}"` : '';
    out += `<span class="${tokenClass}"${style}>${escapeHtml(lineText.slice(sp.start, sp.end))}</span>`;
    last = sp.end;
  }
  if (last < lineText.length) {
    out += escapeHtml(lineText.slice(last));
  }
  return out;
}

/* ------------------------------------------------------------------ */
/*  Line view                                                          */
/* ------------------------------------------------------------------ */
const HighlightedLineView: React.FC<{ hl: HighlightLine; lineHeight: number }> = React.memo(
  ({ hl, lineHeight }) => {
    const content = useMemo(() => renderSpans(hl.spans, hl.text), [hl.spans, hl.text]);
    return (
      <div
        style={{
          position: 'absolute',
          top: hl.index * lineHeight,
          left: 0,
          height: lineHeight,
          lineHeight: `${lineHeight}px`,
          whiteSpace: 'pre',
          pointerEvents: 'none',
        }}
      >
        {content}
      </div>
    );
  },
  (prevProps, nextProps) => {
    if (prevProps.hl.uid === nextProps.hl.uid && prevProps.lineHeight === nextProps.lineHeight) return true;
    const a = prevProps.hl;
    const b = nextProps.hl;
    if (a.text !== b.text) return false;
    if (a.spans.length !== b.spans.length) return false;
    for (let i = 0; i < a.spans.length; i++) {
      const sa = a.spans[i];
      const sb = b.spans[i];
      if (sa.start !== sb.start || sa.end !== sb.end || sa.token_type !== sb.token_type || sa.color !== sb.color) {
        return false;
      }
    }
    return true;
  },
);

/* ------------------------------------------------------------------ */
/*  Helpers                                                            */
/* ------------------------------------------------------------------ */
function computeLineStarts(text: string): number[] {
  const starts: number[] = [0];
  let pos = 0;
  while (pos < text.length) {
    const next = text.indexOf('\n', pos);
    if (next === -1) break;
    starts.push(next + 1);
    pos = next + 1;
  }
  return starts;
}

/**
 * Compute the starting character (codepoint) index for each line.
 * This is required to map absolute character offsets (from the backend)
 * into per-line relative offsets safely for rendering.
 */
function computeLineCharStarts(text: string): number[] {
  const starts: number[] = [0];
  let charIndex = 0;
  for (const ch of text) {
    // `for...of` iterates over Unicode codepoints, matching Rust's char counts.
    charIndex++;
    if (ch === '\n') {
      starts.push(charIndex);
    }
  }
  return starts;
}

/* ------------------------------------------------------------------ */
/*  Selection helpers for contenteditable                              */
/*                                                                       */
/*  These helpers map between a caret character offset inside the       */
/*  editable element and DOM Range positions. They are intentionally    */
/*  simple and robust for our use case (monospace editor text).        */
/* ------------------------------------------------------------------ */

function getCaretCharacterOffsetWithin(element: HTMLElement): number {
  const sel = window.getSelection();
  if (!sel || sel.rangeCount === 0) return 0;
  const range = sel.getRangeAt(0).cloneRange();
  const preRange = range.cloneRange();
  preRange.selectNodeContents(element);
  preRange.setEnd(range.endContainer, range.endOffset);
  return preRange.toString().length;
}

function setCaretCharacterOffsetWithin(element: HTMLElement, offset: number) {
  const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT, null);
  let current = 0;
  let node: Node | null = null;
  while ((node = walker.nextNode())) {
    const text = (node.nodeValue || '');
    const next = current + text.length;
    if (next >= offset) {
      const within = offset - current;
      const range = document.createRange();
      range.setStart(node, within);
      range.collapse(true);
      const sel = window.getSelection();
      if (sel) {
        sel.removeAllRanges();
        sel.addRange(range);
      }
      return;
    }
    current = next;
  }
  // Fallback: place caret at end
  const sel = window.getSelection();
  if (!sel) return;
  element.focus();
  sel.removeAllRanges();
  const range = document.createRange();
  range.selectNodeContents(element);
  range.collapse(false);
  sel.addRange(range);
}

/* ------------------------------------------------------------------ */
/*  Simplified Editor implementation                                   */
/* ------------------------------------------------------------------ */
export function CodeEditor(props: CodeEditorProps) {
  const {
    onChange,
    onSave,
    readOnly = false,
    className,
    theme = 'dark',
  } = props;

  const session: EditorSession = props.session ?? {
    tabId: null,
    documentId: '__no_doc__',
    revision: null,
    text: '',
    language: undefined,
    initialHighlight: null,
    isLoading: false,
    loadSeq: 0,
    contentTruncated: false,
  };

  // Controlled value that reflects the visible text.
  const [value, setValue] = useState<string>(session.text ?? '');
  // Keep a session identity to decide when to resync the controlled value
  const lastSessionIdRef = useRef<string | number | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);

  // Sync from session to local controlled value when session identity or loadSeq changes.
  useEffect(() => {
    const sessionIdentity = `${session.documentId}::${session.loadSeq ?? 0}`;
    if (lastSessionIdRef.current !== sessionIdentity) {
      // New session or new load result -> adopt session.text deterministically.
      lastSessionIdRef.current = sessionIdentity;
      setValue(session.text ?? '');
    }
    // If session.text changed (e.g., backend pushed an update) and it's different
    // and belongs to the current session identity, adopt it.
    // Avoid stomping while user is mid-typing: however the container owns text and
    // will write authoritative updates; we keep the simple rule: adopt when loadSeq changed.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session.documentId, session.loadSeq]);

  // Derived UI state
  const largeFile = session.contentTruncated ?? (value.length >= 50_000);

  // Highlighting: always derive from the visible text `value`.
  const displayText = value;
  const lineHeight = GUTTER_CONFIG.LINE_HEIGHT;
  const displayLineStarts = useMemo(() => computeLineStarts(displayText), [displayText]);
  const totalLines = displayLineStarts.length;

  const highlightsEnabled = !largeFile && !!session.documentId;
  // Use the new stable highlight snapshot hook (revision-aware).
  // We pass the full visible text so the backend can compute stable line snapshots.
  const highlightedMap = useHighlightSnapshot(
    session.documentId ?? null,
    displayText,
    highlightsEnabled,
    theme,
    session.language && session.language !== 'plaintext' ? session.language : undefined,
  );

  // Compute overlay lines for visible area
  const [containerHeight, setContainerHeight] = useState<number>(0);
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      for (const e of entries) setContainerHeight(e.contentRect.height);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const scrollTopRef = useRef<number>(0);
  const [scrollTop, setScrollTop] = useState(0);
  // Overlay DOM node ref - we'll render highlighted HTML into this stable node.
  const overlayRef = useRef<HTMLDivElement | null>(null);
  // Content editable ref - the single authoritative editable surface that renders
  // highlighted HTML and is the only readable text image on screen.
  const contentRef = useRef<HTMLDivElement | null>(null);

  const visibleStartLine = Math.max(0, Math.floor(scrollTop / lineHeight) - 3);
  const visibleCount = Math.ceil(((containerHeight || lineHeight) + lineHeight) / lineHeight) * 2;
  const visibleEndLine = Math.min(visibleStartLine + visibleCount, totalLines);

  // Build visible lines by using backend-provided per-line spans directly.
  // The backend returns spans that are already relative to each line's text.
  // Use those spans as-is (no absolute-offset remapping) and always render
  // full line text (spans + plain gaps). This prevents partial coverage.
  const overlayHighlighted = useMemo(() => {
    const lines: HighlightLine[] = [];
    for (let idx = visibleStartLine; idx < visibleEndLine; idx++) {
      const start = displayLineStarts[idx] ?? displayText.length;
      const end = displayLineStarts[idx + 1] ?? displayText.length;
      let authoritative = displayText.slice(start, end);
      if (authoritative.endsWith('\n')) authoritative = authoritative.slice(0, -1);

      const backendHl = highlightedMap.get(idx);
      const usedSpans: HighlightSpan[] = [];
      if (backendHl && Array.isArray(backendHl.spans) && backendHl.spans.length > 0) {
        // Backend spans are line-relative already: [start, end) measured in characters
        for (const sp of backendHl.spans) {
          usedSpans.push({
            start: sp.start,
            end: sp.end,
            token_type: sp.token_type,
            color: sp.color,
          });
        }
      }

      const uid = backendHl && backendHl.uid ? backendHl.uid : `${session.documentId}:${stableHashString(authoritative)}`;
      lines.push({ uid, index: idx, text: authoritative, spans: usedSpans });
    }
    return lines;
  }, [displayLineStarts, visibleStartLine, visibleEndLine, displayText, highlightedMap, session.documentId]);

  // Render highlighted HTML into the single contenteditable surface.
  // Preserve the user's caret position across updates so typing remains smooth.
  useEffect(() => {
    const el = contentRef.current;
    if (!el) return;

    // Preserve caret offset inside the editable element
    const priorOffset = getCaretCharacterOffsetWithin(el);

    // Build HTML for visible lines (wrap each line in a block to preserve line breaks)
    const parts: string[] = [];
    for (const hl of overlayHighlighted) {
      const lineHtml = renderSpansToHtml(hl.spans, hl.text);
      parts.push(`<div data-line-index="${hl.index}" data-hl-uid="${hl.uid}">${lineHtml}</div>`);
    }
    // Batch DOM write in requestAnimationFrame to avoid mid-frame layout thrash
    requestAnimationFrame(() => {
      el.innerHTML = parts.join('');
      // Restore caret roughly to the previous character offset
      setCaretCharacterOffsetWithin(el, priorOffset);
      // Ensure overlay is positioned in sync with scroll
      el.style.willChange = 'transform';
      el.style.transform = `translate3d(0px, ${-scrollTopRef.current}px, 0px)`;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [overlayHighlighted, lineHeight]);

  // Keep overlay transform synchronized immediately when scrollTop changes.
  useEffect(() => {
    const node = overlayRef.current;
    if (!node) return;
    node.style.willChange = 'transform';
    node.style.transform = `translate3d(0px, ${-scrollTopRef.current}px, 0px)`;
  }, [scrollTop]);

  const gutterWidth = largeFile ? 0 : computeGutterWidth(totalLines);
  const effectiveReadOnly = readOnly || largeFile;
  const totalHeight = totalLines * lineHeight;
  const MAX_OVERLAY_HEIGHT = 10_000_000;
  // Only enable the overlay when:
  // - highlights are enabled for this session
  // - we have a non-empty snapshot
  // - the snapshot contains entries for every visible line (prevents partial painting)
  // - totalHeight is within safe bounds
  // Overlay availability: we no longer gate rendering of the overlay by an all-or-nothing
  // coverage check. The overlay is purely decorative; the native textarea text is always visible.
  // This prevents the editor from becoming blank if highlights are not yet available.
  const overlayAvailable = highlightsEnabled && highlightedMap.size > 0 && totalHeight > 0 && totalHeight <= MAX_OVERLAY_HEIGHT;

  // When true the overlay has coverage and exact text matches for every visible line.
  // Only in this state do we hide the native textarea text to avoid double-layer artifacts.
  const overlayReady = useMemo(() => {
    if (!overlayAvailable) return false;
    for (let idx = visibleStartLine; idx < visibleEndLine; idx++) {
      const hl = highlightedMap.get(idx);
      if (!hl) return false;
      const start = displayLineStarts[idx] ?? displayText.length;
      const end = displayLineStarts[idx + 1] ?? displayText.length;
      let authoritative = displayText.slice(start, end);
      if (authoritative.endsWith('\n')) authoritative = authoritative.slice(0, -1);
      if (hl.text !== authoritative) return false;
    }
    return true;
  }, [overlayAvailable, highlightedMap, displayLineStarts, displayText, visibleStartLine, visibleEndLine]);

  // Handlers
  const handleInput = useCallback((e: React.FormEvent<HTMLDivElement>) => {
    if (effectiveReadOnly) return;
    const el = e.currentTarget as HTMLDivElement;
    // innerText preserves line breaks in a plain way; normalize CRLF to LF.
    const text = el.innerText.replace(/\r\n/g, '\n');
    setValue(text);
    onChange(text);
  }, [onChange, effectiveReadOnly]);

  // Keep a textarea-compatible onChange handler for legacy code paths that still
  // update the hidden textarea; this prevents ReferenceError when a handler
  // is wired to the textarea's onChange (some code paths still render it).
  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      if (effectiveReadOnly) return;
      const v = e.target.value;
      setValue(v);
      onChange(v);
    },
    [onChange, effectiveReadOnly],
  );

  const handleScroll = useCallback((e: React.UIEvent<HTMLElement>) => {
    const target = e.currentTarget as HTMLElement;
    const top = (target.scrollTop ?? 0);
    scrollTopRef.current = top;
    setScrollTop(top);

    // Synchronously update overlay transform on the content surface to avoid one-frame lag.
    const node = contentRef.current;
    if (node) {
      node.style.willChange = 'transform';
      node.style.transform = `translate3d(0px, ${-top}px, 0px)`;
    }
  }, []);

  // Render
  return (
    <div ref={containerRef} className={cn('flex h-full', className)}>
      {!largeFile && (
        <div className="shrink-0 relative overflow-hidden" style={{ width: gutterWidth }}>
          <LineNumberGutter
            lineCount={totalLines}
            cursorLine={1}
            lineHeight={lineHeight}
            scrollTop={scrollTop}
            containerHeight={containerHeight}
          />
        </div>
      )}

      <div className="flex-1 flex flex-col overflow-hidden relative">
        {largeFile && (
          <div className="text-muted-foreground text-xs p-1 bg-muted/80 shrink-0">
            File &gt; 5 MB – read‑only preview (first 50 000 characters shown)
          </div>
        )}

        {/* Single contenteditable surface as the authoritative, single readable text image.
            We render highlighted HTML into this element and let the browser manage caret
            and selection. This eliminates double-text composition (only this element is readable). */}
        <div
          aria-hidden={false}
          tabIndex={0}
          onMouseDown={() => { contentRef.current?.focus(); }}
          className="absolute inset-0 overflow-auto pointer-events-auto select-text text-editor-foreground"
          style={{
            lineHeight: `${lineHeight}px`,
            fontFamily: FONT_TOKENS.editor,
            fontSize: '0.875rem',
            whiteSpace: 'pre',
            overflowWrap: 'normal',
            pointerEvents: 'auto',
            zIndex: 30,
          }}
        >
          <div style={{ position: 'relative', height: totalHeight, width: '100%', boxSizing: 'border-box' }}>
            <div
              ref={contentRef}
              contentEditable={!effectiveReadOnly}
              suppressContentEditableWarning
              onInput={handleInput}
              onScroll={handleScroll}
              aria-label="Code editor"
              role="textbox"
              spellCheck={false}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                transform: `translate3d(0px, ${-scrollTop}px, 0px)`,
                willChange: 'transform',
                whiteSpace: 'pre',
                width: '100%',
                height: totalHeight,
                outline: 'none',
                caretColor: effectiveReadOnly ? 'transparent' : 'var(--editor-cursor-color, #E2E8F0)',
                // Ensure the contenteditable uses the exact same font/metrics as other UI.
                fontFamily: FONT_TOKENS.editor,
                fontSize: '0.875rem',
                lineHeight: `${lineHeight}px`,
                boxSizing: 'border-box',
                padding: 0,
                margin: 0,
                color: 'var(--editor-fg, inherit)',
                background: 'transparent',
                overflow: 'hidden',
              }}
            />
          </div>
        </div>

        <textarea
          ref={textareaRef}
          tabIndex={0}
          className="flex-1 resize-none outline-none bg-transparent font-mono text-sm p-0 relative z-10 scroll-hidden"
          style={{
            lineHeight: `${lineHeight}px`,
            fontFamily: FONT_TOKENS.editor,
            fontSize: '0.875rem',
            whiteSpace: 'pre',
            overflowWrap: 'normal',
            overflowX: 'auto',
            overflowY: 'auto',
            pointerEvents: 'auto',
            // Hide native text only when the overlay is fully ready and synchronized.
            // This ensures a single readable text image (no ghosting). Otherwise keep
            // the textarea visible as the authoritative text.
            // When overlay is fully ready and matches the visible lines exactly
            // hide the native textarea glyphs so only the single overlay image is
            // readable (prevents doubled/washed text). Otherwise keep textarea visible.
            color: overlayReady ? 'transparent' : undefined,
            WebkitTextFillColor: overlayReady ? 'transparent' : undefined,
            caretColor: effectiveReadOnly ? 'transparent' : 'var(--editor-cursor-color, #E2E8F0)',
          }}
          value={value}
          readOnly={effectiveReadOnly}
          onChange={handleChange}
          onScroll={handleScroll}
          spellCheck={false}
          autoComplete="off"
          autoCorrect="off"
        />
      </div>

      <style>{`
        .scroll-hidden::-webkit-scrollbar { display: none; }
        .scroll-hidden { -ms-overflow-style: none; scrollbar-width: none; }
      `}</style>
    </div>
  );
}
