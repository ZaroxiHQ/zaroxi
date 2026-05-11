/**
 * EditorView theme (deterministic) for the CodeMirror experiment.
 *
 * This file exports `zaroxiCodeMirrorTheme` which is a deterministic CM6 extension
 * (EditorView.theme + EditorView.baseTheme) ensuring gutters, fold markers, caret,
 * selection and token classes are visible regardless of global app CSS.
 *
 * This variant uses the centralized --color-* CSS variables emitted by the theme system.
 * Using a single variable naming convention avoids the previous mismatch that made the
 * gutter and token colors invisible.
 *
 * Required CSS variables (provided by theme-store):
 *   --color-editor-background
 *   --color-text-on-surface
 *   --color-editor-cursor
 *   --color-editor-selection
 *   --color-editor-line-highlight
 *   --color-editor-gutter-background
 *   --color-text-faint
 *   --color-text-secondary
 *   --color-syntax-keyword, --color-syntax-string, --color-syntax-comment, etc
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
        // Use the centralized --color-* variables from the theme service
        background: 'var(--color-editor-background)',
        color: 'var(--color-text-on-surface)',
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
        caretColor: 'var(--color-editor-cursor)',
        // Ensure the visible font-size applies to content
        fontFamily:
          'var(--editor-font-family, ui-monospace, SFMono-Regular, Menlo, Monaco, "Roboto Mono", "Segoe UI Mono", "Courier New", monospace)',
        fontSize: 'var(--editor-font-size, 14px)',
        lineHeight: '1.55',
        whiteSpace: 'pre',
      },
      '.cm-selectionBackground': {
        backgroundColor: 'var(--color-editor-selection)',
      },
      '.cm-activeLine': {
        backgroundColor: 'var(--color-editor-line-highlight)',
      },
      // Make the gutter background match the editor background by default.
      // Prefer explicit gutter variables if provided by the crate/theme.
      '.cm-gutters': {
        background: 'var(--color-editor-gutter-background)',
        // Use a faint text color for gutter numbers
        color: 'var(--color-text-faint)',
        borderRight: '0',
        padding: '0 6px',
        boxSizing: 'border-box',
      },
      '.cm-lineNumbers .cm-gutterElement': {
        paddingRight: '6px',
      },
      '.cm-activeLineGutter': {
        color: 'var(--color-text-secondary)',
      },
      '.cm-foldGutterElement': {
        color: 'var(--color-text-faint)',
      },
      '.cm-cursor': {
        borderLeft: '2px solid var(--color-editor-cursor)',
      },
      // Ensure fold gutter column is visible
      '.cm-foldGutter': {
        display: 'block',
      },
    },
    { dark: true },
  ),
  // Token styles scoped under .cm-content to ensure editor-local specificity.
  // These use the centralized --color-syntax-* variables produced by the theme service.
  EditorView.baseTheme({
    '.cm-content .ts-keyword': { color: 'var(--color-syntax-keyword)', fontWeight: 600 },
    '.cm-content .ts-string': { color: 'var(--color-syntax-string)' },
    '.cm-content .ts-comment': { color: 'var(--color-syntax-comment)', fontStyle: 'italic' },
    '.cm-content .ts-number': { color: 'var(--color-syntax-number)' },
    '.cm-content .ts-function': { color: 'var(--color-syntax-function)' },
    '.cm-content .ts-type': { color: 'var(--color-syntax-type)' },
    '.cm-content .ts-variable': { color: 'var(--color-syntax-variable)' },
    '.cm-content .ts-constant': { color: 'var(--color-syntax-constant)' },
    '.cm-content .ts-operator': { color: 'var(--color-syntax-operator)' },
    '.cm-content .ts-property': { color: 'var(--color-syntax-property)' },
    '.cm-content .ts-macro': { color: 'var(--color-syntax-macro)' },
    '.cm-content .ts-attribute': { color: 'var(--color-syntax-attribute)' },

    // Fallback broader selectors (in case some DOM shapes differ)
    '.cm-editor .ts-keyword': { color: 'var(--color-syntax-keyword)', fontWeight: 600 },
    '.cm-editor .ts-string': { color: 'var(--color-syntax-string)' },
    '.cm-editor .ts-comment': { color: 'var(--color-syntax-comment)', fontStyle: 'italic' },
    '.cm-editor .ts-number': { color: 'var(--color-syntax-number)' },

    // Small accessibility boosts for folded placeholder
    '.cm-content .cm-foldPlaceholder': {
      background: 'transparent',
      color: 'var(--color-text-on-surface)',
      border: 'none',
      padding: '0 4px',
    },
  }),
];
