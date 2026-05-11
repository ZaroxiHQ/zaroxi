/**
 * Deterministic CodeMirror setup for the editor using standard CM6 language packages.
 *
 * - Exports createBaseExtensions(opts, languageExtension?, docKey?) which installs
 *   required extensions for gutters, folding UI, theme, selection, and history.
 * - No Tree-sitter state or decoration plumbing is present in this file.
 */

import { EditorView, drawSelection, highlightActiveLine, keymap, lineNumbers, highlightActiveLineGutter } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { foldGutter, syntaxHighlighting } from '@codemirror/language';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { HighlightStyle, tags as t } from '@lezer/highlight';

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
 * Deterministic modern HighlightStyle using @lezer/highlight tags.
 * We define a conservative style (cmHighlightStyle) and a strong debug style
 * (debugHighlightStyle) used temporarily to prove whether syntaxHighlighting is
 * being applied in the mounted editor.
 */
const cmHighlightStyle = HighlightStyle.define([
  { tag: t.keyword, color: 'var(--color-syntax-keyword)' },
  { tag: t.string, color: 'var(--color-syntax-string)' },
  { tag: t.comment, color: 'var(--color-syntax-comment)', fontStyle: 'italic' },
  { tag: t.number, color: 'var(--color-syntax-number)' },
  { tag: t.bool, color: 'var(--color-syntax-constant)' },
  { tag: t.null, color: 'var(--color-syntax-constant)' },
  { tag: t.typeName, color: 'var(--color-syntax-type)' },
  { tag: t.function(t.variableName), color: 'var(--color-syntax-function)' },
  { tag: t.variableName, color: 'var(--color-syntax-variable)' },
  { tag: t.propertyName, color: 'var(--color-syntax-property)' },
]);

// Strong diagnostic highlight style (very high contrast) used only for proving
// the pipeline during this debugging iteration. This intentionally vivid palette
// makes it trivial to see whether syntaxHighlighting is active.
const debugHighlightStyle = HighlightStyle.define([
  { tag: t.comment, color: '#FF00FF', fontStyle: 'italic' },    // magenta
  { tag: t.keyword, color: '#FF0000', fontWeight: 'bold' },     // red
  { tag: t.string, color: '#00FF00' },                          // green
  { tag: t.number, color: '#00FFFF' },                          // cyan
  { tag: t.typeName, color: '#FFA500' },                        // orange
  { tag: t.function(t.variableName), color: '#FFD700' },        // gold
  { tag: t.variableName, color: '#FFFFFF', background: 'rgba(0,0,0,0.06)' }, // white on faint bg
  { tag: t.propertyName, color: '#00A8FF' },                    // bright blue
]);

// Use the diagnostic style as the active highlight style during debugging so
// token application is visually obvious. Replace with cmHighlightStyle later.
const activeHighlightStyle = debugHighlightStyle;

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
    // Attach the active highlight style (diagnostic). This uses modern API:
    // syntaxHighlighting from @codemirror/language with a HighlightStyle from @lezer/highlight.
    syntaxHighlighting(activeHighlightStyle),
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
