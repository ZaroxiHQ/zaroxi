/**
 * EditorView theme (deterministic) for the CodeMirror experiment.
 *
 * This file exports `zaroxiCodeMirrorTheme` which is a deterministic CM6 extension
 * (EditorView.theme + EditorView.baseTheme) ensuring gutters, fold markers, caret,
 * selection and token classes are visible regardless of global app CSS.
 *
 * Colors are intentionally high-contrast for Step 1 verification.
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
      },
      '.cm-content': {
        caretColor: 'var(--editor-caret, #ffffff)',
      },
      '.cm-selectionBackground': {
        backgroundColor: 'rgba(255,255,255,0.06)',
      },
      '.cm-activeLine': {
        backgroundColor: 'rgba(255,255,255,0.02)',
      },
      '.cm-gutters': {
        background: '#1f2937', // high contrast gutter background
        color: '#f8fafc', // bright line numbers
        borderRight: '0',
        padding: '0 6px',
        boxSizing: 'border-box',
      },
      '.cm-lineNumbers .cm-gutterElement': {
        paddingRight: '6px',
      },
      '.cm-activeLineGutter': {
        color: '#ffcc00', // obvious active gutter highlight for verification
      },
      '.cm-foldGutterElement': {
        color: '#ff6b6b', // visible fold marker color
      },
      '.cm-cursor': {
        borderLeft: '2px solid var(--editor-caret, #ffffff)',
      },
    },
    { dark: true },
  ),
  EditorView.baseTheme({
    // Token classes used by Tree-sitter decoration mapping (strong readable colors)
    '.ts-keyword': { color: '#ff7b72', fontWeight: 600 },
    '.ts-string': { color: '#9ae6b4' },
    '.ts-comment': { color: '#94a3b8', fontStyle: 'italic' },
    '.ts-number': { color: '#fbbf24' },
    '.ts-function': { color: '#60a5fa' },
    '.ts-type': { color: '#7dd3fc' },
    '.ts-variable': { color: '#dbe7ff' },
    '.ts-constant': { color: '#f78c6c' },
  }),
];
