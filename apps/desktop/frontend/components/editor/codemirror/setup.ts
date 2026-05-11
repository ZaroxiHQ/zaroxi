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

  // Required highlight style — map tokens to theme CSS variables (no hard-coded colors).
  // This ensures token colors follow the centralized theme-store (--color-syntax-*) values.
  const highlightStyle = HighlightStyle.define([
    { tag: tags.keyword, color: 'var(--color-syntax-keyword)', fontWeight: '600' },
    { tag: tags.function(tags.variableName), color: 'var(--color-syntax-function)' },
    { tag: tags.string, color: 'var(--color-syntax-string)' },
    { tag: tags.comment, color: 'var(--color-syntax-comment)', fontStyle: 'italic' },
    { tag: tags.typeName, color: 'var(--color-syntax-type)' },
    { tag: tags.variableName, color: 'var(--color-syntax-variable)' },
    { tag: tags.constant(tags.variableName), color: 'var(--color-syntax-constant)' },
    { tag: tags.number, color: 'var(--color-syntax-number)' },
    { tag: tags.operator, color: 'var(--color-syntax-operator)' },
    { tag: tags.punctuation, color: 'var(--color-syntax-punctuation)' },
    { tag: tags.propertyName, color: 'var(--color-syntax-property)' },
    { tag: tags.macroName, color: 'var(--color-syntax-macro)' },
    { tag: tags.attributeName, color: 'var(--color-syntax-attribute)' },
    // Fallback mapping for generic identifiers and builtins
    { tag: tags.definition(tags.variableName), color: 'var(--color-syntax-variable)' },
    { tag: tags.builtin, color: 'var(--color-syntax-builtin)' },
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
