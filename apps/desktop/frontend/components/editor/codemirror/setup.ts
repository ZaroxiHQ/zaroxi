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
import { history } from '@codemirror/commands';
import { defaultKeymap, historyKeymap } from '@codemirror/commands';

import { zaroxiCodeMirrorTheme } from './theme';

type Selection = { from: number; to: number };

/**
 * Build the base extensions for an editor instance.
 * - opts.onChange will be called when document changes occur.
 * - languageExtension is an optional CM6 extension (LanguageSupport) to provide
 *   syntax highlighting and language-specific behavior (folding, indentation).
 *
 * Note: We dynamically import the optional @codemirror/highlight package at runtime
 * to avoid Vite import-analysis errors when the package is not present in certain
 * environments. If the package is available, we attach a working syntaxHighlighting
 * using defaultHighlightStyle as a proof-of-life highlight. If not available, we
 * silently skip it (editor falls back to CSS token classes in theme).
 */
export async function createBaseExtensions(
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

  // Attempt to dynamically import the highlight helper while avoiding Vite's static
  // import-analysis. We use an eval-based import with a composed string so Vite does
  // not see a literal "@codemirror/highlight" in the source and fail pre-transform.
  let highlightExtension: any = null;
  try {
    const pkg = '@codemirror/highlight';
    // Use eval-based dynamic import to avoid Vite static analysis of literal import specifiers.
    // eslint-disable-next-line no-eval
    const mod = await eval("import('" + pkg + "')");
    const { syntaxHighlighting, defaultHighlightStyle } = mod as any;
    if (typeof syntaxHighlighting === 'function' && defaultHighlightStyle) {
      highlightExtension = syntaxHighlighting(defaultHighlightStyle, { fallback: true });
    }
  } catch (err) {
    // eslint-disable-next-line no-console
    console.debug('[codemirror] optional @codemirror/highlight import failed; skipping syntaxHighlighting', err);
    highlightExtension = null;
  }

  // Compose extensions (deterministic)
  const extensions: any[] = [
    // Theme must be present to guarantee gutter visibility
    zaroxiCodeMirrorTheme,
    // Gutter + folding UI (language-provided folding will integrate)
    lineNumbers(),
    foldGutter(),
    highlightActiveLineGutter(),
    // Selection and caret
    drawSelection(),
    highlightActiveLine(),
    // History + keymaps
    history(),
    keymap.of([...defaultKeymap, ...historyKeymap]),
    // Language support (if provided)
    languageExtension ?? [],
    // Attach highlight extension only if we successfully imported it.
    ...(highlightExtension ? [highlightExtension] : []),
    // Update listener
    updateListener,
  ];

  return extensions;
}

/**
 * Create a fresh EditorState for initial mounting.
 */
export async function createState(
  initialText: string,
  opts: { onChange: (text: string, selection?: Selection) => void },
  languageExtension?: any,
  docKey?: string,
) {
  const extensions = await createBaseExtensions(opts, languageExtension, docKey);
  return EditorState.create({
    doc: initialText ?? '',
    extensions,
  });
}
