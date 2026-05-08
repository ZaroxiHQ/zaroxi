import React, {
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
/**
 * We add a small stable `uid` to each HighlightLine produced by the backend
 * or created locally. The uid allows the renderer to keep DOM nodes stable
 * across asynchronous updates and reduces remounts/flashing.
 */
interface HighlightSpan {
  start: number;
  end: number;
  token_type: string;
  color?: string;
}
interface HighlightLine {
  uid: string;           // stable identifier for this logical line presentation
  index: number;         // current 0-based line index (used for positioning)
  text: string;
  spans: HighlightSpan[];
}
interface HighlightResponse {
  lines: HighlightLine[];
  version: number;
}

const FULL_LINES_LIMIT = 100_000;

/**
 * Request highlighting for the exact editor text currently displayed.
 *
 * Improvements made here:
 * - Each created HighlightLine now receives a stable uid based on documentId + version + lineIndex.
 * - Responses are ignored when a newer request id exists (existing behavior preserved).
 * - Cached authoritative maps are stored in cacheRef keyed by documentId + text to allow immediate reuse.
 */
function useFullHighlight(
  documentId: string | null,
  text: string,
  enabled: boolean,
  theme?: 'dark' | 'light',
  language?: string,
) {
  // Return a Map<lineIndex, HighlightLine> so the view layer can reuse unchanged objects by identity.
  const [mapState, setMapState] = useReducer(
    (_prev: Map<number, HighlightLine>, next: Map<number, HighlightLine>) => next,
    new Map<number, HighlightLine>(),
  );

  // Per-document cache: exact text -> stable map + version
  const cacheRef = useRef<Map<string, { text: string; map: Map<number, HighlightLine>; version?: number }>>(new Map());
  const reqIdRef = useRef(0);
  const debounceRef = useRef<number | null>(null);
  const prevDocRef = useRef<string | null>(null);
  const retriesRef = useRef<Map<string, number>>(new Map());

  // Tuned scheduling parameters to avoid pointless roundtrips while typing.
  const SMALL_FILE_THRESHOLD = 1500;
  const MIN_DEBOUNCE_MS = 40;
  const MAX_DEBOUNCE_MS = 120;
  const EDIT_THROTTLE_MS = 300;

  const lastFetchRef = useRef<Map<string, number>>(new Map());

  useEffect(() => {
    if (!documentId || !enabled) {
      setMapState(new Map());
      prevDocRef.current = documentId;
      return;
    }

    // Fast path: exact cached text -> apply stable map immediately.
    const cached = cacheRef.current.get(documentId);
    if (cached && cached.text === text) {
      setMapState(cached.map);
      prevDocRef.current = documentId;
      return;
    }

    const isDocSwitch = prevDocRef.current !== documentId;
    prevDocRef.current = documentId;

    reqIdRef.current += 1;
    const thisReq = reqIdRef.current;
    let cancelled = false;

    const recordFetch = () => {
      if (!documentId) return;
      lastFetchRef.current.set(documentId, Date.now());
    };

    /**
     * When applying a backend result we:
     * - Reuse any previous objects at the same line index when text+spans match.
     * - Assign a deterministic uid for each created line: `${documentId}:${resVersion}:${idx}`
     *   so that the renderer can use the uid as a stable key.
     */
    const applyResultIfCurrent = (resLines: { index: number; text: string; spans: HighlightSpan[] }[], resVersion?: number) => {
      if (cancelled || reqIdRef.current !== thisReq) return false;

      const prevCacheEntry = cacheRef.current.get(documentId);
      const prevMapForReuse = prevCacheEntry ? prevCacheEntry.map : mapState;

      const newMap = new Map<number, HighlightLine>();
      let anyDifferent = false;

      for (const l of resLines) {
        const idx = l.index;
        const resSpansJson = JSON.stringify(l.spans);

        const prevHL = prevMapForReuse ? prevMapForReuse.get(idx) : undefined;
        if (prevHL && prevHL.text === l.text && JSON.stringify(prevHL.spans) === resSpansJson) {
          // reuse object and preserve uid
          newMap.set(idx, prevHL);
          const prevStateHL = mapState.get(idx);
          if (prevStateHL !== prevHL) anyDifferent = true;
        } else {
          // create new object with deterministic uid
          const uid = `${documentId}:${resVersion ?? 0}:${idx}`;
          const created: HighlightLine = {
            uid,
            index: idx,
            text: l.text,
            spans: l.spans,
          };
          newMap.set(idx, created);
          const prevStateHL = mapState.get(idx);
          if (
            !prevStateHL ||
            prevStateHL.text !== created.text ||
            JSON.stringify(prevStateHL.spans) !== resSpansJson
          ) {
            anyDifferent = true;
          }
        }
      }

      if (mapState.size !== newMap.size) anyDifferent = true;

      // Cache authoritative result for this exact text so future opens can use it immediately.
      cacheRef.current.set(documentId, { text, map: newMap, version: resVersion });

      if (anyDifferent) {
        setMapState(newMap);
      }

      return true;
    };

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

        const resLines = res.lines || [];
        const resVersion = res.version;

        const prevCache = cacheRef.current.get(documentId);
        const resEmpty = resLines.length === 0;
        const hasPrevNonEmpty = prevCache && prevCache.text === text && prevCache.map.size > 0;

        if (resEmpty && hasPrevNonEmpty) {
          const attempts = retriesRef.current.get(documentId) ?? 0;
          if (attempts < 2) {
            retriesRef.current.set(documentId, attempts + 1);
            const backoff = 80 * (attempts + 1);
            setTimeout(() => {
              if (!cancelled && reqIdRef.current === thisReq) {
                void fetchExact();
              }
            }, backoff);
            console.debug('[highlight] transient empty — retry scheduled', { documentId, attempts: attempts + 1 });
            return;
          } else {
            retriesRef.current.delete(documentId);
          }
        } else {
          retriesRef.current.delete(documentId);
        }

        // Map backend DTO to minimal tuples to avoid carrying any unexpected fields.
        const normalized = resLines.map((l) => ({ index: l.index, text: l.text, spans: l.spans }));
        applyResultIfCurrent(normalized, resVersion);
      } catch (err) {
        console.warn('highlight_text failed:', err);
      }
    };

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
          const normalized = res.lines.map((l) => ({ index: l.index, text: l.text, spans: l.spans }));
          applyResultIfCurrent(normalized, res.version);
          return true;
        }
      } catch (err) {
        console.debug('highlight_document failed or not present:', err);
      }
      return false;
    };

    if (debounceRef.current) {
      window.clearTimeout(debounceRef.current);
      debounceRef.current = null;
    }

    const doWork = async () => {
      if (isDocSwitch) {
        // Try range fetch first — it gives us many lines quickly for initial painting.
        const gotRange = await fetchDocumentRange();
        // Always request the precise highlighting for the exact text as well to ensure correctness.
        void fetchExact();
        return gotRange;
      } else {
        await fetchExact();
      }
    };

    const lastFetch = documentId ? lastFetchRef.current.get(documentId) ?? 0 : 0;
    const now = Date.now();
    const shouldImmediateEditFetch =
      !cached || isDocSwitch || text.length <= SMALL_FILE_THRESHOLD || (now - lastFetch) >= EDIT_THROTTLE_MS;

    if (shouldImmediateEditFetch) {
      // Schedule on next animation frame to coalesce micro-bursts of edits.
      requestAnimationFrame(() => {
        void doWork();
      });
    } else {
      const adaptiveMs = Math.max(
        MIN_DEBOUNCE_MS,
        Math.min(MAX_DEBOUNCE_MS, Math.floor(text.length / 300)),
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

  return mapState;
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

  // Render spans using token-type class names only.
  // Rationale:
  // - We do NOT hardcode a mapping from token -> CSS color here.
  // - Styling comes from theme CSS (token classes) or the backend may include
  //   an explicit `color` on a span (used as a last-resort inline override).
  // - Keeping styling purely class-driven allows Tree-sitter / theme to evolve
  //   without client-side mappings and reduces layout churn.
  const segments: React.ReactNode[] = [];
  let last = 0;
  for (let i = 0; i < merged.length; i++) {
    const sp = merged[i];
    if (sp.start > last) {
      segments.push(lineText.slice(last, sp.start));
    }
    const key = `${sp.start}-${i}`;

    // Stable token class derived from token_type (sanitised).
    const tokenClass = `syntax-${String(sp.token_type || 'plain')
      .toLowerCase()
      .replace(/[^a-z0-9_-]/g, '-')}`;

    // Only apply inline style when backend provided a concrete color.
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

/**
 * Highlighted line renderer - memoized to avoid rebuilding DOM for unchanged lines.
 *
 * Notes:
 * - We use `uid` as the component key in the parent list. The memo comparator
 *   compares uid/text/spans to determine equality and avoid unnecessary DOM churn.
 */
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
    // If uid didn't change and lineHeight unchanged, skip work.
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
    return true; // equal -> skip update
  },
);

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
  const highlightedMap = useFullHighlight(
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

  /**
   * visiblePrevRef stores a histogram of previously-rendered line objects keyed
   * by fingerprint (text + spans). Each entry is an array of available instances
   * (this handles duplicate identical lines in the viewport). When building the
   * new visible slice, we attempt to reuse an available instance for the same
   * fingerprint. This preserves object identity and prevents remounts / flashes
   * for unchanged content even when scrolling or nearby edits shift indices.
   */
  const visiblePrevRef = useRef<Map<string, HighlightLine[]>>(new Map());

  /**
   * Lightweight local highlighter for visible lines.
   * Provides an immediate, deterministic visual rendering while backend spans are pending.
   * Only applied to visible lines to avoid CPU cost during continuous typing.
   */
  const localHighlightLine = useCallback((lineText: string): HighlightSpan[] => {
    const spans: HighlightSpan[] = [];
    if (!lineText || lineText.length === 0) return spans;

    const commentIdx = lineText.indexOf('//');
    if (commentIdx !== -1) {
      spans.push({ start: commentIdx, end: lineText.length, token_type: 'comment' });
      return spans;
    }

    const stringRe = /(["'])(?:\\.|(?!\1).)*\1/g;
    let m: RegExpExecArray | null;
    while ((m = stringRe.exec(lineText)) !== null) {
      spans.push({ start: m.index, end: m.index + m[0].length, token_type: 'string' });
    }

    const numRe = /\b\d+(\.\d+)?\b/g;
    while ((m = numRe.exec(lineText)) !== null) {
      spans.push({ start: m.index, end: m.index + m[0].length, token_type: 'number' });
    }

    const kwRe = /\b(fn|function|return|if|else|for|while|const|let|var|pub|use|mod|struct|enum|impl|class|import|switch|case)\b/g;
    while ((m = kwRe.exec(lineText)) !== null) {
      spans.push({ start: m.index, end: m.index + m[0].length, token_type: 'keyword' });
    }

    spans.sort((a, b) => a.start - b.start || (a.end - a.start) - (b.end - b.start));
    const merged: HighlightSpan[] = [];
    for (const sp of spans) {
      const s = Math.max(0, sp.start);
      const e = Math.min(lineText.length, sp.end);
      if (e <= s) continue;
      const last = merged[merged.length - 1];
      if (!last || s >= last.end) {
        merged.push({ start: s, end: e, token_type: sp.token_type, color: sp.color });
      } else if (e > last.end) {
        last.end = e;
      }
    }
    return merged;
  }, []);

  const visibleHighlighted = useMemo(() => {
    const prevBuckets = visiblePrevRef.current;
    const newBuckets = new Map<string, HighlightLine[]>();
    const lines: HighlightLine[] = [];

    const fingerprint = (text: string, spans: HighlightSpan[]) =>
      text + '|' + (spans.length ? JSON.stringify(spans) : '[]');

    // Populate an availability map so each identical fingerprint can reuse
    // previously-rendered instances in FIFO order.
    const available = new Map<string, HighlightLine[]>();
    for (const [k, arr] of prevBuckets.entries()) {
      available.set(k, arr.slice());
    }

    for (let idx = visibleStartLine; idx < visibleEndLine; idx++) {
      const start = lineStarts[idx] ?? activeState.value.length;
      const end = lineStarts[idx + 1] ?? activeState.value.length;
      let authoritative = activeState.value.slice(start, end);
      if (authoritative.endsWith('\n')) authoritative = authoritative.slice(0, -1);

      // Prefer backend spans for this index; if missing, try to match by text (handles shifted indices).
      let backendHl = highlightedMap.get(idx);
      if (!backendHl) {
        for (const v of highlightedMap.values()) {
          if (v.text === authoritative) {
            backendHl = v;
            break;
          }
        }
      }

      const usedSpans = backendHl ? backendHl.spans : localHighlightLine(authoritative);
      const key = fingerprint(authoritative, usedSpans);

      // Try to reuse an existing instance for this exact fingerprint
      const bucket = available.get(key);
      if (bucket && bucket.length > 0) {
        const instance = bucket.shift()!;
        // Update index for positioning while preserving uid and spans identity.
        instance.index = idx;
        instance.text = authoritative;
        instance.spans = usedSpans;
        lines.push(instance);

        // Record for next cycle reuse
        const outArr = newBuckets.get(key) ?? [];
        outArr.push(instance);
        newBuckets.set(key, outArr);
        continue;
      }

      // No reusable instance: create a new one. Use backend uid when available for stability,
      // otherwise derive a locally-stable uid using documentId + a timestamp fallback.
      const uid = backendHl && backendHl.uid ? backendHl.uid : `${documentId}:${Date.now()}:${idx}`;
      const created: HighlightLine = {
        uid,
        index: idx,
        text: authoritative,
        spans: usedSpans,
      };
      lines.push(created);
      const outArr = newBuckets.get(key) ?? [];
      outArr.push(created);
      newBuckets.set(key, outArr);
    }

    // Save for next render; we keep only the visible bucketed instances to limit memory use.
    visiblePrevRef.current = newBuckets;
    return lines;
  }, [highlightedMap, visibleStartLine, visibleEndLine, activeState.value, lineStarts, localHighlightLine]);

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
                  // Use the stable uid as the key. This is the critical change that
                  // prevents large remounts when lines are inserted/removed elsewhere
                  // in the document; the uid is preserved for identical content.
                  <HighlightedLineView key={hl.uid} hl={hl} lineHeight={lineHeight} />
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
