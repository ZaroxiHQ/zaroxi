/**
 * Deterministic CodeMirror setup for the editor using standard CM6 language packages.
 *
 * - Exports createBaseExtensions(opts, languageExtension?, docKey?) which installs
 *   required extensions for gutters, folding UI, theme, selection, and history.
 * - No Tree-sitter state or decoration plumbing is present in this file.
 */

import { EditorView, drawSelection, highlightActiveLine, keymap, lineNumbers, highlightActiveLineGutter } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { foldGutter, syntaxHighlighting, HighlightStyle } from '@codemirror/language';
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
    // Namespace and builtin tokens
    { tag: [t.namespace], color: p.namespace },
    { tag: [t.builtin], color: p.builtin },
    // Parameters (function parameters, placeholders)
    { tag: [t.parameter], color: p.parameter },
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
      if (typeof (import as any).meta !== 'undefined' && (import as any).meta.env && (import as any).meta.env.MODE) {
        return (import as any).meta.env.MODE === 'development';
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

  // Validator / sanitizer:
  // - Ensures every tag referenced is defined and has an `id` (what HighlightStyle expects).
  // - In development: log details and throw so the author can fix the mapping explicitly.
  // - In production: filter out invalid tag elements; if an entry ends up with no valid tags,
  //   the entire entry is omitted to avoid the runtime `tag.id` crash.
  function validateAndSanitize(styles: any[]) {
    const invalidEntries: Array<{ index: number; entry: any; invalidTags: any[] }> = [];

    const sanitized = styles.map((entry, idx) => {
      const tags = entry.tag;
      // Normalize to array for uniform handling
      const tagList = Array.isArray(tags) ? tags.slice() : [tags];
      const validTags = tagList.filter((tg) => {
        try {
          return tg !== undefined && tg !== null && typeof (tg as any).id !== 'undefined';
        } catch {
          return false;
        }
      });

      if (validTags.length === 0) {
        invalidEntries.push({ index: idx, entry, invalidTags: tagList });
        // Return entry with undefined tag so caller can decide to omit it
        return { ...entry, tag: undefined };
      }

      // Preserve original structure (single vs array)
      const normalizedTag = Array.isArray(tags) ? validTags : validTags[0];
      return { ...entry, tag: normalizedTag };
    });

    if (invalidEntries.length > 0) {
      // Emit detailed diagnostic to console for the developer.
      // Include index and a compact preview of the invalid tag objects.
      try {
        // eslint-disable-next-line no-console
        console.error('[codemirror] Invalid highlight tag mappings detected:', invalidEntries.map((e) => ({
          index: e.index,
          invalidTagsPreview: e.invalidTags.map((t) => {
            try {
              // Best-effort stringification without throwing
              if (!t) return String(t);
              if (typeof t === 'object') return Object.prototype.toString.call(t);
              return String(t);
            } catch {
              return '<<unserializable>>';
            }
          }),
        })));
      } catch {
        // ignore logging failures
      }

      const msg = `[codemirror] Invalid highlight tag mappings detected (${invalidEntries.length} entries). See console for details.`;

      if (isDev) {
        // Fail fast in development so invalid tag usage can be fixed explicitly.
        throw new Error(msg);
      }
      // In production environments, proceed but omit invalid entries to avoid crashing.
    }

    // Omit any entries where tag became undefined
    return sanitized.filter((e) => typeof e.tag !== 'undefined');
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
) {
  // Editor update listener to forward change events to the host
  const updateListener = EditorView.updateListener.of((update) => {
    if (update.docChanged) {
      const text = update.state.doc.toString();
      const sel = update.state.selection.main;
      opts.onChange(text, { from: sel.from, to: sel.to });
    }
  });

  // NOTE: Custom HighlightStyle removed temporarily to prevent runtime crash
  // (TypeError: undefined is not an object (evaluating 'style.tag.id')).
  // Reintroduce language token styling after the editor mounts and we verify the
  // exact tag set available from @lezer/highlight in the runtime environment.
  // highlightExtension intentionally omitted in this debug patch.

  // Compose extensions (deterministic)
  // Create specific ext instances so we can log their presence for debugging.
  const lineNumbersExt = lineNumbers();
  const foldGutterExt = languageExtension ? foldGutter() : null;
  // Runtime debug: report whether language support was provided.
  // eslint-disable-next-line no-console
  console.debug('[codemirror] createBaseExtensions', {
    docKey,
    languageProvided: !!languageExtension,
  });

  // Assemble extensions deterministically and include an explicit syntaxHighlighting
  // extension for diagnostics. We intentionally attach the `activeHighlightStyle`
  // (debug high-contrast) so we can prove token colors render.
  const extensions: any[] = [
    // Theme must be present to guarantee gutter visibility
    zaroxiCodeMirrorTheme,
    // Line numbers gutter (always show)
    lineNumbersExt,
    // Fold gutter: include only when languageExtension is provided (language support typically enables folding)
    ...(foldGutterExt ? [foldGutterExt] : []),
    highlightActiveLineGutter(),
    // Selection and caret
    drawSelection(),
    highlightActiveLine(),
    // History + keymaps
    history(),
    keymap.of([...defaultKeymap, ...historyKeymap]),
    // Language support (if provided)
    languageExtension ?? [],
    // Attach the app highlight style using the modern CM6 API:
    // syntaxHighlighting from @codemirror/language with a HighlightStyle defined above.
    syntaxHighlighting(appHighlightStyle),
    // Update listener
    updateListener,
  ];

  // Runtime diagnostics: explicit booleans to help determine whether highlighting and language are present
  const hasSyntaxHighlightingExtension = true;
  const hasLanguageExtension = !!languageExtension;
  // eslint-disable-next-line no-console
  console.debug('[codemirror] createBaseExtensions assembled', {
    docKey,
    hasSyntaxHighlightingExtension,
    hasLanguageExtension,
    extensionsCount: Array.isArray(extensions) ? extensions.length : 'unknown',
  });

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
) {
  const extensions = createBaseExtensions(opts, languageExtension, docKey);
  return EditorState.create({
    doc: initialText ?? '',
    extensions,
  });
}
