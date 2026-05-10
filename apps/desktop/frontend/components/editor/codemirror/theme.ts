import { EditorView } from '@codemirror/view';
import { Extension } from '@codemirror/state';

/**
 * Minimal CodeMirror theme that maps a few semantic CSS vars from the app.
 * Adjust colors later to better match the app theme.
 */
export const cmBaseTheme = EditorView.theme({
  '&': {
    height: '100%',
    backgroundColor: 'var(--editor-background, #0b1220)',
    color: 'var(--editor-foreground, #dbe7ff)',
  },
  '.cm-content': {
    caretColor: 'var(--editor-caret, #ffffff)',
  },
  '.cm-activeLine': {
    backgroundColor: 'rgba(255,255,255,0.02)',
  },
  '.cm-gutters': {
    backgroundColor: 'var(--editor-gutter, #071021)',
    color: 'var(--editor-gutter-foreground, #7f8aa3)',
    border: 'none',
  },
  '.cm-lineNumbers .cm-gutterElement': {
    paddingRight: '6px',
  },
}, { dark: true });

export function themeExtension(): Extension {
  return cmBaseTheme;
}
