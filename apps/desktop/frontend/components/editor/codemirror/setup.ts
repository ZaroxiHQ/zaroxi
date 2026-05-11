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
import * as lezerHighlight from '@lezer/highlight';

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
const _hl = lezerHighlight as any;
const HighlightStyleImpl = _hl.HighlightStyle ?? (_hl.default && _hl.default.HighlightStyle);
const tags = _hl.tags ?? (_hl.default && _hl.default.tags);

let cmHighlightStyle: any = null;
if (HighlightStyleImpl && tags) {
  cmHighlightStyle = HighlightStyleImpl.define([
    { tag: tags.keyword, color: 'var(--color-syntax-keyword)' },
    { tag: tags.string, color: 'var(--color-syntax-string)' },
    { tag: tags.comment, color: 'var(--color-syntax-comment)', fontStyle: 'italic' },
    { tag: tags.number, color: 'var(--color-syntax-number)' },
    { tag: tags.bool, color: 'var(--color-syntax-constant)' },
    { tag: tags.null, color: 'var(--color-syntax-constant)' },
    { tag: tags.typeName, color: 'var(--color-syntax-type)' },
    { tag: tags.function, color: 'var(--color-syntax-function)' },
    { tag: tags.variableName, color: 'var(--color-syntax-variable)' },
    { tag: tags.propertyName, color: 'var(--color-syntax-property)' },
  ]);
} else {
  // If HighlightStyle/tags are not available at runtime, leave cmHighlightStyle null.
  // The createBaseExtensions() function will conditionally skip attaching syntaxHighlighting.
  cmHighlightStyle = null;
}

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
    // Modern syntax highlighting (safe): attach a conservative HighlightStyle backed by @lezer/highlight tags.
    // If cmHighlightStyle couldn't be resolved at runtime, omit the syntaxHighlighting extension to avoid runtime import errors.
    ...(cmHighlightStyle ? [syntaxHighlighting(cmHighlightStyle)] : []),
    // Update listener
    updateListener,
  ];

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
