/**
 * Deterministic CodeMirror setup for the editor using standard CM6 language packages.
 *
 * - Exports createBaseExtensions(opts, languageExtension?, docKey?) which installs
 *   required extensions for gutters, folding UI, theme, selection, and history.
 * - No Tree-sitter state or decoration plumbing is present in this file.
 */

import { EditorView, drawSelection, highlightActiveLine, keymap, lineNumbers, highlightActiveLineGutter } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { foldGutter } from '@codemirror/language';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';

import { zaroxiCodeMirrorTheme } from './theme';

type Selection = { from: number; to: number };

/**
 * Build the base extensions for an editor instance.
 * - opts.onChange will be called when document changes occur.
 * - languageExtension is an optional CM6 extension (LanguageSupport) to provide
 *   syntax highlighting and language-specific behavior (folding, indentation).
 *
 * Note: The editor requires a syntax highlight extension. We statically import
 * the official @codemirror/highlight API at build time and construct a HighlightStyle.
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
  // Runtime debug: report whether language support was provided and which gutter extensions are included.
  // eslint-disable-next-line no-console
  console.debug('[codemirror] createBaseExtensions', {
    docKey,
    languageProvided: !!languageExtension,
    lineNumbersIncluded: !!lineNumbersExt,
    foldGutterIncluded: !!foldGutterExt,
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
    // Syntax highlighting temporarily removed to avoid runtime crash (see EDITOR_DEBUG_NOTES).
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
