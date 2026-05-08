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
  const [lines, setLines] = useReducer(
    (_prev: HighlightLine[], next: HighlightLine[]) => next,
    [],
  );

  // Per-document cache: maps documentId -> { text, lines } so we can reuse
  // highlights when switching back to a tab without re-requesting if the text
  // has not changed. This prevents unnecessary refreshes when opening/switching tabs.
  const cacheRef = useRef<Map<string, { text: string; lines: HighlightLine[] }>>(new Map());

  const reqIdRef = useRef(0);
  const debounceRef = useRef<number | null>(null);
  const lastAppliedRef = useRef<{
    documentId: string | null;
    textHash: number | null;
  }>({ documentId: null, textHash: null });

  // Synchronous lightweight fallback highlighter for immediate visual feedback.
  // This runs on the exact `text` supplied by the editor (single source of truth)
  // and gives instant coloring while the backend Tree-sitter highlighter runs.
  function computeImmediateFallback(fullText: string, langHint?: string): HighlightLine[] {
    if (!fullText) return [];
    const linesArr = fullText.split('\n');
    const out: HighlightLine[] = [];
    // simple heuristics per-language (keep minimal to avoid heavy CPU)
    const commentPattern = langHint === 'toml' || langHint === 'ini' ? /#.*/ : /\/\/.*/;
    const stringPattern = /(["'])(?:\\.|(?!\1).)*\1/g;
    const numberPattern = /\b\d+(\.\d+)?\b/g;

    for (let idx = 0; idx < linesArr.length; idx++) {
      const line = linesArr[idx];
      const spans: HighlightSpan[] = [];

      // comments
      const cm = line.match(commentPattern);
      if (cm && cm.index !== undefined) {
        spans.push({ start: cm.index, end: line.length, token_type: 'comment', color: '#7C7C7C' });
      }

      // strings
      let m: RegExpExecArray | null;
      // eslint-disable-next-line no-cond-assign
      while ((m = stringPattern.exec(line)) !== null) {
        spans.push({ start: m.index ?? 0, end: (m.index ?? 0) + m[0].length, token_type: 'string', color: '#98C379' });
      }

      // numbers (only when not inside strings)
      // naive approach: skip number matches that fall inside existing spans
      const numberMatches = Array.from(line.matchAll(numberPattern));
      for (const nm of numberMatches) {
        const ns = nm.index ?? 0;
        const ne = ns + (nm[0]?.length ?? 0);
        let inside = false;
        for (const s of spans) {
          if (ns >= s.start && ne <= s.end) {
            inside = true;
            break;
          }
        }
        if (!inside) {
          spans.push({ start: ns, end: ne, token_type: 'number', color: '#D19A66' });
        }
      }

      // sort spans by start and merge simple overlaps (prefer earlier spans)
      spans.sort((a, b) => a.start - b.start || a.end - b.end);
      out.push({ index: idx, text: line, spans });
    }
    return out;
  }

  // Helper to compute a simple hash for the supplied text.
  function hashText(s: string): number {
    let h = 2166136261 >>> 0;
    for (let i = 0; i < s.length; i++) {
      h ^= s.charCodeAt(i);
      h += (h << 1) + (h << 4) + (h << 7) + (h << 8) + (h << 24);
    }
    return h >>> 0;
  }

  useEffect(() => {
    if (!documentId || !enabled) {
      setLines([]);
      lastAppliedRef.current = { documentId, textHash: null };
      return;
    }

    const currentTextHash = hashText(text);

    // If we have cached highlights for this document and the source text is identical,
    // reuse them immediately and skip an IPC roundtrip.
    const cached = cacheRef.current.get(documentId);
    if (cached && cached.text === text) {
      // Avoid setting state if we already applied this cached result.
      if (lastAppliedRef.current.documentId !== documentId || lastAppliedRef.current.textHash !== currentTextHash) {
        console.debug('[highlight] applying cached highlights', { documentId, lines: cached.lines?.length });
        setLines(cached.lines);
        lastAppliedRef.current = { documentId, textHash: currentTextHash };
      }
      return;
    }

    // Immediately apply a lightweight fallback so the user sees colored text on first paint.
    try {
      const immediate = computeImmediateFallback(text, language);
      // Only apply fallback if we haven't already applied the same text.
      if (lastAppliedRef.current.documentId !== documentId || lastAppliedRef.current.textHash !== currentTextHash) {
        console.debug('[highlight] applying immediate fallback highlights', { documentId, lines: immediate.length });
        setLines(immediate);
        lastAppliedRef.current = { documentId, textHash: currentTextHash };
      }
    } catch (e) {
      // Fallback highlighter must not throw; ignore.
      console.warn('[highlight] fallback failed', e);
    }

    let cancelled = false;
    // bump request id for this batch
    reqIdRef.current += 1;
    const thisReq = reqIdRef.current;

    const doFetch = async () => {
      try {
        console.debug('[highlight] requesting highlight_text', { documentId, length: text.length, language });
        // First attempt: ask the backend to highlight the exact text, using any language hint.
        const args = {
          request: {
            documentId,
            text,
            theme: theme ?? 'dark',
            language: language ?? undefined,
          },
        };
        const res: HighlightResponse = await bridge.invoke('highlight_text', args);
        console.debug('[highlight] highlight_text response', { documentId, lines: res?.lines?.length });

        // Only apply if still current and not cancelled.
        if (cancelled || reqIdRef.current !== thisReq) {
          console.debug('[highlight] response ignored (stale/cancelled)', { documentId });
          return;
        }

        // Cache the spans by the exact text and apply them only if they differ
        // from what we've already applied for this document/text. This avoids
        // visible reflows when the same highlights are reapplied.
        cacheRef.current.set(documentId, { text, lines: res.lines || [] });
        const applied = lastAppliedRef.current;
        if (applied.documentId !== documentId || applied.textHash !== currentTextHash) {
          console.debug('[highlight] applying backend highlights', { documentId, count: res.lines?.length });
          setLines(res.lines || []);
          lastAppliedRef.current = { documentId, textHash: currentTextHash };
        } else {
          // If we already applied the same text (unlikely here), still update cache.
          // No UI update necessary.
          console.debug('[highlight] backend highlights match already applied state', { documentId });
        }
      } catch (err) {
        console.warn('full highlight (text) failed:', err);
        if (!cancelled && reqIdRef.current === thisReq) {
          // Leave whatever fallback/cached highlights are currently visible instead of clearing.
        }
      }
    };

    // Clear any prior debounce
    if (debounceRef.current) {
      window.clearTimeout(debounceRef.current);
      debounceRef.current = null;
    }

    // When switching document or language, fetch immediately (no debounce) so backend highlights arrive quickly.
    // Otherwise debounce to batch keystrokes.
    const isDocSwitch = lastAppliedRef.current.documentId !== documentId || lastAppliedRef.current.textHash !== currentTextHash;

    if (isDocSwitch) {
      // run without extra delay to get authoritative highlights soon
      void doFetch();
    } else {
      // Debounce edits to avoid flooding the backend when typing; still fetch after short delay.
      debounceRef.current = window.setTimeout(() => {
        void doFetch();
      }, 120);
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

  // Sort spans by start (primary) then end (secondary) to ensure stable,
  // left-to-right processing. Previous implementation sorted by span length,
  // which produced incorrect overlay ordering and prevented highlights from
  // being rendered correctly.
  const sorted = [...spans].sort((a, b) => {
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
    // pass frontend-known language hint (if any) to improve detection
    language,
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
      // If the backend returned a highlighted line for this index, use it.
      const hl = map.get(idx);
      if (hl) {
        lines.push(hl);
        continue;
      }

      // Fallback: derive the exact visible line text from the authoritative editor text.
      // This ensures the overlay always renders the real document content (single source of truth)
      // even when the backend hasn't produced highlights (e.g. PlainText or language not available).
      const start = lineStarts[idx] ?? activeState.value.length;
      const end = lineStarts[idx + 1] ?? activeState.value.length;
      let raw = activeState.value.slice(start, end);
      if (raw.endsWith('\n')) raw = raw.slice(0, -1);
      lines.push({
        index: idx,
        text: raw,
        spans: [],
      });
    }

    return lines;
  }, [allHighlighted, visibleStartLine, visibleEndLine, activeState.value, lineStarts]);

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
              // The overlay is intentionally non-interactive (pointer-events: none)
              // so clicks pass through to the real textarea. We keep it above the
              // textarea visually so highlighted markup is always visible.
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
