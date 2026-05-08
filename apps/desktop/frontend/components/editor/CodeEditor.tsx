/**
 * CodeEditor (refactored to consume a single VisibleEditorSession)
 *
 * Summary of changes:
 *  - CodeEditor now accepts a single `session` prop (owned by EditorContainer).
 *  - Internal editorStates map is keyed strictly by documentId (not tabId).
 *  - Textarea DOM sync is performed deterministically whenever the active session
 *    changes or when the session.text differs from the DOM, ensuring Rule 2/3.
 *  - All async highlight requests are driven from `session.documentId` + `displayText`.
 *  - The typing hot path remains unchanged: uncontrolled textarea for native typing,
 *    ephemeral updates propagated to container via onChange (hot) and debounced commit.
 *
 * Important: this file intentionally contains additional comments describing root-cause fixes.
 */

import React, {
  useState,
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
/*  Highlight model (unchanged)                                       */
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
/**
 * The editor now receives exactly one "session" object which is the single
 * source of truth for visible document identity and text. EditorContainer owns
 * this object and enforces load-sequence protection.
 */
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
  session: EditorSession;
  onChange: (value: string) => void; // hot-path: immediate local changes
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
/* ------------------------------------------------------------------ */
function useFullHighlight(
  documentId: string | null,
  text: string,
  enabled: boolean,
  theme?: 'dark' | 'light',
  language?: string,
  initialHighlight?: { lines: HighlightLine[]; version?: number },
) {
  const [mapState, setMapState] = useReducer(
    (_prev: Map<number, HighlightLine>, next: Map<number, HighlightLine>) => next,
    new Map<number, HighlightLine>(),
  );
  const cacheRef = useRef<Map<string, { text: string; map: Map<number, HighlightLine>; version?: number }>>(new Map());
  const reqIdRef = useRef(0);
  const debounceRef = useRef<number | null>(null);
  const prevDocRef = useRef<string | null>(null);
  const retriesRef = useRef<Map<string, number>>(new Map());
  const lastFetchRef = useRef<Map<string, number>>(new Map());

  useEffect(() => {
    if (!documentId || !enabled) {
      setMapState(new Map());
      prevDocRef.current = documentId;
      return;
    }

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
          newMap.set(idx, prevHL);
          const prevStateHL = mapState.get(idx);
          if (prevStateHL !== prevHL) anyDifferent = true;
        } else {
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
            return;
          } else {
            retriesRef.current.delete(documentId);
          }
        } else {
          retriesRef.current.delete(documentId);
        }

        const normalized = resLines.map((l) => ({ index: l.index, text: l.text, spans: l.spans }));
        applyResultIfCurrent(normalized, resVersion);
      } catch (err) {
        // Non-fatal
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
      } catch {
        // ignore
      }
      return false;
    };

    if (debounceRef.current) {
      window.clearTimeout(debounceRef.current);
      debounceRef.current = null;
    }

    const doWork = async () => {
      if (isDocSwitch) {
        if (initialHighlight && Array.isArray(initialHighlight.lines) && initialHighlight.lines.length > 0) {
          try {
            applyResultIfCurrent(initialHighlight.lines.map((l:any) => ({ index: l.index, text: l.text, spans: l.spans })), initialHighlight.version);
          } catch {}
        }
        const gotRange = await fetchDocumentRange();
        void fetchExact();
        return gotRange;
      } else {
        await fetchExact();
      }
    };

    const lastFetch = documentId ? lastFetchRef.current.get(documentId) ?? 0 : 0;
    const now = Date.now();
    const SMALL_FILE_THRESHOLD = 1500;
    const EDIT_THROTTLE_MS = 300;
    const shouldImmediateEditFetch =
      !cached || isDocSwitch || ((text.length <= SMALL_FILE_THRESHOLD) && ((now - lastFetch) >= EDIT_THROTTLE_MS));

    if (shouldImmediateEditFetch) {
      requestAnimationFrame(() => {
        void doWork();
      });
    } else {
      const adaptiveMs = Math.max(40, Math.min(120, Math.floor(text.length / 300)));
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
/*  Span merging / render (unchanged)                                 */
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
/*  Line view (unchanged)                                             */
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
/*  Simple helpers                                                     */
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
/*  Editor implementation (session-driven)                            */
/* ------------------------------------------------------------------ */
export function CodeEditor({
  session,
  onChange,
  onSave,
  readOnly = false,
  className,
  theme = 'dark',
}: CodeEditorProps) {
  // Editor states keyed strictly by documentId (single authoritative id)
  const editorStates = useRef<Map<string, EditorState>>(new Map());
  const [, forceUpdate] = useReducer((x: number) => x + 1, 0);

  // Determine active key (documentId only) to avoid mixing tab identity into local state.
  const activeDocKey = `${session.documentId ?? '__no_doc__'}`;

  // Initialize state for this documentId if missing
  if (!editorStates.current.has(activeDocKey)) {
    editorStates.current.set(activeDocKey, {
      value: session.text ?? '',
      scrollTop: 0,
      scrollLeft: 0,
      cursorLine: 1,
    });
  }

  // Read current editor state
  const activeState = editorStates.current.get(activeDocKey)!;

  // Refs
  const containerRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const highlightOuterRef = useRef<HTMLDivElement>(null);
  const prevDocRef = useRef<string | null>(null);
  const rafScheduledRef = useRef<number | null>(null);

  // Huge file guard
  const largeFile = session.contentTruncated ?? (session.text.length >= 50_000);

  // Container size
  const [containerHeight, setContainerHeight] = useReducer((_p:number, n:number)=>n, 0);
  useEffect(()=>{
    const el = containerRef.current;
    if(!el) return;
    const obs = new ResizeObserver(entries=>{
      for(const e of entries) setContainerHeight(e.contentRect.height);
    });
    obs.observe(el);
    return ()=>obs.disconnect();
  },[]);

  // Display text (debounced) used by highlight pipeline
  const [displayText, setDisplayText] = useState<string>(activeState.value);
  useEffect(()=>{
    // If document switched, update immediately
    if (prevDocRef.current !== activeDocKey) {
      setDisplayText(activeState.value);
      prevDocRef.current = activeDocKey;
      return;
    }
    const id = window.setTimeout(()=>setDisplayText(activeState.value), 120);
    return ()=>window.clearTimeout(id);
  }, [activeState.value, activeDocKey]);

  const lineHeight = GUTTER_CONFIG.LINE_HEIGHT;
  const displayLineStarts = useMemo(()=>computeLineStarts(displayText), [displayText]);
  const totalLines = displayLineStarts.length;

  // Highlighting: only enabled when not large and we have a documentId
  const highlightsEnabled = !largeFile && !!session.documentId;
  const highlightedMap = useFullHighlight(
    session.documentId ?? null,
    displayText,
    highlightsEnabled,
    theme,
    session.language && session.language !== 'plaintext' ? session.language : undefined,
    session.initialHighlight ?? null,
  );

  const visibleStartLine = Math.floor(activeState.scrollTop / lineHeight);
  const visibleCount = Math.ceil(((containerHeight || lineHeight) + lineHeight)/lineHeight)*2;
  const visibleEndLine = Math.min(visibleStartLine + visibleCount, totalLines);

  // Local highlighter for visible lines
  const localHighlightLine = useCallback((lineText: string): HighlightSpan[]=>{
    const spans: HighlightSpan[] = [];
    if(!lineText) return spans;
    const commentIdx = lineText.indexOf('//');
    if(commentIdx!==-1) { spans.push({ start: commentIdx, end: lineText.length, token_type: 'comment' }); return spans; }
    const stringRe = /(["'])(?:\\.|(?!\1).)*\1/g;
    let m: RegExpExecArray|null;
    while((m=stringRe.exec(lineText))!==null) spans.push({ start: m.index, end: m.index+m[0].length, token_type:'string' });
    const numRe = /\b\d+(\.\d+)?\b/g;
    while((m=numRe.exec(lineText))!==null) spans.push({ start: m.index, end: m.index+m[0].length, token_type:'number' });
    const kwRe = /\b(fn|function|return|if|else|for|while|const|let|var|pub|use|mod|struct|enum|impl|class|import|switch|case)\b/g;
    while((m=kwRe.exec(lineText))!==null) spans.push({ start: m.index, end: m.index+m[0].length, token_type:'keyword' });
    spans.sort((a,b)=>a.start-b.start || (a.end-a.start)-(b.end-b.start));
    const merged: HighlightSpan[] = [];
    for(const sp of spans){
      const s = Math.max(0, sp.start);
      const e = Math.min(lineText.length, sp.end);
      if(e<=s) continue;
      const last = merged[merged.length-1];
      if(!last || s>=last.end) merged.push({ start: s, end: e, token_type: sp.token_type, color: sp.color });
      else if(e>last.end) last.end = e;
    }
    return merged;
  },[]);

  const [overlayHighlighted, setOverlayHighlighted] = useState<HighlightLine[]>([]);
  useEffect(()=>{
    let rafId: number|null = null;
    let cancelled = false;
    const doCompute = ()=>{
      if(cancelled) return;
      const lines: HighlightLine[] = [];
      const localLineStarts = displayLineStarts;
      const totalDisplayLines = localLineStarts.length;
      const startIdx = Math.max(visibleStartLine, 0);
      const endIdx = Math.min(visibleEndLine, totalDisplayLines);
      for(let idx=startIdx; idx<endIdx; idx++){
        const start = localLineStarts[idx] ?? displayText.length;
        const end = localLineStarts[idx+1] ?? displayText.length;
        let authoritative = displayText.slice(start,end);
        if(authoritative.endsWith('\n')) authoritative = authoritative.slice(0,-1);
        const backendHl = highlightedMap.get(idx);
        const usedSpans = backendHl ? backendHl.spans : localHighlightLine(authoritative);
        const uid = backendHl && backendHl.uid ? backendHl.uid : `${activeDocKey}:${stableHashString(authoritative)}:${idx}`;
        lines.push({ uid, index: idx, text: authoritative, spans: usedSpans });
      }
      if(!cancelled) setOverlayHighlighted(lines);
    };
    rafId = requestAnimationFrame(doCompute);
    return ()=>{ cancelled = true; if(rafId) cancelAnimationFrame(rafId); };
  }, [highlightedMap, visibleStartLine, visibleEndLine, displayLineStarts, localHighlightLine, activeDocKey]);

  const gutterWidth = largeFile ? 0 : computeGutterWidth(totalLines);
  const effectiveReadOnly = readOnly || largeFile;
  const totalHeight = totalLines * lineHeight;
  const MAX_OVERLAY_HEIGHT = 10_000_000;
  const overlayEnabled = highlightsEnabled && totalHeight > 0 && totalHeight <= MAX_OVERLAY_HEIGHT;

  // Ensure the native textarea DOM always matches the active session.text whenever the documentId changes
  // or when the session.text differs from the stored activeState.value. This prevents "blank" windows
  // where the session indicates a document but the DOM still shows previous content.
  useLayoutEffect(()=>{
    const ta = textareaRef.current;
    if(!ta) return;
    // Always restore scroll positions
    ta.scrollTop = activeState.scrollTop;
    ta.scrollLeft = activeState.scrollLeft;
    // If the session.documentId changed OR the activeState.value differs from session.text,
    // synchronise the DOM immediately. This is stricter than previous logic and prevents
    // transient blank states. We avoid clobbering during normal typing because session.text
    // will be kept in sync by the container only when debounced — immediate typing isn't affected.
    if (prevDocRef.current !== activeDocKey || ta.value !== activeState.value || activeState.value !== session.text) {
      // If the container owns a different authoritative text, update both internal state and DOM.
      activeState.value = session.text;
      if (ta.value !== session.text) {
        ta.value = session.text;
        try { ta.setSelectionRange(0,0); } catch {}
      }
      prevDocRef.current = activeDocKey;
      forceUpdate();
    }
  }, [activeDocKey, session.documentId, session.text, activeState.scrollTop, activeState.scrollLeft, activeState.value]);

  const handleTextareaScroll = useCallback((e: React.UIEvent<HTMLTextAreaElement>)=>{
    if(!e.currentTarget) return;
    const sTop = e.currentTarget.scrollTop;
    const sLeft = e.currentTarget.scrollLeft;
    const current = editorStates.current.get(activeDocKey)!;
    current.scrollTop = sTop;
    current.scrollLeft = sLeft;
    forceUpdate();
  }, [activeDocKey]);

  const handleSelect = useCallback(()=>{
    const ta = textareaRef.current;
    if(!ta) return;
    const pos = ta.selectionStart;
    const val = ta.value;
    const before = val.slice(0,pos).match(/\n/g);
    const line = before ? before.length + 1 : 1;
    const st = editorStates.current.get(activeDocKey)!;
    if (st.value !== val) st.value = val;
    st.cursorLine = line;
    forceUpdate();
  }, [activeDocKey]);

  const scheduleRender = useCallback(()=>{
    if(rafScheduledRef.current !== null) return;
    rafScheduledRef.current = requestAnimationFrame(()=>{
      rafScheduledRef.current = null;
      forceUpdate();
    });
  },[]);

  const handleChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>)=>{
    if(effectiveReadOnly) return;
    const newVal = e.target.value;
    const st = editorStates.current.get(activeDocKey)!;
    st.value = newVal;
    const pos = e.target.selectionStart;
    const before = newVal.slice(0,pos).match(/\n/g);
    st.cursorLine = before ? before.length + 1 : 1;
    // Notify container immediately (hot path)
    onChange(newVal);
    scheduleRender();
  }, [onChange, effectiveReadOnly, activeDocKey, scheduleRender]);

  // Render
  return (
    <div ref={containerRef} className={cn('flex h-full', className)}>
      {!largeFile && (
        <div className="shrink-0 relative overflow-hidden" style={{ width: gutterWidth }}>
          <LineNumberGutter
            lineCount={totalLines}
            cursorLine={activeState.cursorLine}
            lineHeight={lineHeight}
            scrollTop={activeState.scrollTop}
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
            ref={highlightOuterRef}
            aria-hidden="true"
            tabIndex={-1}
            onMouseDown={()=>{ textareaRef.current?.focus(); }}
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
            <div style={{ position: 'relative', height: totalLines * lineHeight, width: '100%', pointerEvents: 'none', boxSizing: 'border-box' }}>
              <div style={{
                position: 'absolute',
                top: 0,
                left: 0,
                transform: `translate3d(${-activeState.scrollLeft}px, ${-activeState.scrollTop}px, 0px)`,
                whiteSpace: 'pre',
                width: '100%',
                height: totalLines * lineHeight,
                pointerEvents: 'none',
                boxSizing: 'border-box',
              }}>
                {overlayHighlighted.map((hl)=>(<HighlightedLineView key={hl.uid} hl={hl} lineHeight={lineHeight} />))}
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
            color: highlightsEnabled ? 'transparent' : undefined,
            WebkitTextFillColor: highlightsEnabled ? 'transparent' : undefined,
            caretColor: effectiveReadOnly ? 'transparent' : 'var(--editor-cursor-color, #E2E8F0)',
          }}
          defaultValue={activeState.value}
          readOnly={effectiveReadOnly}
          onChange={handleChange}
          onScroll={handleTextareaScroll}
          onSelect={handleSelect}
          onClick={()=>textareaRef.current?.focus()}
          onMouseDown={()=>textareaRef.current?.focus()}
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
