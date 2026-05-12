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
  useLayoutEffect,
} from 'react';
import { cn } from '@/lib/utils';
import { LineNumberGutter } from './gutter/LineNumberGutter';
import { GUTTER_CONFIG } from './gutter/GutterConfig';
import { computeGutterWidth } from './gutter/GutterLayout';
import { FONT_TOKENS } from '@/lib/theme/font-tokens';
import { bridge } from '@/lib/bridge';
import CodeMirrorEditor from './CodeMirrorEditor';
import EditorSessionStore from '@/stores/EditorSessionStore';

import { getDocumentSyntax, setDocumentSyntax, clearDocumentSyntax } from './syntaxStore';
import { debug, warn } from '@/lib/logger';

// Frontend per-document syntax session store is now persisted in ./syntaxStore.
// The local hook will consult and update that store instead of keeping the
// authoritative snapshot solely in component-local state.

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
  documentId: string;
  language: string;
  lines: HighlightLine[];
  version: number;
}

const FULL_LINES_LIMIT = 100_000;
const LARGE_FILE_CHAR_THRESHOLD = 50_000; // disable syntax for files larger than this (characters)
const LARGE_FILE_BYTES = 5 * 1024 * 1024; // 5 MB - hard threshold: no syntax above this

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
  // Per-hook uid counter to generate unique canonical line ids when not supplied
  const uidCounterRef = useRef(0);
  // Timer ref used to debounce bridge highlight requests from this hook.
  const bridgeTimerRef = useRef<number | null>(null);

  // Seed cache/state immediately from an initialSnapshot when it exactly matches the
  // provided `text`. This ensures the frontend can render the server-provided compact
  // snapshot on first paint without any timers or staged phases.
  useEffect(() => {
    // Only seed an initial snapshot when highlighting is enabled for this session.
    if (!documentId || !initialSnapshot || !enabled) return;

    // Sanity: ensure the snapshot belongs to the same document and language.
    if ((initialSnapshot as any).documentId !== documentId) return;
    if (language && (initialSnapshot as any).language !== language) return;

    const reconstructed = initialSnapshot.lines.map((l) => l.text).join('\n');
    const matchesText =
      reconstructed === text ||
      reconstructed + '\n' === text ||
      (text.endsWith('\n') && reconstructed === text.slice(0, -1));
    if (!matchesText) return;

    if (initialSnapshot.lines.length > 0) {
      const seeded = new Map<number, HighlightLine>();
      for (const l of initialSnapshot.lines) {
        // Prefer the server-provided uid when present. Otherwise generate a
        // unique uid that does NOT embed the current numeric index so that
        // identities survive line insert/delete shifts.
        const uid = l.uid ?? `${documentId}:${stableHashString(l.text)}:${uidCounterRef.current++}`;
        seeded.set(l.index, { uid, index: l.index, text: l.text, spans: l.spans });
      }
      const textKey = `${documentId}::${stableHashString(text)}`;
      cacheRef.current.set(textKey, { text, map: seeded, version: initialSnapshot.version });
      setMapState(new Map(seeded));
      // Also populate document-bound store for other mounts.
      setDocumentSyntax(documentId, { text, map: new Map(seeded), version: initialSnapshot.version, language: (initialSnapshot as any).language ?? undefined });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [documentId, initialSnapshot, enabled]);

  // Core: request highlights with debounce when documentId/text/enabled/language/theme changes.
  // Debounce avoids bridge churn on fast typing and prevents tight feedback loops that can cause
  // React update depth problems when stores/other components react to persisted snapshots.
  useEffect(() => {
    if (!documentId) {
      // No document -> clear visible map.
      setMapState(new Map());
      return;
    }

    // If highlighting is disabled for this session (large file or explicit off),
    // clear any prior snapshot immediately and do not issue highlight requests.
    if (!enabled) {
      setMapState(new Map());
      cacheRef.current.clear();
      return;
    }

    // Fast-path: if a document-bound store has highlights for this exact text, reuse.
    const global = documentId ? getDocumentSyntax(documentId) : undefined;
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

    // Start request generation with a monotonic id to discard stale arrivals.
    reqIdRef.current += 1;
    const thisReq = reqIdRef.current;

    // Debounce the bridge invocation to avoid issuing a request per keystroke.
    if (bridgeTimerRef.current) {
      window.clearTimeout(bridgeTimerRef.current);
      bridgeTimerRef.current = null;
    }
    bridgeTimerRef.current = window.setTimeout(async () => {
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

        // Ensure the response matches the document and language we requested.
        if (res.documentId !== documentId) return;
        const requestedLang = language ?? '';
        if (res.language !== requestedLang) return;

        // Ensure the returned snapshot was produced from the exact text we sent.
        const reconstructed = res.lines.map((l) => l.text).join('\n');
        const matchesText =
          reconstructed === text ||
          reconstructed + '\n' === text ||
          (text.endsWith('\n') && reconstructed === text.slice(0, -1));
        if (!matchesText) return;

        // Build a full incoming map and replace the visible map atomically.
        const incomingMap = new Map<number, HighlightLine>();
        const prevSessionForCache = documentId ? getDocumentSyntax(documentId) : undefined;
        const usedUidsLocal = new Set<string>();

        for (const l of res.lines) {
          // Prefer prior uid for same index/text from the persisted document store.
          let canonicalUid: string | undefined;
          if (prevSessionForCache && prevSessionForCache.map) {
            const prevEntry = prevSessionForCache.map.get(l.index);
            if (prevEntry && prevEntry.text === l.text && !usedUidsLocal.has(prevEntry.uid)) {
              canonicalUid = prevEntry.uid;
              usedUidsLocal.add(canonicalUid);
            } else {
              for (const [_, v] of prevSessionForCache.map) {
                if (v.text === l.text && !usedUidsLocal.has(v.uid)) {
                  canonicalUid = v.uid;
                  usedUidsLocal.add(canonicalUid);
                  break;
                }
              }
            }
          }

          // Fallback: generate a deterministic uid that includes a per-hook counter
          // to guarantee uniqueness while allowing reuse across snapshots.
          if (!canonicalUid) {
            canonicalUid = `${documentId}:${stableHashString(l.text)}:${uidCounterRef.current++}`;
            usedUidsLocal.add(canonicalUid);
          }

          incomingMap.set(l.index, { uid: canonicalUid, index: l.index, text: l.text, spans: l.spans });
        }

        // Patch existing map non-destructively to avoid remount storms.
        setMapState((prev) => {
          const updated = new Map(prev);
          const incomingIndices = new Set<number>();

          for (const [idx, hl] of incomingMap.entries()) {
            incomingIndices.add(idx);
            const existing = updated.get(idx);
            let changed = false;
            if (!existing) {
              changed = true;
            } else if (existing.text !== hl.text) {
              changed = true;
            } else {
              const sa = existing.spans || [];
              const sb = hl.spans || [];
              if (sa.length !== sb.length) {
                changed = true;
              } else {
                for (let i = 0; i < sa.length; i++) {
                  const a = sa[i];
                  const b = sb[i];
                  if (!a || !b || a.start !== b.start || a.end !== b.end || a.token_type !== b.token_type || (a.color ?? null) !== (b.color ?? null)) {
                    changed = true;
                    break;
                  }
                }
              }
            }
            if (changed) {
              updated.set(idx, hl);
            }
          }

          // Remove indices not present in the incoming snapshot.
          for (const k of Array.from(updated.keys())) {
            if (!incomingIndices.has(k)) {
              updated.delete(k);
            }
          }

          return updated;
        });

        // Cache result for fast reuse.
        cacheRef.current.set(textKey, { text, map: incomingMap, version: res.version });

        // Persisting into global store is intentionally disabled here to avoid
        // cross-component render cycles that can trigger update-depth issues.
        // If persistence is later required, gate it behind an explicit, non-hot-path flush.
        // setDocumentSyntax(documentId, { text, map: new Map(incomingMap), version: res.version, language: res.language });
      } catch {
        // Non-fatal: keep previous snapshot visible.
      }
    }, 200) as unknown as number;

    return () => {
      try {
        if (bridgeTimerRef.current) {
          window.clearTimeout(bridgeTimerRef.current);
          bridgeTimerRef.current = null;
        }
      } catch {}
    };
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
  // Debounced outward change emission refs to avoid immediate persistence echoes.
  const changeEmitTimerRef = useRef<number | null>(null);
  const lastEmittedHashRef = useRef<string | null>(null);
  // Locked large-file decision derived synchronously from session metadata.
  // Use a strict byte-size check (TextEncoder) to enforce the 5 MB rule.
  const initialLarge = session.contentTruncated ?? (session.text ? (new TextEncoder().encode(session.text).length > LARGE_FILE_BYTES) : false);
  const largeFileRef = useRef<boolean>(initialLarge);
  // Log initial decision only when debugging.
  if (initialLarge) {
    debug(`[CodeEditor] file ${session.documentId} initial large-file decision: true (session.contentTruncated=${String(session.contentTruncated)})`);
  }
  // Local container ref used for measurements only. The CustomSurface component
  // remains the single vertical scroller (its own internal container).
  const containerRef = useRef<HTMLDivElement | null>(null);

  // Sync from session to local controlled value when session identity or loadSeq changes.
  useEffect(() => {
    const sessionIdentity = `${session.documentId}::${session.loadSeq ?? 0}`;
    if (lastSessionIdRef.current !== sessionIdentity) {
      // New session or new load result -> adopt session.text deterministically.
      lastSessionIdRef.current = sessionIdentity;
      setValue(session.text ?? '');

      // Cancel any pending outbound change emission to avoid echoing this adoption.
      try {
        if (changeEmitTimerRef.current) {
          window.clearTimeout(changeEmitTimerRef.current);
          changeEmitTimerRef.current = null;
        }
      } catch {}

      // Record last emitted hash as the adopted text to avoid re-sending identical content.
      try {
        lastEmittedHashRef.current = typeof session.text === 'string' ? stableHashString(session.text) : null;
      } catch {}

      // Decide large-file for this session deterministically and persist it.
      // Lock the decision synchronously so highlight/hydration logic sees it immediately.
      const decidedLarge = session.contentTruncated ?? (session.text ? (new TextEncoder().encode(session.text).length > LARGE_FILE_BYTES) : false);
      largeFileRef.current = decidedLarge;
      if (decidedLarge) {
        console.error(`[CodeEditor] session ${session.documentId} marked large-file (locked for session)`);
        try {
          // Remove any persisted syntax for this document so stale highlights cannot be reused.
          clearDocumentSyntax(session.documentId);
          console.error(`[CodeEditor] cleared persisted syntax for large-file document ${session.documentId}`);
        } catch (e) {
          console.error(`[CodeEditor] failed to clear persisted syntax for ${session.documentId}:`, e);
        }
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session.documentId, session.loadSeq]);

  // Derived UI state
  // Start with a conservative default; prefer a previously locked session decision when present.
  // `largeFileRef` is set once per session load and prevents flip-flopping while the user edits.
  let largeFile = largeFileRef.current ?? (session.contentTruncated ?? false);

  // Highlighting: always derive from the visible text `value`.
  const displayText = value;
  const lineHeight = GUTTER_CONFIG.LINE_HEIGHT;
  const displayLineStarts = useMemo(() => computeLineStarts(displayText), [displayText]);
  const totalLines = displayLineStarts.length;

  // Final large-file decision: if not already locked by session, decide now and lock it.
  // This ensures the large-file mode remains stable for the session and cannot flip
  // while the user types.
  if (largeFileRef.current === null) {
    const decided = session.contentTruncated ?? (value.length > LARGE_FILE_CHAR_THRESHOLD || totalLines > FULL_LINES_LIMIT);
    largeFileRef.current = decided;
    largeFile = decided;
  } else {
    largeFile = largeFileRef.current;
  }

  // Ensure any persisted syntax is cleared and no syntax requests will be triggered
  // when the session is a large-file. Do this in a side-effect so we don't perform
  // synchronous side-effects during render.
  useEffect(() => {
    if (largeFile && session.documentId) {
      try {
        debug(`[CodeEditor] large-file mode enabled for ${session.documentId}`);
        // Clear any persisted frontend syntax snapshot so stale highlights can't be reused.
        clearDocumentSyntax(session.documentId);
        debug(`[CodeEditor] cleared persisted syntax for large-file ${session.documentId}`);
      } catch (e) {
        warn(`[CodeEditor] failed to clear persisted syntax for ${session.documentId}: ${String(e)}`);
      }
    }
  }, [largeFile, session.documentId]);

  // Compute overlay lines for visible area early so the highlight hook can
  // request a visible-range snapshot immediately on mount.
  const [containerHeight, setContainerHeight] = useState<number>(0);
  useLayoutEffect(() => {
    const el = containerRef.current;
    if (!el) {
      // No container yet — leave containerHeight as-is.
      return;
    }
    // Synchronously initialise container height before paint to avoid an
    // initial tiny visible window that causes incorrect visibleCount.
    setContainerHeight(el.clientHeight || 0);
    const ro = new ResizeObserver((entries) => {
      for (const e of entries) setContainerHeight(e.contentRect.height);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const scrollTopRef = useRef<number>(0);
  const scrollPersistTimer = useRef<number | null>(null);

  // Lightweight non-reactive scroll diagnostics (temporary).
  // This uses refs and a best-effort window export so we can inspect
  // scroll frequency without any React state updates on the hot path.
  const scrollStatsRef = useRef<{ events: number; lastTs: number; burstMax: number }>({ events: 0, lastTs: 0, burstMax: 0 });
  function recordScrollEvent() {
    try {
      const now = Date.now();
      const s = scrollStatsRef.current;
      s.events = (s.events || 0) + 1;
      const delta = now - (s.lastTs || now);
      // Simple burst heuristic: increments burst counter for short deltas.
      if (delta < 50) s.burstMax = Math.max(s.burstMax || 0, (s.burstMax || 0) + 1);
      s.lastTs = now;
      // Expose non-reactive runtime metric (best-effort, bounded)
      try {
        (window as any).__zaroxi_scroll_stats = {
          events: s.events,
          lastTs: s.lastTs,
          burstMax: s.burstMax,
        };
      } catch {}
    } catch {}
  }

  // Authoritative cursor line (0-based). Updated by CustomSurface via onCursorChange.
  const [cursorLine, setCursorLine] = useState<number>(0);
  // Overlay DOM node ref - we'll render highlighted HTML into this stable node.
  // Overlay refs disabled for hard-reset baseline: no overlay DOM writes or transform syncs.
  // const overlayRef = useRef<HTMLDivElement | null>(null);
  // const contentRef = useRef<HTMLDivElement | null>(null);

  // Use the real container clientHeight when available to compute visible ranges
  // synchronously during render. This prevents the initial "tiny visible window"
  // problem where containerHeight was still zero.
  const measuredContainerHeight = (containerRef.current && containerRef.current.clientHeight) || containerHeight || 0;

  // When in large-file plain-text mode we disable virtualization for correctness:
  // render the entire available document (the preview content) rather than a tiny
  // clipped window. Performance tradeoff is acceptable for correctness-first fix.
  let visibleStartLine: number;
  let visibleCount: number;
  let visibleEndLine: number;
  if (largeFile) {
    visibleStartLine = 0;
    visibleEndLine = totalLines;
    visibleCount = totalLines;
  } else {
    const scrollTop = scrollTopRef.current ?? 0;
    visibleStartLine = Math.max(0, Math.floor(scrollTop / lineHeight) - 3);
    visibleCount = Math.ceil((measuredContainerHeight + lineHeight) / lineHeight) * 2;
    visibleEndLine = Math.min(visibleStartLine + visibleCount, totalLines);
  }

  // Highlights disabled temporarily to avoid highlight/bridge hot-path while
  // root edit sync loop is being fixed. Re-enable once sync is stable.
  const highlightsEnabled = false;
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
  const uidCounterRef = useRef(0);
  const overlayHighlighted: HighlightLine[] = useMemo(() => {
    // Diagnostics: log visible range for easier debugging (debug-only).
    debug(`[CodeEditor] visible range ${visibleStartLine}..${visibleEndLine} of ${totalLines} lines`);

    // Large-file plain-text path: avoid relying on any backend highlights and
    // render the available text lines in full. This disables reuse of persisted
    // uids to ensure no stale syntax is shown.
    if (largeFile) {
      const lines: HighlightLine[] = [];
      for (let idx = 0; idx < totalLines; idx++) {
        const start = displayLineStarts[idx] ?? displayText.length;
        const end = displayLineStarts[idx + 1] ?? displayText.length;
        let authoritative = displayText.slice(start, end);
        if (authoritative.endsWith('\n')) authoritative = authoritative.slice(0, -1);
        const uid = `${session.documentId}:${stableHashString(authoritative)}:${uidCounterRef.current++}`;
        lines.push({ uid, index: idx, text: authoritative, spans: [] });
      }
      return lines;
    }

    // Normal (non-large-file) path: reuse backend-provided highlightedMap and
    // attempt to preserve prior uids to avoid remount storms.
    const lines: HighlightLine[] = [];

    const prevSession = session.documentId ? getDocumentSyntax(session.documentId) : undefined;
    const usedUids = new Set<string>();

    for (let idx = visibleStartLine; idx < visibleEndLine; idx++) {
      const start = displayLineStarts[idx] ?? displayText.length;
      const end = displayLineStarts[idx + 1] ?? displayText.length;
      let authoritative = displayText.slice(start, end);
      if (authoritative.endsWith('\n')) authoritative = authoritative.slice(0, -1);

      const backend = highlightedMap.get(idx);

      // Attempt to reuse an existing uid for this index if the prior session had
      // the same text at the same index.
      let uid: string | undefined = undefined;
      if (prevSession && prevSession.map) {
        const prevEntry = prevSession.map.get(idx);
        if (prevEntry && prevEntry.text === authoritative && !usedUids.has(prevEntry.uid)) {
          uid = prevEntry.uid;
          usedUids.add(uid);
        } else {
          // Fallback: try to find any previous entry with the same text (first unused).
          for (const [_, v] of prevSession.map) {
            if (v.text === authoritative && !usedUids.has(v.uid)) {
              uid = v.uid;
              usedUids.add(uid);
              break;
            }
          }
        }
      }

      if (!uid) {
        uid = `${session.documentId}:${stableHashString(authoritative)}:${uidCounterRef.current++}`;
        usedUids.add(uid);
      }

      if (backend && backend.text === authoritative) {
        lines.push({ uid, index: idx, text: authoritative, spans: backend.spans });
      } else {
        lines.push({ uid, index: idx, text: authoritative, spans: [] });
      }
    }
    return lines;
  }, [highlightedMap, visibleStartLine, visibleEndLine, displayLineStarts, displayText, session.documentId, largeFile, totalLines]);

  // Overlay DOM writes disabled for baseline: no-op effect to keep hook signature stable.
  useEffect(() => {
    // Intentionally empty: overlay rendering is removed in this baseline reset.
  }, [overlayHighlighted, lineHeight]);

  // Overlay transform synchronization disabled for baseline.
  // Keep this effect non-reactive to avoid referencing undefined values
  // during render/eval time. Removing reactive deps avoids ReferenceError
  // and ensures the scroll hot-path does not accidentally re-run the effect.
  useEffect(() => {
    // no-op - intentionally no reactive dependency
  }, []);

  // Always compute gutter width from the canonical totalLines so gutter stays
  // in sync with the document model even in large-file/read-only view.
  const gutterWidth = computeGutterWidth(totalLines);
  const effectiveReadOnly = readOnly || largeFile;
  const totalHeight = totalLines * lineHeight;
  const MAX_OVERLAY_HEIGHT = 10_000_000;
  // Overlay availability / readiness disabled for baseline to avoid any gating logic.
  const overlayAvailable = false;
  const overlayReady = false;

  // Handlers
  // Debounced outward onChange emitter to avoid immediate store persistence on every keystroke.
  const scheduleEmitChange = useCallback((nextText: string) => {
    try {
      if (changeEmitTimerRef.current) {
        window.clearTimeout(changeEmitTimerRef.current);
        changeEmitTimerRef.current = null;
      }
    } catch {}

    const nextHash = stableHashString(nextText);
    // If identical to last emitted, skip scheduling.
    if (lastEmittedHashRef.current === nextHash) {
      return;
    }

    // Publish a short-lived editor-origin marker so persistence layers can avoid
    // echoing this editor-originated write back into the UI. This marker carries
    // only the documentId and a compact hash (no full-text payload).
    try {
      (window as any).__zaroxi_last_editor_emit = {
        documentId: session?.documentId ?? null,
        hash: nextHash,
        ts: Date.now(),
      };
    } catch {}

    const __emit_tid = window.setTimeout(() => {
      try {
        lastEmittedHashRef.current = nextHash;
        onChange(nextText);
      } catch {}
      changeEmitTimerRef.current = null;
    }, 300) as unknown as number;
    changeEmitTimerRef.current = __emit_tid;
    try {
      const _w: any = typeof window !== 'undefined' ? (window as any) : undefined;
      if (_w) {
        _w.__zaroxi_timers = _w.__zaroxi_timers || [];
        _w.__zaroxi_timers.push({ id: __emit_tid, type: 'emit', docId: session?.documentId ?? null, ts: Date.now() });
        if (_w.__zaroxi_timers.length > 5000) _w.__zaroxi_timers.shift();
      }
    } catch {}
  }, [onChange, session?.documentId]);

  const flushPendingChange = useCallback(() => {
    try {
      if (changeEmitTimerRef.current) {
        window.clearTimeout(changeEmitTimerRef.current);
        changeEmitTimerRef.current = null;
      }
    } catch {}
  }, []);

  const handleInput = useCallback((e: React.FormEvent<HTMLDivElement>) => {
    if (effectiveReadOnly) return;
    const el = e.currentTarget as HTMLDivElement;
    // innerText preserves line breaks in a plain way; normalize CRLF to LF.
    const text = el.innerText.replace(/\r\n/g, '\n');
    setValue(text);
    scheduleEmitChange(text);
  }, [scheduleEmitChange, effectiveReadOnly]);

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

    // Record event in non-reactive diagnostics (very cheap).
    recordScrollEvent();

    // Update runtime ref for any synchronous consumers but avoid frequent rerenders.
    scrollTopRef.current = top;

    // Throttle persistence of scroll position to avoid write hot-paths during fast scrolling.
    try {
      if (scrollPersistTimer.current) {
        window.clearTimeout(scrollPersistTimer.current);
      }
      const __scroll_tid = window.setTimeout(() => {
        try {
          if (session && session.tabId) {
            // Persist lightweight snapshot to editor session store (no DOM).
            EditorSessionStore.setSnapshot(session.tabId, {
              tabId: session.tabId,
              documentId: session.documentId ?? null,
              scrollTop: top,
              lastActiveAt: Date.now(),
            } as any);
          }
        } catch {}
        scrollPersistTimer.current = null;
      }, 200) as unknown as number;
      scrollPersistTimer.current = __scroll_tid;
      try {
        const _w: any = typeof window !== 'undefined' ? (window as any) : undefined;
        if (_w) {
          _w.__zaroxi_timers = _w.__zaroxi_timers || [];
          _w.__zaroxi_timers.push({ id: __scroll_tid, type: 'scroll_persist', docId: session?.documentId ?? null, ts: Date.now() });
          if (_w.__zaroxi_timers.length > 5000) _w.__zaroxi_timers.shift();
        }
      } catch {}
    } catch {}
    // No immediate setState to avoid rerenders; UI uses the live DOM scroll.
  }, [session]);

  // Editor engine removed: using CustomSurface (custom DOM-based editor).
  useEffect(() => {
    // No-op: the CustomSurface component mounts the editor DOM and handles input.
    return () => {
      // Ensure any pending scroll persistence timers are cleared on unmount.
      try {
        if (scrollPersistTimer.current) {
          window.clearTimeout(scrollPersistTimer.current);
          scrollPersistTimer.current = null;
        }
      } catch {}
    };
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

      <div className="flex-1 relative">
        {largeFile && (
          <div className="text-muted-foreground text-xs p-1 bg-muted/80 shrink-0">
            File &gt; 5 MB – read‑only preview (first 50 000 characters shown)
          </div>
        )}

        {/* For large files we render a stable plain-text preview without overlay.
            Both gutter and content live in the same scroll container (sharedScrollRef). */}
        {largeFile ? (
          <div style={{ padding: 8 }}>
            <pre
              className="whitespace-pre font-mono p-2"
              style={{
                margin: 0,
                whiteSpace: 'pre',
                fontFamily: FONT_TOKENS.editor,
                fontSize: '0.875rem',
                lineHeight: `${lineHeight}px`,
              }}
            >
              {value}
            </pre>
          </div>
        ) : (
          <CodeMirrorEditor
            documentId={session.documentId ?? null}
            text={value}
            languageId={session.language}
            onChange={(v: string) => { setValue(v); onChange(v); }}
            readOnly={effectiveReadOnly}
          />
        )}
      </div>

      <style>{`
        .scroll-hidden::-webkit-scrollbar { display: none; }
        .scroll-hidden { -ms-overflow-style: none; scrollbar-width: none; }
      `}</style>
    </div>
  );
}
