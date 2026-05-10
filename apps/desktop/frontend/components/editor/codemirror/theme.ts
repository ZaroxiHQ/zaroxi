/**
 * EditorView theme (deterministic) for the CodeMirror experiment.
 *
 * This file exports `zaroxiCodeMirrorTheme` which is a deterministic CM6 extension
 * (EditorView.theme + EditorView.baseTheme) ensuring gutters, fold markers, caret,
 * selection and token classes are visible regardless of global app CSS.
 *
 * This variant uses CSS variables provided by the surrounding app/theme (the crate)
 * to avoid hard-coded colors. It also ensures token styles are scoped with
 * `.cm-content .ts-*` selectors so they reliably apply inside the editor DOM.
 *
 * Recommended variables your crate/theme should provide (with fallbacks used below):
 *   --editor-background
 *   --editor-foreground
 *   --editor-caret
 *   --editor-selection
 *   --editor-active-line
 *   --editor-gutter-background  (falls back to --editor-background)
 *   --editor-gutter-foreground  (falls back to --editor-foreground)
 *   --editor-active-gutter-foreground
 *   --editor-fold-foreground
 *   --syntax-keyword, --syntax-string, --syntax-comment, etc (optional)
 *   --editor-font-family
 *   --editor-font-size
 */

import { EditorView } from '@codemirror/view';

export const zaroxiCodeMirrorTheme = [
  EditorView.theme(
    {
      '&': {
        height: '100%',
      },
      '.cm-editor': {
        background: 'var(--editor-background)',
        color: 'var(--editor-foreground)',
        height: '100%',
        // Reasonable default font sizing & family; crate can override via CSS variables.
        fontFamily:
          'var(--editor-font-family, ui-monospace, SFMono-Regular, Menlo, Monaco, "Roboto Mono", "Segoe UI Mono", "Courier New", monospace)',
        fontSize: 'var(--editor-font-size, 14px)',
        lineHeight: '1.55',
      },
      '.cm-scroller': {
        height: '100%',
        overflow: 'auto',
        // Smooth scrolling on supported platforms
        WebkitOverflowScrolling: 'touch',
      },
      '.cm-content': {
        caretColor: 'var(--editor-caret)',
        // Ensure the visible font-size applies to content
        fontFamily:
          'var(--editor-font-family, ui-monospace, SFMono-Regular, Menlo, Monaco, "Roboto Mono", "Segoe UI Mono", "Courier New", monospace)',
        fontSize: 'var(--editor-font-size, 14px)',
        lineHeight: '1.55',
        whiteSpace: 'pre',
      },
      '.cm-selectionBackground': {
        backgroundColor: 'var(--editor-selection)',
      },
      '.cm-activeLine': {
        backgroundColor: 'var(--editor-active-line)',
      },
      // Make the gutter background match the editor background by default.
      // Prefer an explicit gutter background variable if provided by the crate/theme.
      '.cm-gutters': {
        background: 'var(--editor-gutter-background)',
        color: 'var(--editor-gutter-foreground)',
        borderRight: '0',
        padding: '0 6px',
        boxSizing: 'border-box',
      },
      '.cm-lineNumbers .cm-gutterElement': {
        paddingRight: '6px',
      },
      '.cm-activeLineGutter': {
        color: 'var(--editor-active-gutter-foreground)',
      },
      '.cm-foldGutterElement': {
        color: 'var(--editor-fold-foreground)',
      },
      '.cm-cursor': {
        borderLeft: '2px solid var(--editor-caret)',
      },
      // Ensure fold gutter column is visible
      '.cm-foldGutter': {
        display: 'block',
      },
    },
    { dark: true },
  ),
  // Token styles scoped under .cm-content to ensure editor-local specificity.
  // Also provide fallback selectors to increase specificity in case theme layering differs.
  EditorView.baseTheme({
    '.cm-content .ts-keyword': { color: 'var(--syntax-keyword)', fontWeight: 600 },
    '.cm-content .ts-string': { color: 'var(--syntax-string)' },
    '.cm-content .ts-comment': { color: 'var(--syntax-comment)', fontStyle: 'italic' },
    '.cm-content .ts-number': { color: 'var(--syntax-number)' },
    '.cm-content .ts-function': { color: 'var(--syntax-function)' },
    '.cm-content .ts-type': { color: 'var(--syntax-type)' },
    '.cm-content .ts-variable': { color: 'var(--syntax-variable)' },
    '.cm-content .ts-constant': { color: 'var(--syntax-constant)' },
    '.cm-content .ts-operator': { color: 'var(--syntax-operator)' },
    '.cm-content .ts-property': { color: 'var(--syntax-property)' },
    '.cm-content .ts-macro': { color: 'var(--syntax-macro)' },
    '.cm-content .ts-attribute': { color: 'var(--syntax-attribute)' },

    // Fallback broader selectors (in case some DOM shapes differ)
    '.cm-editor .ts-keyword': { color: 'var(--syntax-keyword)', fontWeight: 600 },
    '.cm-editor .ts-string': { color: 'var(--syntax-string)' },
    '.cm-editor .ts-comment': { color: 'var(--syntax-comment)', fontStyle: 'italic' },
    '.cm-editor .ts-number': { color: 'var(--syntax-number)' },

    // Small accessibility boosts for folded placeholder
    '.cm-content .cm-foldPlaceholder': {
      background: 'transparent',
      color: 'var(--editor-foreground)',
      border: 'none',
      padding: '0 4px',
    },
  }),
];
