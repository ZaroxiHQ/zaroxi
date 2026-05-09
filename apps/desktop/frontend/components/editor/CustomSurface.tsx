import React, { useRef, useEffect, useCallback } from 'react';
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

/**
 * CustomSurface
 *
 * - Renders visible lines as DOM nodes with inline token spans (foreground coloring).
 * - Uses an off-screen, focused <textarea> to capture input including IME/composition.
 * - Updates are applied by calling onChange with the new text; the parent controls the authoritative text.
 * - Only changed lines should be patched by React reconciliation (React keying by uid).
 *
 * Note: This is a pragmatic single-surface approach. The visible DOM is the single readable
 * layer; the hidden textarea is not visible and solely used to accept input events.
 */
function CustomSurface(props: CustomSurfaceProps) {
  const { value, onChange, lines, lineHeight, totalHeight, className } = props;
  const parentRef = useRef<HTMLDivElement | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  // Keep textarea focused when the surface is clicked.
  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;
    const onClick = () => {
      if (textareaRef.current) textareaRef.current.focus();
    };
    el.addEventListener('mousedown', onClick);
    return () => el.removeEventListener('mousedown', onClick);
  }, []);

  // Sync textarea value with authoritative value to support composition/IME.
  useEffect(() => {
    if (textareaRef.current && textareaRef.current.value !== value) {
      textareaRef.current.value = value;
    }
  }, [value]);

  const handleTextareaChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const v = e.target.value;
    onChange(v);
  }, [onChange]);

  // Render visible lines with stable keys (uid) so React patches only changed lines.
  return (
    <div
      ref={parentRef}
      className={className}
      style={{
        position: 'relative',
        overflow: 'auto',
        height: '100%',
        fontFamily: FONT_TOKENS.editor,
        fontSize: '0.875rem',
        lineHeight: `${lineHeight}px`,
        whiteSpace: 'pre',
      }}
    >
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
      </div>

      {/* Hidden input for IME/composition and keyboard input.
          It is visually hidden (opacity 0) and positioned to the top-left,
          but remains focusable and receives composition events reliably. */}
      <textarea
        ref={textareaRef}
        aria-label="editor input"
        spellCheck={false}
        onChange={handleTextareaChange}
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
        defaultValue={value}
      />
    </div>
  );
