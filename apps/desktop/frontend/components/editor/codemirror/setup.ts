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
import { syntaxHighlighting, HighlightStyle } from '@codemirror/language';
import { tags } from '@lezer/highlight';

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

  // Required highlight style — start with a known-good visible palette so syntax is clearly visible.
  // Once we confirm visual correctness we can switch these to theme CSS variables.
  const highlightStyle = HighlightStyle.define([
    { tag: tags.keyword, color: '#FF6B6B', fontWeight: '600' },
    { tag: tags.function(tags.variableName), color: '#4CAF50' },
    { tag: tags.string, color: '#FFB74D' },
    { tag: tags.comment, color: '#7E8794', fontStyle: 'italic' },
    { tag: tags.typeName, color: '#5B8CFF' },
    { tag: tags.variableName, color: '#E6EAF2' },
    { tag: tags.constant(tags.variableName), color: '#FFB74D' },
    { tag: tags.number, color: '#B39DDB' },
    { tag: tags.operator, color: '#C792EA' },
    { tag: tags.punctuation, color: '#AAB2BF' },
    { tag: tags.propertyName, color: '#FFCB6B' },
    { tag: tags.macroName, color: '#C792EA' },
    { tag: tags.attributeName, color: '#F78C6C' },
    // Fallback mapping for generic identifiers and builtins
    { tag: tags.definition(tags.variableName), color: '#E6EAF2' },
    { tag: tags.builtin, color: '#FF5370' },
  ]);
  const highlightExtension = syntaxHighlighting(highlightStyle, { fallback: true });

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
    // Required syntax highlighting extension so language tokens are visibly styled.
    highlightExtension,
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
