import {
  useEffect,
  useLayoutEffect,
  useRef,
  useReducer,
  useCallback,
  useMemo,
} from 'react';
import { cn } from '@/lib/utils';
import { useTabsStore } from '@/features/tabs/store';
import { LineNumberGutter } from './gutter/LineNumberGutter';
import { GUTTER_CONFIG } from './gutter/GutterConfig';
import { computeGutterWidth } from './gutter/GutterLayout';
import { FONT_TOKENS } from '@/lib/theme/font-tokens';
import { bridge } from '@/lib/bridge';

/* ------------------------------------------------------------------ */
/*  Highlight model (unchanged via backend)                            */
/* ------------------------------------------------------------------ */
interface HighlightSpan {
  start: number;
  end: number;
  token_type: string;
  color?: string;
}
interface HighlightLine {
  index: number;
  text: string;
  spans: HighlightSpan[];
}
interface HighlightResponse {
  lines: HighlightLine[];
}

const FULL_LINES_LIMIT = 100_000;

/**
 * Request highlighting for the exact editor text currently displayed.
 *
 * Key properties:
 * - Ensures highlights are always derived from the in-memory document text (single source of truth).
 * - Debounces frequent edits to avoid flooding the backend.
 * - Guards against out-of-order responses using a local request id.
 *
 * Returns an array of HighlightLine (same shape used previously).
 */
function useFullHighlight(
  documentId: string | null,
  text: string,
  enabled: boolean,
  theme?: 'dark' | 'light',
) {
  const [lines, setLines] = useReducer(
    (_prev: HighlightLine[], next: HighlightLine[]) => next,
    [],
  );

  const reqIdRef = useRef(0);
  const debounceRef = useRef<number | null>(null);

  useEffect(() => {
    if (!documentId || !enabled) {
      setLines([]);
      return;
    }

    let cancelled = false;
    // bump request id for this batch
    reqIdRef.current += 1;
    const thisReq = reqIdRef.current;

    const doFetch = async () => {
      // Clear stale highlights immediately so previous document's spans never
      // appear over the new file content.
      setLines([]);
      try {
        // Use the bridge wrapper which normalizes errors
        const res: HighlightResponse = await bridge.invoke('highlight_text', {
          request: {
            documentId,
            text,
            theme: theme ?? 'dark',
          },
        });
        // Only apply if this response matches the latest request id and not cancelled.
        if (!cancelled && reqIdRef.current === thisReq) {
          setLines(res.lines);
        }
      } catch (err) {
        console.warn('full highlight (text) failed:', err);
        if (!cancelled && reqIdRef.current === thisReq) setLines([]);
      }
    };

    // Debounce to avoid making a request on every keystroke.
    // Small delay keeps editor feeling responsive while avoiding high IPC churn.
    if (debounceRef.current) {
      window.clearTimeout(debounceRef.current);
    }
    // Use a short debounce; immediate for large documents could be disabled by caller.
    debounceRef.current = window.setTimeout(doFetch, 120);

    return () => {
      cancelled = true;
      if (debounceRef.current) {
        window.clearTimeout(debounceRef.current);
        debounceRef.current = null;
      }
    };
  }, [documentId, text, enabled, theme]);

  return lines;
}

/* ------------------------------------------------------------------ */
/*  Span merging (removes overlaps, innermost wins)                    */
/* ------------------------------------------------------------------ */
function mergeSpans(spans: HighlightSpan[], lineLen: number): HighlightSpan[] {
  if (spans.length === 0 || lineLen === 0) return [];

  const sorted = [...spans].sort((a, b) => (a.end - a.start) - (b.end - b.start));

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

const MAX_LINE_LEN = 5_000;

function renderSpans(spans: HighlightSpan[], lineText: string) {
  if (spans.length === 0 || lineText.length > MAX_LINE_LEN) {
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
    segments.push(
      <span key={key} style={sp.color ? { color: sp.color } : undefined}>
        {lineText.slice(sp.start, sp.end)}
      </span>,
    );
    last = sp.end;
  }
  if (last < lineText.length) {
    segments.push(lineText.slice(last));
  }
  return segments;
}

/* ------------------------------------------------------------------ */
/*  Viewport / helpers                                                */
/* ------------------------------------------------------------------ */
interface CodeEditorProps {
  initialValue: string;
  onChange: (value: string) => void;
  filePath?: string;
  language?: string;
  readOnly?: boolean;
  className?: string;
  contentTruncated?: boolean;
  theme?: 'dark' | 'light';
}

const TRUNCATE_CHARS = 50_000;

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
/*  Editor state (value, scroll, cursor) kept per open file              */
/* ------------------------------------------------------------------ */
interface EditorState {
  /** The file content as edited locally. */
  value: string;
  /** Vertical scroll position of the textarea. */
  scrollTop: number;
  /** Horizontal scroll position of the textarea. */
  scrollLeft: number;
  /** 1‑based line number of the (primary) cursor. */
  cursorLine: number;
}

/**
 * CodeEditor – a tab‑isolated editor with syntax highlighting.
 *
 * Every open file gets its own EditorState stored in a Map keyed by `filePath`.
 * When the active file changes we simply switch to the existing state (or
 * initialise a fresh one).  No state is leaked between documents, and unsaved
 * local edits survive tab switches.
 */
export function CodeEditor({
  initialValue,
  onChange,
  filePath,
  readOnly = false,
  className,
  contentTruncated,
  theme = 'dark',
}: CodeEditorProps) {
  /* –– editor‑states map (persists across re‑renders) –– */
  const editorStates = useRef<Map<string, EditorState>>(new Map());
  // We force a re‑render whenever we mutate the map so React picks up the new data.
  const [, forceUpdate] = useReducer((x: number) => x + 1, 0);

  /* derive a stable key for the *active* document */
  const activeFilePath = filePath ?? '__no_file__';

  /* –– initialise state for a file that is opened for the first time –– */
  if (!editorStates.current.has(activeFilePath)) {
    editorStates.current.set(activeFilePath, {
      value: initialValue,
      scrollTop: 0,
      scrollLeft: 0,
      cursorLine: 1,
    });
  }

  /* read the *current* document’s state (always in‑sync with the map) */
  const activeState = editorStates.current.get(activeFilePath)!;

  /* –– refs –– */
  const containerRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const highlightOuterRef = useRef<HTMLDivElement>(null);

  /* –– huge‑file guard –– */
  const largeFile = contentTruncated ?? (initialValue.length >= TRUNCATE_CHARS);

  /* –– container height from ResizeObserver –– */
  const [containerHeight, setContainerHeight] = useReducer(
    (_prev: number, next: number) => next,
    0,
  );

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setContainerHeight(entry.contentRect.height);
      }
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  /* –– line metrics (always derived from the active document’s value) –– */
  const lineHeight = GUTTER_CONFIG.LINE_HEIGHT;
  const lineStarts = useMemo(() => computeLineStarts(activeState.value), [activeState.value]);
  const totalLines = lineStarts.length;

  /* –– syntax highlight model (per‑display document) –– */
  const highlightsEnabled = !largeFile && !!activeFilePath;
  const allHighlighted = useFullHighlight(
    activeFilePath,
    activeState.value,
    highlightsEnabled,
    theme,
  );

  /* –– viewport (visible lines) –– */
  const visibleStartLine = Math.floor(activeState.scrollTop / lineHeight);
  const visibleCount =
    Math.ceil(((containerHeight || lineHeight) + lineHeight) / lineHeight) * 2;
  const visibleEndLine = Math.min(visibleStartLine + visibleCount, totalLines);

  const visibleHighlighted = useMemo(
    () =>
      allHighlighted.filter(
        (l) => l.index >= visibleStartLine && l.index < visibleEndLine,
      ),
    [allHighlighted, visibleStartLine, visibleEndLine],
  );

  /* –– gutter –– */
  const gutterWidth = largeFile ? 0 : computeGutterWidth(totalLines);
  const effectiveReadOnly = readOnly || largeFile;

  /* –– synchronize textarea native scroll position when the active file changes –– */
  useLayoutEffect(() => {
    const ta = textareaRef.current;
    if (!ta) return;
    ta.scrollTop = activeState.scrollTop;
    ta.scrollLeft = activeState.scrollLeft;
  }, [activeFilePath]);

  /* –– scroll event –– */
  const handleTextareaScroll = useCallback(
    (e: React.UIEvent<HTMLTextAreaElement>) => {
      if (!e.currentTarget) return;
      const sTop = e.currentTarget.scrollTop;
      const sLeft = e.currentTarget.scrollLeft;

      // mutate the map and schedule a re‑render so the overlay / gutter stay in sync
      const current = editorStates.current.get(activeFilePath)!;
      current.scrollTop = sTop;
      current.scrollLeft = sLeft;
      forceUpdate();
    },
    [activeFilePath],
  );

  /* –– cursor tracking –– */
  const handleSelect = useCallback(() => {
    const ta = textareaRef.current;
    if (!ta) return;
    const pos = ta.selectionStart;
    const before = activeState.value.slice(0, pos).match(/\n/g);
    const line = before ? before.length + 1 : 1;
    const st = editorStates.current.get(activeFilePath)!;
    st.cursorLine = line;
    forceUpdate();
  }, [activeFilePath, activeState.value]);

  /* –– edit handling –– */
  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      if (readOnly) return;
      const newVal = e.target.value;

      const st = editorStates.current.get(activeFilePath)!;
      st.value = newVal;
      forceUpdate();

      onChange(newVal);
      const pos = e.target.selectionStart;
      const before = newVal.slice(0, pos).match(/\n/g);
      st.cursorLine = before ? before.length + 1 : 1;

      // mark dirty in the tabs store once we leave the handler
      useTabsStore.getState().markDirty(activeFilePath);
    },
    [onChange, readOnly, activeFilePath],
  );

  /* –– render –– */
  return (
    <div ref={containerRef} className={cn('flex h-full', className)}>
      {/* gutter */}
      {!largeFile && (
        <div
          className="shrink-0 relative overflow-hidden"
          style={{ width: gutterWidth }}
        >
          <LineNumberGutter
            lineCount={totalLines}
            cursorLine={activeState.cursorLine}
            lineHeight={lineHeight}
            scrollTop={activeState.scrollTop}
            containerHeight={containerHeight}
          />
        </div>
      )}

      {/* scrollable text area */}
      <div className="flex-1 flex flex-col overflow-hidden relative">
        {largeFile && (
          <div className="text-muted-foreground text-xs p-1 bg-muted/80 shrink-0">
            File &gt; 5 MB – read‑only preview (first 50 000 characters shown)
          </div>
        )}

        {/* highlight overlay */}
        {highlightsEnabled && (
          <div
            ref={highlightOuterRef}
            aria-hidden="true"
            tabIndex={-1}
            onMouseDown={() => {
              // Defensive: focus the real textarea if any event reaches the overlay.
              // Do NOT preventDefault() here — letting the underlying textarea handle
              // native pointer behavior ensures IME/composition and focus work reliably.
              textareaRef.current?.focus();
            }}
            className="absolute inset-0 overflow-hidden pointer-events-none select-none text-editor-foreground"
            style={{
              lineHeight: `${lineHeight}px`,
              fontFamily: FONT_TOKENS.editor,
              fontSize: '0.875rem',
              whiteSpace: 'pre',
              overflowWrap: 'normal',
              // Defensive: ensure overlay never receives pointer events and is visually behind the textarea.
              pointerEvents: 'none',
              zIndex: 0,
            }}
          >
            <div
              style={{
                position: 'relative',
                height: totalLines * lineHeight,
                width: '100%',
                pointerEvents: 'none',
                boxSizing: 'border-box',
              }}
            >
              <div
                style={{
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  // Use exact pixel scrollTop for vertical sync and scrollLeft for horizontal sync.
                  transform: `translate3d(${-activeState.scrollLeft}px, ${-activeState.scrollTop}px, 0px)`,
                  whiteSpace: 'pre',
                  width: '100%',
                  height: totalLines * lineHeight,
                  pointerEvents: 'none',
                  boxSizing: 'border-box',
                }}
              >
                {visibleHighlighted.map((hl) => (
                  <div
                    key={hl.index}
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
                    {renderSpans(hl.spans, hl.text)}
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {/* editable textarea */}
        <textarea
          key={activeFilePath}
          ref={textareaRef}
          tabIndex={0}
          className="flex-1 resize-none outline-none bg-transparent font-mono text-sm p-0 relative z-20 scroll-hidden"
          style={{
            lineHeight: `${lineHeight}px`,
            fontFamily: FONT_TOKENS.editor,
            fontSize: '0.875rem',
            whiteSpace: 'pre',
            overflowWrap: 'normal',
            overflowX: 'auto',
            overflowY: 'auto',
            pointerEvents: 'auto',
            // Hide the underlying textarea text while highlighting is enabled, but
            // preserve IME/composition and caret by also setting the WebKit text-fill
            // when required. This avoids composition/caret failures on some platforms.
            color: highlightsEnabled ? 'transparent' : undefined,
            WebkitTextFillColor: highlightsEnabled ? 'transparent' : undefined,
            caretColor: highlightsEnabled
              ? 'var(--editor-cursor-color, #E2E8F0)'
              : effectiveReadOnly
                ? 'transparent'
                : undefined,
          }}
          value={activeState.value}
          readOnly={effectiveReadOnly}
          onChange={handleChange}
          onScroll={handleTextareaScroll}
          onSelect={handleSelect}
          onClick={() => textareaRef.current?.focus()}
          onMouseDown={() => {
            // ensure clicks always focus the underlying textarea; defensive for mobile and composed events.
            textareaRef.current?.focus();
          }}
          spellCheck={false}
          autoComplete="off"
          autoCorrect="off"
        />
      </div>

      {/* hide scrollbar chrome */}
      <style>{`
        .scroll-hidden::-webkit-scrollbar { display: none; }
        .scroll-hidden {
          -ms-overflow-style: none;
          scrollbar-width: none;
        }
      `}</style>
    </div>
  );
}
