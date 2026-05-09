/**
 * CodeEditor (simplified and deterministic)
 *
 * Key changes:
 * - Consumes a single authoritative `session` prop.
 * - Maintains a controlled textarea value seeded from session.text on session identity/load changes.
 * - Typing updates local value and notifies container via onChange immediately.
 * - Highlight overlay is always derived from the textarea's visible value.
 * - No hidden adoption or contentRef games.
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
/*  useFullHighlight (keeps behaviour but accepts explicit session id) */
/*  We keep this mostly unchanged and reuse it to drive overlay highlights.
*/
function useFullHighlight(
  documentId: string | null,
  text: string,
  enabled: boolean,
  theme?: 'dark' | 'light',
  language?: string,
  initialHighlight?: { lines: HighlightLine[]; version?: number },
) {
  const [mapState, setMapState] = React.useState<Map<number, HighlightLine>>(new Map());
  const cacheRef = useRef<Map<string, { text: string; map: Map<number, HighlightLine>; version?: number }>>(new Map());
  const reqIdRef = useRef(0);
  const debounceRef = useRef<number | null>(null);
  const prevDocRef = useRef<string | null>(null);

  useEffect(() => {
    if (!documentId || !enabled) {
      setMapState(new Map());
      prevDocRef.current = documentId;
      return;
    }

    const cached = cacheRef.current.get(documentId);
    if (cached && cached.text === text) {
      setMapState(new Map(cached.map));
      prevDocRef.current = documentId;
      return;
    }

    const isDocSwitch = prevDocRef.current !== documentId;
    prevDocRef.current = documentId;

    reqIdRef.current += 1;
    const thisReq = reqIdRef.current;
    let cancelled = false;

    // Produce stable per-line UIDs that survive unrelated edits.
    const applyResultIfCurrent = (resLines: { index: number; text: string; spans: HighlightSpan[] }[], resVersion?: number) => {
      if (cancelled || reqIdRef.current !== thisReq) return false;

      const newMap = new Map<number, HighlightLine>();
      for (const l of resLines) {
        // Use line text hash as the primary stability key. This avoids remount storms when
        // other lines are inserted/deleted and the numeric version changes.
        const uid = `${documentId}:${stableHashString(l.text)}:${l.index}`;
        newMap.set(l.index, { uid, index: l.index, text: l.text, spans: l.spans });
      }

      cacheRef.current.set(documentId, { text, map: newMap, version: resVersion });
      setMapState(newMap);
      return true;
    };

    const fetchExact = async () => {
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
        const normalized = res.lines.map((l) => ({ index: l.index, text: l.text, spans: l.spans }));
        applyResultIfCurrent(normalized, res.version);
      } catch {
        // ignore
      }
    };

    if (debounceRef.current) {
      window.clearTimeout(debounceRef.current);
      debounceRef.current = null;
    }
    // Increase the minimum debounce so we don't churn highlights on every keystroke.
    debounceRef.current = window.setTimeout(() => {
      void fetchExact();
    }, Math.max(80, Math.min(200, Math.floor(text.length / 300))));

    return () => {
      cancelled = true;
      if (debounceRef.current) {
        window.clearTimeout(debounceRef.current);
        debounceRef.current = null;
      }
    };
  }, [documentId, text, enabled, theme, language, initialHighlight]);

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
  const highlightedMap = useFullHighlight(
    session.documentId ?? null,
    displayText,
    highlightsEnabled,
    theme,
    session.language && session.language !== 'plaintext' ? session.language : undefined,
    session.initialHighlight ?? null,
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

  const visibleStartLine = Math.max(0, Math.floor(scrollTop / lineHeight) - 3);
  const visibleCount = Math.ceil(((containerHeight || lineHeight) + lineHeight) / lineHeight) * 2;
  const visibleEndLine = Math.min(visibleStartLine + visibleCount, totalLines);

  const overlayHighlighted = useMemo(() => {
    const lines: HighlightLine[] = [];
    for (let idx = visibleStartLine; idx < visibleEndLine; idx++) {
      const start = displayLineStarts[idx] ?? displayText.length;
      const end = displayLineStarts[idx + 1] ?? displayText.length;
      let authoritative = displayText.slice(start, end);
      if (authoritative.endsWith('\n')) authoritative = authoritative.slice(0, -1);
      const backendHl = highlightedMap.get(idx);
      const usedSpans = backendHl ? backendHl.spans : [];
      const uid = backendHl && backendHl.uid ? backendHl.uid : `${session.documentId}:${stableHashString(authoritative)}:${idx}`;
      lines.push({ uid, index: idx, text: authoritative, spans: usedSpans });
    }
    return lines;
  }, [displayLineStarts, visibleStartLine, visibleEndLine, displayText, highlightedMap, session.documentId]);

  const gutterWidth = largeFile ? 0 : computeGutterWidth(totalLines);
  const effectiveReadOnly = readOnly || largeFile;
  const totalHeight = totalLines * lineHeight;
  const MAX_OVERLAY_HEIGHT = 10_000_000;
  const overlayEnabled = highlightsEnabled && highlightedMap.size > 0 && totalHeight > 0 && totalHeight <= MAX_OVERLAY_HEIGHT;

  // Handlers
  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      if (effectiveReadOnly) return;
      const v = e.target.value;
      setValue(v);
      onChange(v);
    },
    [onChange, effectiveReadOnly],
  );

  const handleScroll = useCallback((e: React.UIEvent<HTMLTextAreaElement>) => {
    const top = e.currentTarget.scrollTop;
    scrollTopRef.current = top;
    setScrollTop(top);
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

        {overlayEnabled && (
          <div
            aria-hidden="true"
            tabIndex={-1}
            onMouseDown={() => { textareaRef.current?.focus(); }}
            className="absolute inset-0 overflow-hidden pointer-events-none select-none text-editor-foreground"
            style={{
              lineHeight: `${lineHeight}px`,
              fontFamily: FONT_TOKENS.editor,
              fontSize: '0.875rem',
              whiteSpace: 'pre',
              overflowWrap: 'normal',
              pointerEvents: 'none',
              zIndex: 30,
            }}
          >
            <div style={{ position: 'relative', height: totalHeight, width: '100%', pointerEvents: 'none', boxSizing: 'border-box' }}>
              <div style={{
                position: 'absolute',
                top: 0,
                left: 0,
                transform: `translate3d(${-0}px, ${-scrollTop}px, 0px)`,
                whiteSpace: 'pre',
                width: '100%',
                height: totalHeight,
                pointerEvents: 'none',
                boxSizing: 'border-box',
              }}>
                {overlayHighlighted.map((hl) => (<HighlightedLineView key={hl.uid} hl={hl} lineHeight={lineHeight} />))}
              </div>
            </div>
          </div>
        )}

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
            color: overlayEnabled ? 'transparent' : undefined,
            WebkitTextFillColor: overlayEnabled ? 'transparent' : undefined,
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
