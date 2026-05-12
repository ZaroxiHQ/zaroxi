/**
 * Deterministic CodeMirror setup for the editor using standard CM6 language packages.
 *
 * Provides a professional, explicit large-file policy with three profiles:
 * - normal: full CM6 editor with syntax highlighting and gutter
 * - large: reduced features, gutter optional, syntax optional for stability
 * - extreme: minimal, read-only viewer/editor with maximum safety
 *
 * Exports (module scope):
 *  - PROFILE_THRESHOLDS
 *  - analyzeText
 *  - decideProfile
 *  - createBaseExtensions
 *  - createState
 *
 * This refactor hoists helpers to module scope and avoids nested `export` usage
 * so bundlers like esbuild/vite accept the module. The implementation keeps
 * modern CM6 APIs and avoids deprecated imports.
 */

import { EditorView, drawSelection, highlightActiveLine, keymap, lineNumbers, highlightActiveLineGutter } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { syntaxHighlighting, HighlightStyle } from '@codemirror/language';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { tags as t } from '@lezer/highlight';

import { zaroxiCodeMirrorTheme } from './theme';

type Selection = { from: number; to: number };

/* -------------------------
   Highlight style plumbing
   ------------------------- */

/** Helper to compose nested CSS var(...) fallbacks. */
function cssVarChain(...vars: string[]) {
  const normalized = vars.map((v) => (v.startsWith('--') ? v : `--${v}`));
  return normalized.reduceRight((acc, name) => (acc ? `var(${name}, ${acc})` : `var(${name})`), '');
}

function buildSyntaxPalette() {
  const root = document.documentElement;
  let cs: CSSStyleDeclaration | null = null;
  try { cs = getComputedStyle(root); } catch {}
  const primaryText = '--color-text-on-surface';
  const mutedText = '--color-text-faint';
  const accent = '--color-accent';
  const info = '--color-info';
  const success = '--color-success';
  const warning = '--color-warning';

  const syntax = {
    keyword: cssVarChain('--color-syntax-keyword', accent, primaryText),
    function: cssVarChain('--color-syntax-function', primaryText),
    method: cssVarChain('--color-syntax-method', '--color-syntax-function', primaryText),
    string: cssVarChain('--color-syntax-string', success, primaryText),
    comment: cssVarChain('--color-syntax-comment', mutedText),
    type: cssVarChain('--color-syntax-type', info, primaryText),
    variable: cssVarChain('--color-syntax-variable', primaryText),
    constant: cssVarChain('--color-syntax-constant', '--color-syntax-string', primaryText),
    number: cssVarChain('--color-syntax-number', '--color-syntax-constant', warning, primaryText),
    operator: cssVarChain('--color-syntax-operator', primaryText),
    punctuation: cssVarChain('--color-syntax-punctuation', primaryText),
    property: cssVarChain('--color-syntax-property', primaryText),
    tag: cssVarChain('--color-syntax-tag', primaryText),
    attribute: cssVarChain('--color-syntax-attribute', primaryText),
    macro: cssVarChain('--color-syntax-macro', primaryText),
    namespace: cssVarChain('--color-syntax-namespace', primaryText),
    builtin: cssVarChain('--color-syntax-builtin', primaryText),
    parameter: cssVarChain('--color-syntax-parameter', primaryText),
    lifetime: cssVarChain('--color-syntax-lifetime', primaryText),
    regex: cssVarChain('--color-syntax-regex', primaryText),
    markupHeading: cssVarChain('--color-syntax-markup-heading', primaryText),
    markupCode: cssVarChain('--color-syntax-markup-code', primaryText),
  };

  try {
    if (cs) {
      const inspectVars = [
        '--color-syntax-keyword',
        '--color-syntax-string',
        '--color-syntax-comment',
        '--color-syntax-type',
        '--color-syntax-function',
      ];
      const missing = inspectVars.filter((v) => !cs!.getPropertyValue(v).trim());
      if (missing.length > 0) console.debug('[codemirror] missing syntax CSS vars (will use fallbacks):', missing);
    }
  } catch {}

  return syntax;
}

function buildHighlightStyle() {
  const p = buildSyntaxPalette();

  const rawStyles = [
    { tag: [t.blockComment, t.lineComment, t.comment], color: p.comment, fontStyle: 'italic' },
    { tag: [t.keyword, t.atom, t.special(t.keyword)], color: p.keyword, fontWeight: '600' },
    { tag: [t.string, t.special(t.string)], color: p.string },
    { tag: [t.regexp, t.escape], color: p.regex },
    { tag: [t.number, t.bool, t.null], color: p.number },
    { tag: [t.typeName, t.className, t.namespace], color: p.type },
    { tag: [t.function(t.variableName), t.function(t.propertyName), t.function], color: p.function },
    { tag: [t.variableName, t.name], color: p.variable },
    { tag: [t.propertyName], color: p.property },
    { tag: [t.attributeName], color: p.attribute },
    { tag: [t.labelName], color: p.property },
    { tag: [t.macroName], color: p.macro },
    { tag: [t.namespace], color: p.namespace },
    { tag: [t.special(t.variableName)], color: p.lifetime },
    { tag: [t.tagName], color: p.tag },
    { tag: [t.operator, t.punctuation], color: p.operator },
    { tag: [t.heading, t.contentSeparator], color: p.markupHeading },
    { tag: [t.special(t.propertyName), t.macroName], color: p.constant },
    { tag: t.invalid, color: p.operator },
  ];

  // Minimal, defensive sanitizer: drop any entries that lack resolvable tags.
  const isDev = (() => {
    try {
      if (typeof import.meta !== 'undefined' && (import.meta as any).env && (import.meta as any).env.MODE) {
        return (import.meta as any).env.MODE === 'development';
      }
    } catch {}
    try {
      if (typeof process !== 'undefined' && (process as any).env && (process as any).env.NODE_ENV) {
        return (process as any).env.NODE_ENV === 'development';
      }
    } catch {}
    return false;
  })();

  function sanitize(styles: any[]) {
    const sanitized: any[] = [];
    for (let i = 0; i < styles.length; i++) {
      const s = styles[i];
      const tags = Array.isArray(s.tag) ? s.tag : [s.tag];
      const valid = tags.filter((tg) => tg != null && typeof (tg as any).id !== 'undefined');
      if (valid.length === 0) {
        if (isDev) {
          try { console.warn('[codemirror] omitted invalid highlight entry', i); } catch {}
        }
        continue;
      }
      const normalized = Array.isArray(s.tag) ? valid : valid[0];
      sanitized.push({ ...s, tag: normalized });
    }
    if (sanitized.length === 0) {
      // minimal fallback
      return [
        { tag: [t.keyword], color: p.keyword, fontWeight: '600' },
        { tag: [t.string], color: p.string },
        { tag: [t.comment], color: p.comment, fontStyle: 'italic' },
      ];
    }
    return sanitized;
  }

  const safeStyles = sanitize(rawStyles);
  return HighlightStyle.define(safeStyles);
}

const appHighlightStyle = buildHighlightStyle();

/* -------------------------
   Module-scoped helpers
   ------------------------- */

// Small shared theme extension list
const common = [zaroxiCodeMirrorTheme];

// Create an update listener factory that uses the provided opts.onChange.
// This is module-scoped but produces a listener bound to the caller's onChange.
function createNormalUpdateListener(opts: { onChange: (text: string, selection?: Selection) => void }) {
  return EditorView.updateListener.of((update) => {
    if (update.docChanged) {
      try {
        const text = update.state.doc.toString();
        const sel = update.state.selection.main;
        opts.onChange(text, { from: sel.from, to: sel.to });
      } catch {
        // swallow
      }
    }
  });
}

function createMinimalLargeListener() {
  return EditorView.updateListener.of((update) => {
    if (update.docChanged) {
      // intentionally no-op to keep hot path cheap
    }
  });
}

/* -------------------------
   Large-file policy config
   ------------------------- */

/**
 * PROFILE_THRESHOLDS - tunables for file profiling.
 *
 * These defaults are conservative and can be tuned at runtime by tests or
 * admin UI. They are expressed in bytes / lines / characters.
 *
 * Rationale:
 * - Many editors use ~1MB as a reasonable threshold for full features.
 * - 5MB is a conservative threshold for "large" where we should start degrading.
 * - Very long single lines (tens of thousands of characters) are treated as pathological
 *   because they break many layout/measurement assumptions and can cause the worst crashes.
 */
export const PROFILE_THRESHOLDS = {
  normalMaxBytes: 1 * 1024 * 1024, // 1 MB
  largeMaxBytes: 5 * 1024 * 1024, // 5 MB
  normalMaxLines: 10_000,
  largeMaxLines: 100_000,
  normalMaxLineLength: 2_000,
  largeMaxLineLength: 50_000,
  extremeNoGutterLineLength: 200_000,
} as const;

export type FileMetrics = {
  bytes: number;
  lines: number;
  maxLineLength: number;
};

/**
 * analyzeText
 *
 * Measure bytes, line count, and maximum single-line length. Implemented to be
 * fast and avoid allocations when possible.
 */
export function analyzeText(s: string): FileMetrics {
  try {
    const bytes = new TextEncoder().encode(s || '').length;
    let lines = 1;
    let maxLine = 0;
    let cur = 0;
    for (let i = 0; i < s.length; i++) {
      const ch = s.charCodeAt(i);
      if (ch === 10) { // '\n'
        lines++;
        if (cur > maxLine) maxLine = cur;
        cur = 0;
      } else {
        cur++;
      }
    }
    if (cur > maxLine) maxLine = cur;
    if (s.length === 0) lines = 0;
    return { bytes, lines, maxLineLength: maxLine };
  } catch {
    return { bytes: 0, lines: 0, maxLineLength: 0 };
  }
}

/**
 * decideProfile
 *
 * Deterministically classify a document into 'normal' | 'large' | 'extreme'
 * using the PROFILE_THRESHOLDS. Long single lines are treated as first-class
 * risk and can push a file into 'extreme' even if total bytes are modest.
 */
export function decideProfile(metrics: FileMetrics): 'normal' | 'large' | 'extreme' {
  const t = PROFILE_THRESHOLDS;
  if (metrics.maxLineLength > t.largeMaxLineLength || metrics.bytes > t.largeMaxBytes * 4 || metrics.lines > t.largeMaxLines * 4) {
    return 'extreme';
  }
  if (metrics.bytes > t.largeMaxBytes || metrics.lines > t.largeMaxLines || metrics.maxLineLength > t.largeMaxLineLength) {
    return 'large';
  }
  return 'normal';
}

/* -------------------------
   Extension builders
   ------------------------- */

/**
 * normalEditorExtensions
 *
 * Full-featured CM6 editor:
 * - lineNumbers() gutter
 * - active-line + active-line-gutter for usability
 * - history, default keymaps
 * - language support (if provided)
 * - syntaxHighlighting using HighlightStyle
 */
function normalEditorExtensions(
  opts: { onChange: (text: string, selection?: Selection) => void },
  languageExtension: any,
  docKey?: string,
) {
  const highlightStyle = appHighlightStyle ?? null;
  const syntaxExt = highlightStyle ? syntaxHighlighting(highlightStyle) : null;
  return [
    ...common,
    lineNumbers(),
    highlightActiveLineGutter(),
    drawSelection(),
    highlightActiveLine(),
    history(),
    keymap.of([...defaultKeymap, ...historyKeymap]),
    ...(languageExtension ? [languageExtension] : []),
    ...(syntaxExt ? [syntaxExt] : []),
    createNormalUpdateListener(opts),
  ];
}

/**
 * largeFileExtensions
 *
 * Reduced feature set to protect performance:
 * - gutter kept if explicitly allowed (lineNumbers)
 * - avoid active-line gutter extras to reduce layout churn
 * - omit history to reduce per-change bookkeeping
 * - optionally attach language support (without forcing syntax)
 * - minimal update listener
 */
function largeFileExtensions(
  opts: { onChange: (text: string, selection?: Selection) => void },
  languageExtension: any,
  docKey?: string,
  allowSyntax = true,
  showGutter = true,
) {
  const highlightStyle = appHighlightStyle ?? null;
  const syntaxExt = highlightStyle ? syntaxHighlighting(highlightStyle) : null;
  const ext: any[] = [
    ...common,
    ...(showGutter ? [lineNumbers()] : []),
    drawSelection(),
    keymap.of(defaultKeymap),
    ...(languageExtension && allowSyntax ? [languageExtension] : []),
    ...(allowSyntax && syntaxExt ? [syntaxExt] : []),
    createMinimalLargeListener(),
  ];
  return ext;
}

/**
 * extremeFileExtensions
 *
 * Minimal, safe, preferably read-only path for pathological documents.
 * - syntax OFF
 * - editable=false (viewer) by default to avoid accidental expensive operations
 * - only lineNumbers gutter if explicitly safe
 */
function extremeFileExtensions(
  opts: { onChange: (text: string, selection?: Selection) => void },
  languageExtension: any,
  docKey?: string,
  showGutter = false,
) {
  return [
    ...common,
    ...(showGutter ? [lineNumbers()] : []),
    EditorView.editable.of(false),
    createMinimalLargeListener(),
  ];
}

/**
 * createBaseExtensions
 *
 * Module-level export that returns the chosen extension set based on an explicit
 * profile hint. This keeps the primary editor path as CM6 while allowing
 * intentional degradation for large/pathological files.
 */
export function createBaseExtensions(
  opts: { onChange: (text: string, selection?: Selection) => void },
  languageExtension?: any,
  docKey?: string,
  profile: 'normal' | 'large' | 'extreme' = 'normal',
  showGutter: boolean = true,
) {
  try {
    if (profile === 'normal') {
      return normalEditorExtensions(opts, languageExtension, docKey);
    } else if (profile === 'large') {
      const allowSyntax = true; // caller can decide to disable by not providing languageExtension
      return largeFileExtensions(opts, languageExtension, docKey, allowSyntax, showGutter);
    } else {
      return extremeFileExtensions(opts, languageExtension, docKey, showGutter);
    }
  } catch (e) {
    try { console.warn('[codemirror] failed to build extensions for profile', profile, String(e)); } catch {}
    return [
      ...common,
      EditorView.editable.of(false),
      createMinimalLargeListener(),
    ];
  }
}

/**
 * createState
 *
 * Convenience: create an EditorState for a document given an extension profile.
 */
export function createState(
  initialText: string,
  opts: { onChange: (text: string, selection?: Selection) => void },
  languageExtension?: any,
  docKey?: string,
  profile: 'normal' | 'large' | 'extreme' = 'normal',
  showGutter: boolean = true,
) {
  const extensions = createBaseExtensions(opts, languageExtension, docKey, profile, showGutter);
  return EditorState.create({
    doc: initialText ?? '',
    extensions,
  });
}
