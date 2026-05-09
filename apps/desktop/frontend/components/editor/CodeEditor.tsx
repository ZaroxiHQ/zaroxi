/**
 * CodeEditor (hard reset baseline)
 *
 * Hard reset summary:
 * - Removed/disabled all custom syntax presentation layers (DOM overlay, per-line overlay,
 *   DOM patch writes). These layers previously caused doubled/ghosted/washed text.
 * - Sole readable layer: the native <textarea> is now the single authoritative visible text surface.
 * - Syntax highlighting has been temporarily disabled at the presentation layer to ensure a clean baseline.
 * - Follow-up plan: reintroduce syntax via a non-rendering presentation (e.g. overlayed CSS-only decorations or separate gutter/inline annotations)
 *   that never draws glyphs or duplicates the textarea's visible text. Tree-sitter state can remain for future safe reintroduction.
 *
 * Removed/disabled presentation paths in this commit:
 * - DOM overlay innerHTML writes into a contenteditable node (removed).
 * - Per-line absolutely positioned HighlightedLineView rendering (no longer mounted).
 * - Overlay transform sync writes to disabled nodes.
 *
 * Why this fixes ghosting:
 * - Only one composited text layer remains (the textarea): no competing glyph rendering or transform races.
 * - Scrolling and typing operate only against the native control; no requestAnimationFrame DOM patches can desync visuals.
 *
 * (Implementation: this commit removes the overlay JSX and the DOM patching effects that wrote innerHTML,
 *  while preserving non-visual highlight hooks for future work.)
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
import CustomSurface from './CustomSurface';

// Frontend per-document syntax session store.
// Keyed by documentId -> { text, map, version }. This allows highlight snapshots
// to survive editor remounts and be reused across CodeEditor instances.
const syntaxSessionStore = new Map<
  string,
  { text: string; map: Map<number, HighlightLine>; version?: number }
>();

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
  /* visibleStart and visibleCount are accepted for compatibility but ignored
     by this simplified, immediate pipeline. */
  _visibleStart?: number,
  _visibleCount?: number,
) {
  // Authoritative map of lineIndex -> HighlightLine for the current document text.
  const [mapState, setMapState] = useState<Map<number, HighlightLine>>(new Map());

  // Simple per-hook cache to avoid re-requesting identical text within the same session.
  // Still keep the global syntaxSessionStore to allow cross-mount reuse.
  const cacheRef = useRef<Map<string, { text: string; map: Map<number, HighlightLine>; version?: number }>>(
    new Map(),
  );

  // Monotonic request id to detect and discard stale responses.
  const reqIdRef = useRef(0);

  // Seed cache/state immediately from an initialSnapshot when it exactly matches the
  // provided `text`. This ensures the frontend can render the server-provided compact
  // snapshot on first paint without any timers or staged phases.
  useEffect(() => {
    if (!documentId || !initialSnapshot) return;

    const reconstructed = initialSnapshot.lines.map((l) => l.text).join('\n');
    const matchesText =
      reconstructed === text ||
      reconstructed + '\n' === text ||
      (text.endsWith('\n') && reconstructed === text.slice(0, -1));
    if (!matchesText) return;

    if (initialSnapshot.lines.length > 0) {
      const seeded = new Map<number, HighlightLine>();
      for (const l of initialSnapshot.lines) {
        const uid = l.uid ?? `${documentId}:${stableHashString(l.text)}`;
        seeded.set(l.index, { uid, index: l.index, text: l.text, spans: l.spans });
      }
      const textKey = `${documentId}::${stableHashString(text)}`;
      cacheRef.current.set(textKey, { text, map: seeded, version: initialSnapshot.version });
      setMapState(new Map(seeded));
      // Also populate global store for other mounts.
      syntaxSessionStore.set(documentId, { text, map: new Map(seeded), version: initialSnapshot.version });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [documentId, initialSnapshot]);

  // Core: request highlights immediately when documentId/text/enabled/language/theme changes.
  // No timers, no idle callbacks, no debouncing. Each request increments reqId and any
  // later-arriving responses that don't match the latest reqId are discarded.
  useEffect(() => {
    if (!documentId || !enabled) {
      setMapState(new Map());
      return;
    }

    // Fast-path: if a global per-document session store has highlights for this exact text, reuse.
    const global = documentId ? syntaxSessionStore.get(documentId) : undefined;
    if (global && global.text === text) {
      setMapState(new Map(global.map));
      return;
    }

    // Fast-path: local cache for exact text
    const textKey = `${documentId}::${stableHashString(text)}`;
    const cached = cacheRef.current.get(textKey);
    if (cached && cached.text === text) {
      setMapState(new Map(cached.map));
      return;
    }

    // Issue a single immediate request for the full document text.
    reqIdRef.current += 1;
    const thisReq = reqIdRef.current;

    (async () => {
      try {
        const res: HighlightResponse = await bridge.invoke('highlight_text', {
          request: {
            documentId,
            text,
            theme: theme ?? 'dark',
            language: language ?? undefined,
          },
        });

        // Discard stale results.
        if (reqIdRef.current !== thisReq) return;

        // Patch only the returned lines into the existing map non-destructively.
        setMapState((prev) => {
          const updated = new Map(prev);
          for (const l of res.lines) {
            const canonicalUid = `${documentId}:${stableHashString(l.text)}`;
            const existing = updated.get(l.index);

            // Detect identical entry to avoid remounts/flashing.
            let identical = false;
            if (existing && existing.text === l.text) {
              const sa = existing.spans || [];
              const sb = l.spans || [];
              if (sa.length === sb.length) {
                identical = true;
                for (let i = 0; i < sa.length; i++) {
                  const a = sa[i];
                  const b = sb[i];
                  if (!a || !b || a.start !== b.start || a.end !== b.end || a.token_type !== b.token_type || (a.color ?? null) !== (b.color ?? null)) {
                    identical = false;
                    break;
                  }
                }
              }
            }

            if (!identical) {
              updated.set(l.index, { uid: canonicalUid, index: l.index, text: l.text, spans: l.spans });
            }
          }
          return updated;
        });

        // Cache and global-store the incoming snapshot for future fast-paths.
        const incomingMap = new Map<number, HighlightLine>();
        for (const l of res.lines) {
          const canonicalUid = `${documentId}:${stableHashString(l.text)}`;
          incomingMap.set(l.index, { uid: canonicalUid, index: l.index, text: l.text, spans: l.spans });
        }
        cacheRef.current.set(textKey, { text, map: incomingMap, version: res.version });
        syntaxSessionStore.set(documentId, { text, map: new Map(incomingMap), version: res.version });
      } catch {
        // Non-fatal: keep previous snapshot visible.
      }
    })();

    // No timers to clean up.
    // eslint-disable-next-line react-hooks/exhaustive-deps
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

  // Compute overlay lines for visible area early so the highlight hook can
  // request a visible-range snapshot immediately on mount.
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
  // Overlay refs disabled for hard-reset baseline: no overlay DOM writes or transform syncs.
  // const overlayRef = useRef<HTMLDivElement | null>(null);
  // const contentRef = useRef<HTMLDivElement | null>(null);

  const visibleStartLine = Math.max(0, Math.floor(scrollTop / lineHeight) - 3);
  const visibleCount = Math.ceil(((containerHeight || lineHeight) + lineHeight) / lineHeight) * 2;
  const visibleEndLine = Math.min(visibleStartLine + visibleCount, totalLines);

  const highlightsEnabled = !largeFile && !!session.documentId;
  // Use the new stable highlight snapshot hook (revision-aware).
  // We pass the full visible text so the backend can compute stable line snapshots.
  const highlightedMap = useHighlightSnapshot(
    session.documentId ?? null,
    displayText,
    highlightsEnabled,
    theme,
    session.language && session.language !== 'plaintext' ? session.language : undefined,
    session.initialHighlight ?? null,
    visibleStartLine,
    visibleCount,
  );

  // Build visible lines by using backend-provided per-line spans directly.
  // The backend returns spans that are already relative to each line's text.
  // Use those spans as-is (no absolute-offset remapping) and always render
  // full line text (spans + plain gaps). This prevents partial coverage.
  // Build a visible slice of highlight lines from the highlightedMap.
  const overlayHighlighted: HighlightLine[] = useMemo(() => {
    const lines: HighlightLine[] = [];
    for (let idx = visibleStartLine; idx < visibleEndLine; idx++) {
      const start = displayLineStarts[idx] ?? displayText.length;
      const end = displayLineStarts[idx + 1] ?? displayText.length;
      let authoritative = displayText.slice(start, end);
      if (authoritative.endsWith('\n')) authoritative = authoritative.slice(0, -1);

      const backend = highlightedMap.get(idx);
      // Use a canonical UID based purely on documentId + line text hash. This
      // keeps the identity stable between the plain-text rendering and any
      // subsequent highlighted snapshot so we avoid destructive remounts.
      const canonicalUid = `${session.documentId}:${stableHashString(authoritative)}`;

      if (backend && backend.text === authoritative) {
        // Apply backend spans only when the backend snapshot matches the exact
        // authoritative line text. Otherwise keep the line plain until a fresh
        // snapshot arrives for that exact text.
        lines.push({ uid: canonicalUid, index: idx, text: authoritative, spans: backend.spans });
      } else {
        // Plain fallback: no spans, explicit uid is canonical (no index appended).
        lines.push({ uid: canonicalUid, index: idx, text: authoritative, spans: [] });
      }
    }
    return lines;
  }, [highlightedMap, visibleStartLine, visibleEndLine, displayLineStarts, displayText, session.documentId]);

  // Overlay DOM writes disabled for baseline: no-op effect to keep hook signature stable.
  useEffect(() => {
    // Intentionally empty: overlay rendering is removed in this baseline reset.
  }, [overlayHighlighted, lineHeight]);

  // Overlay transform synchronization disabled for baseline.
  useEffect(() => {
    // no-op
  }, [scrollTop]);

  const gutterWidth = largeFile ? 0 : computeGutterWidth(totalLines);
  const effectiveReadOnly = readOnly || largeFile;
  const totalHeight = totalLines * lineHeight;
  const MAX_OVERLAY_HEIGHT = 10_000_000;
  // Overlay availability / readiness disabled for baseline to avoid any gating logic.
  const overlayAvailable = false;
  const overlayReady = false;

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
    // No overlay sync: baseline uses native textarea scrolling only.
  }, []);

  // Editor engine removed: using CustomSurface (custom DOM-based editor).
  useEffect(() => {
    // No-op: the CustomSurface component mounts the editor DOM and handles input.
  }, [value]);

  // Render
  return (
    <div
      ref={containerRef}
      className={cn('flex h-full', className)}
      style={
        {
          ['--editor-foreground' as any]: theme === 'dark' ? '#ffffff' : '#0f172a',
          ['--editor-selection' as any]: theme === 'dark' ? 'rgba(90,120,200,0.25)' : 'rgba(90,120,200,0.18)',
          ['--editor-background' as any]: theme === 'dark' ? '#0b1220' : '#ffffff',
        } as React.CSSProperties
      }
    >
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

        {/* Custom single-surface editor: renders visible lines (token spans) directly
            into the DOM and captures input via a focused hidden textarea for IME/composition.
            This preserves one readable text layer (the rendered DOM) and avoids overlay glyph painting. */}
        <CustomSurface
          value={value}
          onChange={(v: string) => { setValue(v); onChange(v); }}
          lines={overlayHighlighted}
          lineHeight={lineHeight}
          totalHeight={totalHeight}
          className="flex-1"
        />
      </div>

      <style>{`
        .scroll-hidden::-webkit-scrollbar { display: none; }
        .scroll-hidden { -ms-overflow-style: none; scrollbar-width: none; }
      `}</style>
    </div>
  );
}
