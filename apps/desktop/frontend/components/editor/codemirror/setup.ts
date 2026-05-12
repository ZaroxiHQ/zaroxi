/**
 * Deterministic CodeMirror setup for the editor using standard CM6 language packages.
 *
 * - Exports createBaseExtensions(opts, languageExtension?, docKey?) which installs
 *   required extensions for gutters, folding UI, theme, selection, and history.
 * - No Tree-sitter state or decoration plumbing is present in this file.
 */

import { EditorView, drawSelection, highlightActiveLine, keymap, lineNumbers, highlightActiveLineGutter } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { syntaxHighlighting, HighlightStyle } from '@codemirror/language';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { tags as t } from '@lezer/highlight';

import { zaroxiCodeMirrorTheme } from './theme';

type Selection = { from: number; to: number };

/**
 * Conservative HighlightStyle for modern CM6 using @lezer/highlight tags.
 * Colors reference CSS variables provided by the centralized theme system so
 * token colors remain consistent with the rest of the UI.
 *
 * This style intentionally maps a small, safe set of tags to CSS vars and
 * avoids any uncertain or custom tags to prevent runtime crashes.
 *
 * NOTE: Some bundler/resolution combos may not expose named exports as expected.
 * To avoid the runtime "Importing binding name 'HighlightStyle' is not found"
 * error, resolve the runtime exports safely from a namespace import and fall
 * back to omitting syntaxHighlighting when HighlightStyle is not available.
 */
/**
 * Application HighlightStyle using modern CM6 APIs.
 *
 * This HighlightStyle is defined using `HighlightStyle.define` from
 * `@codemirror/language` and tags from `@lezer/highlight`. It uses the
 * requested diagnostic palette to ensure token colors are visibly distinct.
 */
/**
 * Build syntax color expressions using existing CSS custom properties.
 *
 * Strategy:
 * - Prefer explicit --color-syntax-* variables already emitted by theme-store.
 * - For resilience, build a nested var(...) fallback chain that falls back to a semantic
 *   theme variable (e.g. --color-accent or --color-text-on-surface) if a syntax variable
 *   is not provided. No hardcoded hex values are used in the final HighlightStyle.
 *
 * Example resulting expression:
 *   var(--color-syntax-keyword, var(--color-accent, var(--color-text-on-surface)))
 *
 * This approach allows the browser to update token colors automatically when the theme's
 * CSS variables change; no explicit editor reconfiguration is required.
 */

/** Helper: compose a nested var(...) expression from a priority list of CSS custom properties. */
function cssVarChain(...vars: string[]) {
  // Accept inputs both with and without leading "--". Normalize to "--name".
  const normalized = vars.map((v) => (v.startsWith('--') ? v : `--${v}`));
  // Build nested var expressions: var(--a, var(--b, var(--c)))
  return normalized.reduceRight((acc, name) => (acc ? `var(${name}, ${acc})` : `var(${name})`), '');
}

/** Build a small syntax palette expressed as CSS var(...) expressions.
 *  We still use getComputedStyle for a non-blocking diagnostic check but we use
 *  var(...) expressions for the colors so theme switching remains reactive.
 */
function buildSyntaxPalette() {
  const root = document.documentElement;
  let cs: CSSStyleDeclaration | null = null;
  try {
    cs = getComputedStyle(root);
  } catch {
    // getComputedStyle may fail in some unusual test environments; ignore.
  }

  // Primary semantic fallbacks from the theme system
  const primaryText = '--color-text-on-surface';
  const mutedText = '--color-text-faint';
  const accent = '--color-accent';
  const info = '--color-info';
  const success = '--color-success';
  const warning = '--color-warning';

  // Syntax variables already provided by theme-store (use them first)
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
    // Additional semantic groups to improve coverage (macros, namespaces, builtins, parameters, lifetimes)
    macro: cssVarChain('--color-syntax-macro', primaryText),
    namespace: cssVarChain('--color-syntax-namespace', primaryText),
    builtin: cssVarChain('--color-syntax-builtin', primaryText),
    parameter: cssVarChain('--color-syntax-parameter', primaryText),
    lifetime: cssVarChain('--color-syntax-lifetime', primaryText),
    regex: cssVarChain('--color-syntax-regex', primaryText),
    markupHeading: cssVarChain('--color-syntax-markup-heading', primaryText),
    markupCode: cssVarChain('--color-syntax-markup-code', primaryText),
  };

  // Diagnostic: log when a top-level syntax CSS variable is missing (non-blocking)
  if (cs) {
    try {
      const inspectVars = [
        '--color-syntax-keyword',
        '--color-syntax-string',
        '--color-syntax-comment',
        '--color-syntax-type',
        '--color-syntax-function',
      ];
      const missing = inspectVars.filter((v) => !cs!.getPropertyValue(v).trim());
      // eslint-disable-next-line no-console
      if (missing.length > 0) console.debug('[codemirror] missing syntax CSS vars (will use fallbacks):', missing);
    } catch {
      // ignore
    }
  }

  return syntax;
}

/** Build the HighlightStyle using CSS var expressions (no hardcoded hex). */
function buildHighlightStyle() {
  const p = buildSyntaxPalette();

  // Raw style entries — same structure as before, validated below.
  const rawStyles = [
    // Comments
    { tag: [t.blockComment, t.lineComment, t.comment], color: p.comment, fontStyle: 'italic' },

    // Keywords & control
    { tag: [t.keyword, t.atom, t.special(t.keyword)], color: p.keyword, fontWeight: '600' },

    // Strings & regex
    { tag: [t.string, t.special(t.string)], color: p.string },
    { tag: [t.regexp, t.escape], color: p.regex },

    // Numbers / constants
    { tag: [t.number, t.bool, t.null], color: p.number },

    // Types / classes / namespaces
    { tag: [t.typeName, t.className, t.namespace], color: p.type },

    // Functions, methods, and function calls
    { tag: [t.function(t.variableName), t.function(t.propertyName), t.function], color: p.function },

    // Variables, names
    { tag: [t.variableName, t.name], color: p.variable },

    // Properties and attributes
    { tag: [t.propertyName], color: p.property },
    { tag: [t.attributeName], color: p.attribute },
    // Labels/keys (e.g., TOML table keys and headers)
    { tag: [t.labelName], color: p.property },
    // Macros (Rust macros and macro invocations)
    { tag: [t.macroName], color: p.macro },
    // Namespace tokens
    { tag: [t.namespace], color: p.namespace },
    // Lifetimes and special variable-like tokens (Rust lifetimes often treated as special identifiers)
    { tag: [t.special(t.variableName)], color: p.lifetime },

    // Tags (HTML, XML)
    { tag: [t.tagName], color: p.tag },

    // Operators and punctuation
    { tag: [t.operator, t.punctuation], color: p.operator },

    // Markup tokens (headings, code blocks)
    { tag: [t.heading, t.contentSeparator], color: p.markupHeading },
    { tag: [t.special(t.propertyName), t.macroName], color: p.constant },

    // Invalid tokens
    { tag: t.invalid, color: p.operator },
  ];

  // Determine whether we're in development mode (Vite's import.meta.env.MODE or NODE_ENV).
  const isDev = (() => {
    try {
      // Vite provides import.meta.env.MODE; fallback to process.env.NODE_ENV if available.
      // Use a robust check that won't throw in environments where `import.meta` is not inspectable.
      // eslint-disable-next-line no-undef
      if (typeof import.meta !== 'undefined' && (import.meta as any).env && (import.meta as any).env.MODE) {
        return (import.meta as any).env.MODE === 'development';
      }
    } catch (e) {
      // ignore
    }
    try {
      // eslint-disable-next-line no-undef
      if (typeof process !== 'undefined' && (process as any).env && (process as any).env.NODE_ENV) {
        return (process as any).env.NODE_ENV === 'development';
      }
    } catch (e) {
      // ignore
    }
    return false;
  })();

  // Validator / sanitizer (non-fatal) — SURGICAL FIX:
  // - NEVER throw during module initialization.
  // - Log detailed diagnostics for invalid entries.
  // - Additionally, print a focused pre-sanitize inspection for indexes 13 and 14
  //   (these were reported as the source of the `tag.id` crash).
  // - Omit invalid entries and return a minimal safe fallback if nothing remains.
  function validateAndSanitize(styles: any[]) {
    const invalidEntries: Array<{
      index: number;
      entry: any;
      tagExpressions: string[];
      resolvedTags: Array<string | undefined>;
    }> = [];

    const sanitized: any[] = [];

    // Helper: best-effort string for a tag expression
    function stringifyTag(t: any) {
      try {
        if (t === undefined) return 'undefined';
        if (t === null) return 'null';
        if (typeof t === 'object' && typeof (t as any).id !== 'undefined') {
          return `tag.id=${(t as any).id}`;
        }
        if (typeof t === 'function') {
          return `function:${t.name || '<anonymous>'}`;
        }
        return String(t);
      } catch {
        return '<<unserializable>>';
      }
    }

    // Pre-sanitize focused inspection removed to avoid verbose developer-only logs.

    // Core sanitize pass: collect valid tags and record invalid entries
    styles.forEach((entry, idx) => {
      const tags = entry.tag;
      const tagList = Array.isArray(tags) ? tags.slice() : [tags];
      const resolvedTags = tagList.map((tg) => {
        try {
          return tg !== undefined && tg !== null && typeof (tg as any).id !== 'undefined' ? String((tg as any).id) : undefined;
        } catch {
          return undefined;
        }
      });
      const validTags = tagList.filter((tg) => {
        try {
          return tg !== undefined && tg !== null && typeof (tg as any).id !== 'undefined';
        } catch {
          return false;
        }
      });

      if (validTags.length === 0) {
        invalidEntries.push({
          index: idx,
          entry,
          tagExpressions: tagList.map(stringifyTag),
          resolvedTags,
        });
        return; // omit
      }

      const normalizedTag = Array.isArray(tags) ? validTags : validTags[0];
      sanitized.push({ ...entry, tag: normalizedTag });
    });

    // Emit concise diagnostic only in development to avoid log spam.
    if (invalidEntries.length > 0 && isDev) {
      try {
        // eslint-disable-next-line no-console
        console.warn('[codemirror] Some highlight style entries omitted:', invalidEntries.length);
      } catch {}
    }

    // If nothing valid remains, return a minimal safe fallback (guaranteed-valid tags).
    if (sanitized.length === 0) {
      // eslint-disable-next-line no-console
      console.warn('[codemirror] All highlight mappings omitted; using minimal safe fallback to avoid startup failure.');
      const fallback = [
        { tag: [t.keyword], color: p.keyword, fontWeight: '600' },
        { tag: [t.string], color: p.string },
        { tag: [t.comment], color: p.comment, fontStyle: 'italic' },
        { tag: [t.number], color: p.number },
        { tag: [t.typeName], color: p.type },
        { tag: [t.function(t.variableName)], color: p.function },
      ];
      return fallback;
    }

    return sanitized;
  }

  const safeStyles = validateAndSanitize(rawStyles);

  return HighlightStyle.define(safeStyles);
}

/** The app highlight style used by the editor. Built from CSS variables (no hardcoded hex). */
const appHighlightStyle = buildHighlightStyle();

/**
 * Build the base extensions for an editor instance.
 * - opts.onChange will be called when document changes occur.
 * - languageExtension is an optional CM6 extension (LanguageSupport) to provide
 *   syntax highlighting and language-specific behavior (folding, indentation).
 *
 * Note: The editor requires a syntax highlight extension. We attach a conservative
 * HighlightStyle (cmHighlightStyle) using @codemirror/language's syntaxHighlighting.
 * createBaseExtensions is synchronous now (no runtime dynamic imports) to keep the
 * extension graph deterministic and avoid HMR/import-analysis surprises.
 */
export function createBaseExtensions(
  opts: { onChange: (text: string, selection?: Selection) => void },
  languageExtension?: any,
  docKey?: string,
  largeFile: boolean = false,
) {
  // Normal update listener that serializes full doc when document changes.
  // Only attached in the normal (non-large-file) path because doc.toString()
  // can be expensive on very large documents.
  const normalUpdateListener = EditorView.updateListener.of((update) => {
    if (update.docChanged) {
      try {
        const text = update.state.doc.toString();
        const sel = update.state.selection.main;
        opts.onChange(text, { from: sel.from, to: sel.to });
      } catch {
        // Swallow to avoid bubbling errors into editor core.
      }
    }
  });

  // Minimal listener for large-file preview: do not serialize the full document.
  const minimalLargeListener = EditorView.updateListener.of((update) => {
    if (update.docChanged) {
      // intentionally no-op to keep hot path cheap
    }
  });

  // Common minimal theme so the editor renders with app visuals.
  const common = [zaroxiCodeMirrorTheme];

  // Folding is intentionally omitted here to avoid a hard dependency on a separate
  // package that may not be resolvable in all environments. We provide an explicit,
  // professional large-file policy below that builds three clear extension profiles:
  //
  // - NORMAL: full-featured editor with gutter, language support, and syntax highlighting.
  // - LARGE: reduced feature set; gutter kept, syntax optional, expensive listeners disabled.
  // - EXTREME: minimal stable viewer/editor; syntax OFF, minimal extensions, editable=false by default.
  //
  // The following helpers measure file characteristics at runtime and choose a profile.
  // This keeps the primary path on CM6 while ensuring stable behavior for pathological files.

  // Default tunable thresholds (easy to override during testing).
  export const PROFILE_THRESHOLDS = {
    // bytes thresholds (approximate)
    normalMaxBytes: 1 * 1024 * 1024, // 1 MB
    largeMaxBytes: 5 * 1024 * 1024, // 5 MB
    // line count thresholds
    normalMaxLines: 10_000,
    largeMaxLines: 100_000,
    // max single-line length thresholds
    normalMaxLineLength: 2_000,
    largeMaxLineLength: 50_000,
    // when max line length exceeds this, gutter may be disabled for safety
    extremeNoGutterLineLength: 200_000,
  } as const;

  export type FileMetrics = {
    bytes: number;
    lines: number;
    maxLineLength: number;
  };

  // Analyze the provided text and return byte size, line count, and max line length.
  export function analyzeText(s: string): FileMetrics {
    try {
      const bytes = new TextEncoder().encode(s || '').length;
      // Count lines and compute max line length efficiently.
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

  // Profile discriminator: returns 'normal' | 'large' | 'extreme' based on conservative thresholds.
  export function decideProfile(metrics: FileMetrics): 'normal' | 'large' | 'extreme' {
    const t = PROFILE_THRESHOLDS;
    // Pathological single-line files are the worst case; treat long single lines as extreme.
    if (metrics.maxLineLength > t.largeMaxLineLength || metrics.bytes > t.largeMaxBytes * 4 || metrics.lines > t.largeMaxLines * 4) {
      return 'extreme';
    }
    // Large files by size/lines/long lines
    if (metrics.bytes > t.largeMaxBytes || metrics.lines > t.largeMaxLines || metrics.maxLineLength > t.largeMaxLineLength) {
      return 'large';
    }
    // Otherwise normal
    return 'normal';
  }

  // Three explicit extension set builders.
  function normalEditorExtensions(opts: { onChange: (text: string, selection?: Selection) => void }, languageExtension: any, docKey?: string) {
    const highlightStyle = appHighlightStyle ?? null;
    const syntaxExt = highlightStyle ? syntaxHighlighting(highlightStyle) : null;
    return [
      ...common,
      lineNumbers(),
      highlightActiveLineGutter(),
      // Selection and caret
      drawSelection(),
      highlightActiveLine(),
      // History + keymaps
      history(),
      keymap.of([...defaultKeymap, ...historyKeymap]),
      // Language support if provided
      languageExtension ?? [],
      // Syntax highlighting (if HighlightStyle available)
      ...(syntaxExt ? [syntaxExt] : []),
      // Full-featured update listener
      normalUpdateListener,
    ];
  }

  function largeFileExtensions(opts: { onChange: (text: string, selection?: Selection) => void }, languageExtension: any, docKey?: string, allowSyntax = true, showGutter = true) {
    const highlightStyle = appHighlightStyle ?? null;
    const syntaxExt = highlightStyle ? syntaxHighlighting(highlightStyle) : null;
    const ext: any[] = [
      ...common,
      // Keep gutter when safe
      ...(showGutter ? [lineNumbers()] : []),
      // Minimal selection rendering; avoid active-line gutter extras
      drawSelection(),
      // Minimal keymap (omit history to reduce per-change bookkeeping)
      keymap.of(defaultKeymap),
      // Optional language support (for language-aware features like indentation) but no guaranteed syntax
      ...(languageExtension && allowSyntax ? [languageExtension] : []),
      // Only attach syntaxHighlighting when explicitly allowed and available
      ...(allowSyntax && syntaxExt ? [syntaxExt] : []),
      // Minimal listener to avoid expensive full-document serialization
      minimalLargeListener,
    ];
    return ext;
  }

  function extremeFileExtensions(opts: { onChange: (text: string, selection?: Selection) => void }, languageExtension: any, docKey?: string, showGutter = false) {
    // Extreme profile: minimal, safe, and preferably read-only.
    return [
      ...common,
      ...(showGutter ? [lineNumbers()] : []),
      EditorView.editable.of(false),
      // Very small/no-op listener
      minimalLargeListener,
    ];
  }

  /**
   * createBaseExtensions now accepts a profile hint and an explicit showGutter flag.
   * - profile: 'normal' | 'large' | 'extreme'
   * - showGutter: boolean (for extreme cases where gutter itself is unsafe)
   */
  export function createBaseExtensions(
    opts: { onChange: (text: string, selection?: Selection) => void },
    languageExtension?: any,
    docKey?: string,
    profile: 'normal' | 'large' | 'extreme' = 'normal',
    showGutter: boolean = true,
  ) {
    // Decide and return the proper extension set based on profile.
    try {
      if (profile === 'normal') {
        return normalEditorExtensions(opts, languageExtension, docKey);
      } else if (profile === 'large') {
        // For large files, prefer to keep syntax off unless the caller explicitly allows it.
        // The caller (CodeMirrorEditor) can decide allowSyntax based on measured metrics.
        const allowSyntax = true; // caller may request to disable by passing null languageExtension
        return largeFileExtensions(opts, languageExtension, docKey, allowSyntax, showGutter);
      } else {
        // extreme
        return extremeFileExtensions(opts, languageExtension, docKey, showGutter);
      }
    } catch (e) {
      // Fallback to a minimal stable baseline if something unexpected happens.
      try { console.warn('[codemirror] failed to build extensions for profile', profile, String(e)); } catch {}
      return [
        ...common,
        EditorView.editable.of(false),
        minimalLargeListener,
      ];
    }
  }

/**
 * Create a fresh EditorState for initial mounting.
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
