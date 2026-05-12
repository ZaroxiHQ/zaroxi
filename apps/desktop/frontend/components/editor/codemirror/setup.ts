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
  // package that may not be resolvable in all environments. Gutter (lineNumbers)
  // continues to be provided by @codemirror/view; folding can be re-enabled by
  // adding @codemirror/fold and restoring foldGutter() usage.
  const foldExt = null;

  // TRUE minimal large-file profile: start with the smallest possible extension set.
  // This intentionally omits lineNumbers(), folding, active-line, drawSelection, history, keymaps,
  // and syntaxHighlighting to prove a stable baseline for extremely large files.
  if (largeFile) {
    // Rationale: many CM6-side costs are driven by per-line bookkeeping done by
    // gutters/folding/active-line/highlighting. For a read-only large-file viewer
    // we keep only a minimal theme and make the editor non-editable.
    return [
      ...common,
      // Make editor non-editable for stability and to avoid expensive edit paths.
      EditorView.editable.of(false),
      // Minimal no-op update listener to avoid doc serialization.
      minimalLargeListener,
    ];
  }

  // Normal (full-featured) editor extensions for typical files.
  const highlightStyle = appHighlightStyle ?? null;
  const syntaxExt = highlightStyle ? syntaxHighlighting(highlightStyle) : null;

  // Build standard featureful extension set for normal files.
  const extensions: any[] = [
    ...common,
    // Line numbers gutter (normal files)
    lineNumbers(),
    // Fold gutter only when language support is present
    ...(foldExt ? [foldExt] : []),
    highlightActiveLineGutter(),
    // Selection and caret
    drawSelection(),
    highlightActiveLine(),
    // History + keymaps
    history(),
    keymap.of([...defaultKeymap, ...historyKeymap]),
    // Language support (if provided)
    languageExtension ?? [],
    // Syntax highlighting (if HighlightStyle available)
    ...(syntaxExt ? [syntaxExt] : []),
    // Update listener (serializes full text on changes)
    normalUpdateListener,
  ];

  // Runtime diagnostics (lightweight)
  try {
    // eslint-disable-next-line no-console
    console.debug('[codemirror] createBaseExtensions assembled', {
      docKey,
      largeFile,
      extensionsCount: Array.isArray(extensions) ? extensions.length : 'unknown',
    });
  } catch {}

  return extensions;
}

/**
 * Create a fresh EditorState for initial mounting.
 */
export function createState(
  initialText: string,
  opts: { onChange: (text: string, selection?: Selection) => void },
  languageExtension?: any,
  docKey?: string,
  largeFile: boolean = false,
) {
  const extensions = createBaseExtensions(opts, languageExtension, docKey, largeFile);
  return EditorState.create({
    doc: initialText ?? '',
    extensions,
  });
}
