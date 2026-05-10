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
        background: 'var(--editor-background, #0b1220)',
        color: 'var(--editor-foreground, #dbe7ff)',
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
        caretColor: 'var(--editor-caret, #ffffff)',
        // Ensure the visible font-size applies to content
        fontFamily:
          'var(--editor-font-family, ui-monospace, SFMono-Regular, Menlo, Monaco, "Roboto Mono", "Segoe UI Mono", "Courier New", monospace)',
        fontSize: 'var(--editor-font-size, 14px)',
        lineHeight: '1.55',
        whiteSpace: 'pre',
      },
      '.cm-selectionBackground': {
        backgroundColor: 'var(--editor-selection, rgba(255,255,255,0.06))',
      },
      '.cm-activeLine': {
        backgroundColor: 'var(--editor-active-line, rgba(255,255,255,0.02))',
      },
      // Make the gutter background match the editor background by default.
      // Prefer an explicit gutter background variable if provided by the crate/theme.
      '.cm-gutters': {
        background: 'var(--editor-gutter-background, var(--editor-background, #0b1220))',
        color: 'var(--editor-gutter-foreground, var(--editor-foreground, #dbe7ff))',
        borderRight: '0',
        padding: '0 6px',
        boxSizing: 'border-box',
      },
      '.cm-lineNumbers .cm-gutterElement': {
        paddingRight: '6px',
      },
      '.cm-activeLineGutter': {
        color: 'var(--editor-active-gutter-foreground, #ffcc00)',
      },
      '.cm-foldGutterElement': {
        color: 'var(--editor-fold-foreground, #ff6b6b)',
      },
      '.cm-cursor': {
        borderLeft: '2px solid var(--editor-caret, #ffffff)',
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
    '.cm-content .ts-keyword': { color: 'var(--syntax-keyword, #ff7b72)', fontWeight: 600 },
    '.cm-content .ts-string': { color: 'var(--syntax-string, #9ae6b4)' },
    '.cm-content .ts-comment': { color: 'var(--syntax-comment, #94a3b8)', fontStyle: 'italic' },
    '.cm-content .ts-number': { color: 'var(--syntax-number, #fbbf24)' },
    '.cm-content .ts-function': { color: 'var(--syntax-function, #60a5fa)' },
    '.cm-content .ts-type': { color: 'var(--syntax-type, #7dd3fc)' },
    '.cm-content .ts-variable': { color: 'var(--syntax-variable, #dbe7ff)' },
    '.cm-content .ts-constant': { color: 'var(--syntax-constant, #f78c6c)' },

    // Fallback broader selectors (in case some DOM shapes differ)
    '.cm-editor .ts-keyword': { color: 'var(--syntax-keyword, #ff7b72)', fontWeight: 600 },
    '.cm-editor .ts-string': { color: 'var(--syntax-string, #9ae6b4)' },
    '.cm-editor .ts-comment': { color: 'var(--syntax-comment, #94a3b8)', fontStyle: 'italic' },
    '.cm-editor .ts-number': { color: 'var(--syntax-number, #fbbf24)' },

    // Small accessibility boosts for folded placeholder
    '.cm-content .cm-foldPlaceholder': {
      background: 'transparent',
      color: 'var(--editor-foreground, #dbe7ff)',
      border: 'none',
      padding: '0 4px',
    },
  }),
];
