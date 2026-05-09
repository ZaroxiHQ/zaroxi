/**
 * DIAGNOSIS
 *
 * 1) Why it renders but is not editable:
 *    - The visible DOM renders syntax, but input was only wired to a tiny hidden
 *      textarea (defaultValue + onChange) without mapping pointer positions to
 *      caret offsets or synchronising selection/caret rendering. Clicking didn't
 *      compute an index, dragging selection wasn't handled, and keyboard-driven
 *      caret updates weren't reflected in the visible surface.
 *
 * 2) Which input path is missing/broken:
 *    - The pointer -> document offset mapping and the selection/caret sync path
 *      were missing. The textarea captured raw input but never received a
 *      programmatic selection to match where the user clicked, so edits went
 *      to positions the user couldn't control.
 *
 * 3) Focus/caret/selection model:
 *    - Incomplete: there was no visible caret, no selection rendering, no drag
 *      to select, and no reliable sync between textarea.selection and the
 *      rendered overlay.
 *
 * 4) Old editor code conflicts:
 *    - Not conflicting per-se, but remnants (tiny hidden textarea + no mapping)
 *      left a dead input path that couldn't drive the custom surface UX.
 *
 * 5) Architecture I'll use to fix it:
 *    - Keep the visible DOM as the single readable surface.
 *    - Use a hidden textarea solely as the input/IME host.
 *    - Implement explicit editor state derived from `value` (lines array).
 *    - Implement pointer -> index mapping (binary search per line via canvas.measureText).
 *    - On pointer events (click/drag) compute index and set textarea selectionRange.
 *    - Listen to textarea 'input' and 'select' to sync selection/caret state and call onChange.
 *    - Render a visible caret and selection highlights computed from the authoritative text model.
 *
 * Short summary of changes:
 * - Full-featured pointer handling (click, drag) mapping to character index.
 * - Hidden textarea used only for IME/keyboard; selection synced programmatically.
 * - Visible caret and selection rendering.
 * - Kept syntax spans rendering unchanged and compatible with Tree-sitter spans.
 *
 * These changes implement PHASE 1..3 from your plan in a single coherent pass.
 */

import React, {
  useRef,
  useEffect,
  useCallback,
  useState,
  useLayoutEffect,
} from 'react';
import { FONT_TOKENS } from '@/lib/theme/font-tokens';

type HighlightSpan = {
  start: number;
  end: number;
  token_type: string;
  color?: string | null;
};
type HighlightLine = {
  uid: string;
  index: number;
  text: string;
  spans: HighlightSpan[];
};

interface CustomSurfaceProps {
  value: string;
  onChange: (value: string) => void;
  lines: HighlightLine[];
  lineHeight: number;
  totalHeight: number;
  className?: string;
}

/* -------------------------
   Utilities
   ------------------------- */
function mergeSpans(spans: HighlightSpan[], lineLen: number): HighlightSpan[] {
  if (spans.length === 0 || lineLen === 0) return [];

  const sorted = [...spans].sort((a, b) => {
    const la = a.end - a.start;
    const lb = b.end - b.start;
    if (la !== lb) return la - lb;
    const s = a.start - b.start;
    return s !== 0 ? s : a.end - b.end;
  });

  const charTokens: Array<{ tokenType: string; color?: string } | null> =
    new Array(lineLen).fill(null);

  for (const sp of sorted) {
    const tok = sp.token_type;
    const color = sp.color ?? undefined;
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

function renderSpansElements(spans: HighlightSpan[], lineText: string) {
  if (spans.length === 0) return [lineText];

  const merged = mergeSpans(spans, lineText.length);
  if (merged.length === 0) return [lineText];

  const segments: React.ReactNode[] = [];
  let last = 0;
  for (let i = 0; i < merged.length; i++) {
    const sp = merged[i];
    if (sp.start > last) {
      segments.push(lineText.slice(last, sp.start));
    }
    const key = `${sp.start}-${i}`;
    const style: React.CSSProperties | undefined = sp.color ? { color: sp.color } : undefined;
    segments.push(
      <span key={key} style={style} className={`syntax-${String(sp.token_type || 'plain').toLowerCase().replace(/[^a-z0-9_-]/g, '-')}`}>
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

/* -------------------------
   Main Component
   ------------------------- */
export default function CustomSurface(props: CustomSurfaceProps) {
  const { value, onChange, lines, lineHeight, totalHeight, className } = props;

  const containerRef = useRef<HTMLDivElement | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  // Editor state derived from `value`
  const [linesArr, setLinesArr] = useState<string[]>(() => value.split('\n'));
  useEffect(() => {
    setLinesArr(value.split('\n'));
  }, [value]);

  // Selection state (character offsets)
  const [selStart, setSelStart] = useState<number>(0);
  const [selEnd, setSelEnd] = useState<number>(0);

  // Drag state for mouse selection
  const draggingRef = useRef(false);

  // Canvas for text measurement (created eagerly in a DOM-safe way).
  // Historically this was created in a layout effect which allowed measureTextWidth
  // to run before the canvas existed. Create it eagerly when possible so callers
  // that run during render won't hit `canvasRef.current === null`.
  const canvasRef = useRef<HTMLCanvasElement | null>(
    typeof document !== 'undefined' ? document.createElement('canvas') : null
  );

  // Ensure textarea contains up-to-date value for composition/IME
  useEffect(() => {
    const t = textareaRef.current;
    if (!t) return;
    if (t.value !== value) t.value = value;
    // If selection differs, keep it in sync (do not steal focus)
    if (typeof t.selectionStart === 'number' && (t.selectionStart !== selStart || t.selectionEnd !== selEnd)) {
      try {
        t.setSelectionRange(selStart, selEnd);
      } catch {
        // ignore invalid ranges
      }
    }
  }, [value, selStart, selEnd]);

  // Helper: total chars before a line
  const lineStartCharIndex = useCallback((lineIndex: number) => {
    let total = 0;
    for (let i = 0; i < lineIndex && i < linesArr.length; i++) total += linesArr[i].length + 1;
    return total;
  }, [linesArr]);

  // Measure text width using canvas with the computed font.
  // This function is defensive: it lazily creates a canvas if missing and
  // falls back to an approximate width if the 2D context cannot be created.
  const measureTextWidth = useCallback((text: string) => {
    // Ensure a canvas exists (safe for SSR where document may be undefined)
    if (!canvasRef.current && typeof document !== 'undefined') {
      canvasRef.current = document.createElement('canvas');
    }
    const canvas = canvasRef.current;
    const container = containerRef.current;

    // Determine font spec from computed styles (fall back to sensible defaults)
    let fontSize = '14px';
    let fontFamily = FONT_TOKENS.editor;
    if (container) {
      const style = window.getComputedStyle(container);
      fontSize = style.fontSize || fontSize;
      fontFamily = style.fontFamily || fontFamily;
    }

    const fontSpec = `${fontSize} ${fontFamily}`;

    // Fallback: approximate character width when canvas or context is unavailable.
    const approxCharWidth = (parseFloat(fontSize) || 14) * 0.6;
    if (!canvas) {
      return text.length * approxCharWidth;
    }

    const ctx = canvas.getContext('2d');
    if (!ctx) {
      return text.length * approxCharWidth;
    }

    ctx.font = fontSpec;
    return ctx.measureText(text).width;
  }, []);

  // Map (x,y) relative to container -> absolute char index in document
  const posToIndex = useCallback((clientX: number, clientY: number) => {
    const container = containerRef.current;
    if (!container) return 0;
    const rect = container.getBoundingClientRect();
    const x = clientX - rect.left + container.scrollLeft;
    const y = clientY - rect.top + container.scrollTop;
    let line = Math.floor(y / lineHeight);
    line = Math.max(0, Math.min(line, linesArr.length - 1));
    const text = linesArr[line] ?? '';
    // Binary search column position by measuring text widths
    let lo = 0;
    let hi = text.length;
    while (lo < hi) {
      const mid = Math.ceil((lo + hi) / 2);
      const w = measureTextWidth(text.slice(0, mid));
      if (w < x) lo = mid;
      else hi = mid - 1;
    }
    // Adjust: if lo < len and next char still fits, increment
    while (lo < text.length && measureTextWidth(text.slice(0, lo + 1)) <= x) lo++;
    while (lo > 0 && measureTextWidth(text.slice(0, lo)) > x) lo--;
    const index = lineStartCharIndex(line) + lo;
    return Math.max(0, Math.min(index, value.length));
  }, [linesArr, lineHeight, measureTextWidth, lineStartCharIndex, value.length]);

  // Map absolute index -> {line, column, leftPx, topPx}
  const indexToCoords = useCallback((index: number) => {
    const clamped = Math.max(0, Math.min(index, value.length));
    let remaining = clamped;
    let line = 0;
    while (line < linesArr.length) {
      const l = linesArr[line].length + 1; // include newline
      if (remaining < l) break;
      remaining -= l;
      line++;
    }
    if (line >= linesArr.length) {
      line = linesArr.length - 1;
      remaining = linesArr[line].length;
    }
    const col = remaining;
    const left = measureTextWidth(linesArr[line].slice(0, col));
    const top = line * lineHeight;
    return { line, col, left, top };
  }, [linesArr, measureTextWidth, lineHeight, value.length]);

  // Update selection from textarea events
  useEffect(() => {
    const ta = textareaRef.current;
    if (!ta) return;
    const onSelect = () => {
      const a = ta.selectionStart ?? 0;
      const b = ta.selectionEnd ?? 0;
      setSelStart(a);
      setSelEnd(b);
    };
    const onInput = () => {
      const v = ta.value;
      onChange(v);
      // selection will be reflected via 'select' event, but update proactively
      const a = ta.selectionStart ?? 0;
      const b = ta.selectionEnd ?? 0;
      setSelStart(a);
      setSelEnd(b);
    };
    ta.addEventListener('select', onSelect);
    ta.addEventListener('input', onInput);
    return () => {
      ta.removeEventListener('select', onSelect);
      ta.removeEventListener('input', onInput);
    };
  }, [onChange]);

  // Pointer handlers: click to set caret, drag to select
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const onMouseDown = (e: MouseEvent) => {
      if (!(e instanceof MouseEvent)) return;
      e.preventDefault();
      const idx = posToIndex(e.clientX, e.clientY);
      draggingRef.current = true;
      setSelStart(idx);
      setSelEnd(idx);
      // focus textarea and set selection
      const ta = textareaRef.current;
      if (ta) {
        ta.focus();
        try {
          ta.setSelectionRange(idx, idx);
        } catch {}
      }
      // attach move/up listeners on document
      const onMove = (ev: MouseEvent) => {
        const to = posToIndex(ev.clientX, ev.clientY);
        setSelEnd(to);
        const ta2 = textareaRef.current;
        if (ta2) {
          try {
            ta2.setSelectionRange(Math.min(idx, to), Math.max(idx, to));
          } catch {}
        }
      };
      const onUp = (ev: MouseEvent) => {
        draggingRef.current = false;
        document.removeEventListener('mousemove', onMove);
        document.removeEventListener('mouseup', onUp);
      };
      document.addEventListener('mousemove', onMove);
      document.addEventListener('mouseup', onUp);
    };
    container.addEventListener('mousedown', onMouseDown);
    return () => container.removeEventListener('mousedown', onMouseDown);
  }, [posToIndex]);

  // Keyboard handling: keep textarea tiny and intercept Tab to insert tab
  useEffect(() => {
    const ta = textareaRef.current;
    if (!ta) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Tab') {
        e.preventDefault();
        const start = ta.selectionStart ?? 0;
        const end = ta.selectionEnd ?? 0;
        const text = ta.value;
        const newText = text.slice(0, start) + '\t' + text.slice(end);
        ta.value = newText;
        try {
          ta.setSelectionRange(start + 1, start + 1);
        } catch {}
        // Fire input handler manually
        const ev = new InputEvent('input', { bubbles: true });
        ta.dispatchEvent(ev);
      }
    };
    ta.addEventListener('keydown', onKeyDown);
    return () => ta.removeEventListener('keydown', onKeyDown);
  }, []);

  // Render caret position and selection overlays
  const caretCoords = indexToCoords(selEnd);
  const isCollapsed = selStart === selEnd;
  const selectionRanges = (() => {
    if (selStart === selEnd) return [];
    const a = Math.min(selStart, selEnd);
    const b = Math.max(selStart, selEnd);
    const start = indexToCoords(a);
    const end = indexToCoords(b);
    if (start.line === end.line) {
      return [{
        top: start.top,
        left: start.left,
        width: Math.max(0, end.left - start.left),
      }];
    }
    // Multi-line: first line from start.left -> line end, middle full lines, last line 0->end.left
    const ranges: { top: number; left: number; width: number }[] = [];
    const firstLineText = linesArr[start.line] ?? '';
    const firstLineWidth = measureTextWidth(firstLineText);
    ranges.push({ top: start.top, left: start.left, width: Math.max(0, firstLineWidth - start.left) });
    for (let ln = start.line + 1; ln < end.line; ln++) {
      const text = linesArr[ln] ?? '';
      const w = measureTextWidth(text);
      ranges.push({ top: ln * lineHeight, left: 0, width: w });
    }
    ranges.push({ top: end.top, left: 0, width: Math.max(0, end.left) });
    return ranges;
  })();

  // Scroll caret into view when changed
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const caretTop = caretCoords.top;
    const caretBottom = caretTop + lineHeight;
    if (caretTop < container.scrollTop) {
      container.scrollTop = caretTop;
    } else if (caretBottom > container.scrollTop + container.clientHeight) {
      container.scrollTop = caretBottom - container.clientHeight;
    }
  }, [caretCoords, lineHeight]);

  // Render
  return (
    <div
      ref={containerRef}
      className={className}
      style={{
        position: 'relative',
        overflow: 'auto',
        height: '100%',
        fontFamily: FONT_TOKENS.editor,
        fontSize: '0.875rem',
        lineHeight: `${lineHeight}px`,
        whiteSpace: 'pre',
        caretColor: 'transparent', // hide native caret (textarea is invisible)
        WebkitUserSelect: 'none',
      }}
    >
      {/* measurement canvas kept off-DOM */}
      {/* Visible rendered lines */}
      <div style={{ position: 'relative', height: totalHeight, width: '100%' }}>
        {lines.map((hl) => (
          <div
            key={hl.uid}
            style={{
              position: 'absolute',
              top: hl.index * lineHeight,
              left: 0,
              height: lineHeight,
              lineHeight: `${lineHeight}px`,
              whiteSpace: 'pre',
              pointerEvents: 'none',
              width: '100%',
            }}
            aria-hidden={true}
          >
            {renderSpansElements(hl.spans, hl.text)}
          </div>
        ))}

        {/* Selection overlays */}
        {selectionRanges.map((r, i) => (
          <div
            key={`sel-${i}`}
            style={{
              position: 'absolute',
              top: r.top,
              left: r.left,
              height: lineHeight,
              width: Math.max(1, r.width),
              background: 'rgba(90, 120, 200, 0.25)',
              pointerEvents: 'none',
            }}
            aria-hidden={true}
          />
        ))}

        {/* Caret */}
        {isCollapsed && (
          <div
            style={{
              position: 'absolute',
              top: caretCoords.top,
              left: caretCoords.left,
              width: 1,
              height: Math.max(1, lineHeight),
              background: '#000',
              boxShadow: '0 0 0 1px rgba(0,0,0,0.1) inset',
              pointerEvents: 'none',
              animation: 'caret-blink 1s steps(2, start) infinite',
            }}
            aria-hidden={true}
          />
        )}
      </div>

      {/* Hidden textarea for IME/keyboard. Kept tiny and invisible to avoid creating a second readable layer.
          We programmatically set selectionRange on it so input occurs at the correct document offset. */}
      <textarea
        ref={textareaRef}
        aria-label="editor input"
        spellCheck={false}
        onChange={(e) => {
          onChange(e.target.value);
          // selection update handled by select/input listeners registered in effect
        }}
        onFocus={() => {
          // sync selection when focused
          const ta = textareaRef.current;
          if (!ta) return;
          try {
            ta.setSelectionRange(selStart, selEnd);
          } catch {}
        }}
        style={{
          position: 'absolute',
          opacity: 0,
          left: 0,
          top: 0,
          width: '1px',
          height: '1px',
          zIndex: 9999,
          resize: 'none',
          pointerEvents: 'auto',
        }}
        value={value}
        // keep default handlers off; we sync selection in effects
      />
      <style>{`
        @keyframes caret-blink { 50% { opacity: 0 } 100% { opacity: 1 } }
      `}</style>
    </div>
  );
}
