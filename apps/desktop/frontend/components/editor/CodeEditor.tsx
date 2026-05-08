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
  language?: string,
) {
  // Improved, non-flickering Tree-sitter highlight flow with throttled edits:
  // - Keep currently visible highlights until an authoritative backend response arrives.
  // - Use highlight_document on tab switch (server buffer/cache) for speed.
  // - Use highlight_text for edits; debounce minimally for continuous typing,
  //   but allow an immediate fetch when enough time has passed since the last fetch.
  // - Cache authoritative results keyed by exact text so switching back is instant.
  const [lines, setLines] = useReducer(
    (_prev: HighlightLine[], next: HighlightLine[]) => next,
    [],
  );

  const cacheRef = useRef<Map<string, { text: string; lines: HighlightLine[] }>>(new Map());
  const reqIdRef = useRef(0);
  const debounceRef = useRef<number | null>(null);
  const prevDocRef = useRef<string | null>(null);

  // Throttling / sizing parameters
  const SMALL_FILE_THRESHOLD = 1500; // chars
  const MIN_DEBOUNCE_MS = 20;
  const MAX_DEBOUNCE_MS = 120;
  const EDIT_THROTTLE_MS = 300; // if last fetch older than this, fetch immediately on edit

  // Track last fetch timestamps per document so edits don't always wait for debounce.
  const lastFetchRef = useRef<Map<string, number>>(new Map());

  useEffect(() => {
    if (!documentId || !enabled) {
      setLines([]);
      prevDocRef.current = documentId;
      return;
    }

    // If an exact cached result exists, apply it immediately (no network).
    const cached = cacheRef.current.get(documentId);
    if (cached && cached.text === text) {
      setLines(cached.lines);
      prevDocRef.current = documentId;
      return;
    }

    const isDocSwitch = prevDocRef.current !== documentId;
    prevDocRef.current = documentId;

    reqIdRef.current += 1;
    const thisReq = reqIdRef.current;
    let cancelled = false;

    // Helper to record a fetch timestamp
    const recordFetch = () => {
      if (!documentId) return;
      lastFetchRef.current.set(documentId, Date.now());
    };

    // Fetch authoritative highlights for the exact text (used on edits).
    const fetchExact = async () => {
      recordFetch();
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

        // Cache authoritative result and apply only if changed to avoid reflows.
        const prev = cacheRef.current.get(documentId);
        const sameText = prev && prev.text === text;
        const sameLines =
          sameText &&
          prev!.lines.length === (res.lines || []).length &&
          JSON.stringify(prev!.lines) === JSON.stringify(res.lines || []);

        cacheRef.current.set(documentId, { text, lines: res.lines || [] });
        if (!sameLines) {
          setLines(res.lines || []);
        }
      } catch (err) {
        console.warn('highlight_text failed:', err);
        // Keep existing UI; do not clear on error.
      }
    };

    // Try using server-side cached highlights (fast on doc switch when buffer open)
    const fetchDocumentRange = async (): Promise<boolean> => {
      recordFetch();
      try {
        const res: HighlightResponse = await bridge.invoke('highlight_document', {
          request: {
            documentId,
            startLine: 0,
            count: FULL_LINES_LIMIT,
            theme: theme ?? 'dark',
          },
        });
        if (cancelled || reqIdRef.current !== thisReq) return false;

        if (res.lines && res.lines.length > 0) {
          // Cache under the current editor text to allow instant reuse.
          cacheRef.current.set(documentId, { text, lines: res.lines });
          setLines(res.lines);
          return true;
        }
      } catch (err) {
        // highlight_document may not be available for this document; fall back.
        console.debug('highlight_document failed or not present:', err);
      }
      return false;
    };

    // Clear pending debounce
    if (debounceRef.current) {
      window.clearTimeout(debounceRef.current);
      debounceRef.current = null;
    }

    // Strategy: on doc switch try server cached highlights first; otherwise
    // for edits prefer exact text highlights. Never clear the UI while fetching.
    // Optimization: when switching documents, launch both the server-side range
    // highlight and the exact-text highlight concurrently.
    const doWork = async () => {
      if (isDocSwitch) {
        void fetchDocumentRange();
        void fetchExact();
      } else {
        await fetchExact();
      }
    };

    // Decide immediate vs debounced fetch:
    // - Immediate when first-open, doc-switch, or small files.
    // - For edits: immediate if last fetch was older than EDIT_THROTTLE_MS, else short debounce.
    const lastFetch = documentId ? lastFetchRef.current.get(documentId) ?? 0 : 0;
    const now = Date.now();
    const shouldImmediateEditFetch =
      !cached || isDocSwitch || text.length <= SMALL_FILE_THRESHOLD || (now - lastFetch) >= EDIT_THROTTLE_MS;

    if (shouldImmediateEditFetch) {
      void doWork();
    } else {
      // Adaptive debounce for edits on larger files.
      const adaptiveMs = Math.max(
        MIN_DEBOUNCE_MS,
        Math.min(MAX_DEBOUNCE_MS, Math.floor(text.length / 200))
      );
      debounceRef.current = window.setTimeout(() => {
        void doWork();
      }, adaptiveMs);
    }

    return () => {
      cancelled = true;
      if (debounceRef.current) {
        window.clearTimeout(debounceRef.current);
        debounceRef.current = null;
      }
    };
  }, [documentId, text, enabled, theme, language]);

  return lines;
}

/* ------------------------------------------------------------------ */
/*  Span merging (removes overlaps, innermost wins)                    */
/* ------------------------------------------------------------------ */
function mergeSpans(spans: HighlightSpan[], lineLen: number): HighlightSpan[] {
  if (spans.length === 0 || lineLen === 0) return [];

  // Sort spans so that the innermost (shortest) spans get applied first,
  // then by start position. This implements "innermost wins" behavior:
  // short, precise tokens (e.g. identifiers, strings) override larger spans
  // that may cover the same area (e.g. parent expressions or comment spans).
  const sorted = [...spans].sort((a, b) => {
    const la = (a.end - a.start);
    const lb = (b.end - b.start);
    if (la !== lb) return la - lb; // shorter spans first
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
  language,
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

  /**
   * Keep the editor state in sync with incoming `initialValue` for the active file.
   *
   * Rationale:
   * - `initialValue` is populated asynchronously by the container (openFile).
   * - The editor stores per-file state in a ref'd Map and only sets the initial
   *   value once when the entry is created. That means an async load can leave
   *   the map entry with an empty string unless we explicitly adopt the newly
   *   provided content here.
   *
   * Policy:
   * - Only adopt `initialValue` when the tab is not marked dirty (we don't want
   *   to clobber user edits).
   * - Reset caret/scroll for a fresh load.
   */
  useEffect(() => {
    const state = editorStates.current.get(activeFilePath);
    if (!state) return;

    // If incoming content is identical, nothing to do.
    if (state.value === initialValue) return;

    // If the tab is dirty (user edited locally), do not overwrite local edits.
    const tab = useTabsStore.getState().tabs.find((t) => t.id === activeFilePath);
    const isDirty = tab?.isDirty ?? false;
    if (isDirty) {
      return;
    }

    // Adopt the freshly loaded content for this file and reset viewport/caret.
    state.value = initialValue;
    state.cursorLine = 1;
    state.scrollTop = 0;
    state.scrollLeft = 0;
    forceUpdate();
  }, [activeFilePath, initialValue]);

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
    // pass frontend-known language hint (if any) to improve detection.
    // Treat "plaintext" as "unknown" so the backend can derive the language
    // from the file path instead of being forced to PlainText.
    language && language !== 'plaintext' ? language : undefined,
  );

  /* –– viewport (visible lines) –– */
  const visibleStartLine = Math.floor(activeState.scrollTop / lineHeight);
  const visibleCount =
    Math.ceil(((containerHeight || lineHeight) + lineHeight) / lineHeight) * 2;
  const visibleEndLine = Math.min(visibleStartLine + visibleCount, totalLines);

  const visibleHighlighted = useMemo(() => {
    // Build a quick lookup of highlighted lines returned from the backend.
    const map = new Map<number, HighlightLine>();
    for (const l of allHighlighted) {
      map.set(l.index, l);
    }

    const lines: HighlightLine[] = [];
    for (let idx = visibleStartLine; idx < visibleEndLine; idx++) {
      // Derive authoritative line text from the active document (single source of truth).
      const start = lineStarts[idx] ?? activeState.value.length;
      const end = lineStarts[idx + 1] ?? activeState.value.length;
      let authoritative = activeState.value.slice(start, end);
      if (authoritative.endsWith('\n')) authoritative = authoritative.slice(0, -1);

      // If the backend returned a highlighted line for this index, use its spans
      // but ALWAYS render the authoritative text from the editor's buffer. This
      // avoids mismatches where the backend-provided `text` differs from the
      // frontend buffer (causing missing spans or out-of-bounds indices).
      const hl = map.get(idx);
      if (hl) {
        lines.push({
          index: idx,
          text: authoritative,
          spans: hl.spans,
        });
        continue;
      }

      // Fallback: render plain text with no spans.
      lines.push({
        index: idx,
        text: authoritative,
        spans: [],
      });
    }

    return lines;
  }, [allHighlighted, visibleStartLine, visibleEndLine, activeState.value, lineStarts]);

  /* –– gutter –– */
  const gutterWidth = largeFile ? 0 : computeGutterWidth(totalLines);
  const effectiveReadOnly = readOnly || largeFile;

  // Prevent creating extremely tall overlay containers which can crash or
  // destabilize the renderer when many large files are opened. If the visual
  // overlay would exceed MAX_OVERLAY_HEIGHT, we disable the overlay for this
  // document and fall back to plain textarea rendering.
  const totalHeight = totalLines * lineHeight;
  const MAX_OVERLAY_HEIGHT = 10_000_000; // 10 million px safe guard
  const overlayEnabled = highlightsEnabled && totalHeight > 0 && totalHeight <= MAX_OVERLAY_HEIGHT;
  if (!overlayEnabled && highlightsEnabled) {
    // Keep a debug trace for diagnostics; this is safe in dev but inexpensive.
    // eslint-disable-next-line no-console
    console.debug('[highlight] overlay disabled due to excessive height', {
      document: activeFilePath,
      totalLines,
      totalHeight,
    });
  }

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
        {overlayEnabled && (
          <div
            ref={highlightOuterRef}
            aria-hidden="true"
            tabIndex={-1}
            onMouseDown={() => {
              // Defensive: focus the real textarea if any event reaches the overlay.
              textareaRef.current?.focus();
            }}
            className="absolute inset-0 overflow-hidden pointer-events-none select-none text-editor-foreground"
            style={{
              lineHeight: `${lineHeight}px`,
              fontFamily: FONT_TOKENS.editor,
              fontSize: '0.875rem',
              whiteSpace: 'pre',
              overflowWrap: 'normal',
              // Defensive: ensure overlay never receives pointer events and is visually above the textarea.
              pointerEvents: 'none',
              zIndex: 30,
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
            // Hide the underlying textarea text while highlighting is enabled, but
            // preserve IME/composition and caret by also setting the WebKit text-fill
            // when required. This avoids composition/caret failures on some platforms.
            color: highlightsEnabled ? 'transparent' : undefined,
            WebkitTextFillColor: highlightsEnabled ? 'transparent' : undefined,
            // Keep the caret visible even when the text color is transparent so the user can type.
            // When the editor is read-only we hide the caret.
            caretColor: effectiveReadOnly ? 'transparent' : 'var(--editor-cursor-color, #E2E8F0)',
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
