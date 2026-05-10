import { EditorState } from '@codemirror/state';
import { EditorView, drawSelection, highlightActiveLine, highlightActiveLineGutter } from '@codemirror/view';
import { defaultKeymap } from '@codemirror/commands';
import { history, historyKeymap } from '@codemirror/history';
import { lineNumbers } from '@codemirror/gutter';
import { keymap } from '@codemirror/view';
import { themeExtension } from './theme';

/**
 * Create a small, sensible set of CodeMirror extensions for the experiment.
 *
 * - `onChange` will be called synchronously from the update listener when the doc changes.
 */
export function createBaseExtensions(opts: { onChange: (text: string, selection?: { from: number; to: number }) => void }) {
  const updateListener = EditorView.updateListener.of((update) => {
    if (update.docChanged) {
      // Convert document to string eagerly (simple and reliable).
      const text = update.state.doc.toString();
      const sel = update.state.selection.main;
      opts.onChange(text, { from: sel.from, to: sel.to });
    }
  });

  return [
    lineNumbers(),
    drawSelection(),
    highlightActiveLine(),
    highlightActiveLineGutter(),
    history(),
    keymap.of([...defaultKeymap, ...historyKeymap]),
    themeExtension(),
    updateListener,
  ];
}

/**
 * Create a fresh EditorState for initial mounting. Consumers may cache the state object.
 */
export function createState(initialText: string, opts: { onChange: (text: string, selection?: { from: number; to: number }) => void }) {
  return EditorState.create({
    doc: initialText ?? '',
    extensions: createBaseExtensions(opts),
  });
}
