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
const appHighlightStyle = HighlightStyle.define([
  { tag: t.keyword, color: "#c792ea" },
  { tag: t.comment, color: "#676e95", fontStyle: "italic" },
  { tag: t.string, color: "#c3e88d" },
  { tag: t.number, color: "#f78c6c" },
  { tag: t.bool, color: "#ff9cac" },
  { tag: t.typeName, color: "#82aaff" },
  { tag: t.function(t.variableName), color: "#82aaff" },
  { tag: t.variableName, color: "#eeffff" },
  { tag: t.propertyName, color: "#addb67" }
]);

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
