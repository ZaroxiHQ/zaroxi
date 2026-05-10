/**
 * Deterministic CodeMirror setup for the experimental editor path.
 *
 * - Uses explicit static imports for core CM6 features (gutter, folding, view, state).
 * - Provides createBaseExtensions(opts, languageId?, docKey?) which always installs the
 *   required extensions for gutters, folding UI, theme, selection, and history.
 * - Provides a simple Decoration StateField + StateEffect so Tree-sitter can later
 *   dispatch decoration updates into CM state.
 *
 * NOTE: This file implements Step 1 (visible gutter + theme). Later steps will
 * add the Tree-sitter parse scheduling and the StateEffect dispatch from the
 * parser results.
 */

import { EditorView, Decoration, drawSelection, highlightActiveLine, keymap, lineNumbers, highlightActiveLineGutter } from '@codemirror/view';
import { EditorState, StateEffect, StateField } from '@codemirror/state';
import { foldGutter } from '@codemirror/language';
import { history } from '@codemirror/commands';
import { defaultKeymap, historyKeymap } from '@codemirror/commands';

import { zaroxiCodeMirrorTheme } from './theme';
import { createFoldServiceExtension } from './folding';

type Selection = { from: number; to: number };

// Module-scoped handles populated when createBaseExtensions first runs.
let _setTreeSitterStateEffect: StateEffect<any> | null = null;
let _Decoration: typeof Decoration | null = null;

/**
 * Build the base extensions for an editor instance.
 * This deterministically installs the gutter, fold gutter, theme, selection,
 * history and a StateField for future Tree-sitter decorations/foldRanges.
 */
export function createBaseExtensions(
  opts: { onChange: (text: string, selection?: Selection) => void },
  languageId?: string,
  docKey?: string,
) {
  // Define StateEffect and StateField for Tree-sitter state (decorations + folds + version + docKey)
  if (!_setTreeSitterStateEffect) {
    _setTreeSitterStateEffect = StateEffect.define<any>();
  }
  _Decoration = Decoration;

  const treeSitterField = StateField.define<any>({
    create() {
      return {
        decorations: (_Decoration as any).set([], true),
        foldRanges: [],
        parseVersion: 0,
        docKey: docKey ?? undefined,
      };
    },
    update(value: any, tr: any) {
      // Map decoration set across document changes
      if (value && value.decorations && typeof value.decorations.map === 'function') {
        value = {
          ...value,
          decorations: value.decorations.map(tr.changes),
        };
      }

      for (const e of tr.effects) {
        if (_setTreeSitterStateEffect && e.is(_setTreeSitterStateEffect)) {
          const incoming = e.value;
          // Accept the incoming state only if parseVersion is >= current (simple guard)
          if (incoming && typeof incoming.parseVersion === 'number') {
            if (incoming.parseVersion >= (value?.parseVersion ?? 0)) {
              return incoming;
            }
            // otherwise ignore stale incoming result
            return value;
          } else {
            // if incoming is a bare DecorationSet, replace decorations only
            if (incoming && incoming.decorations) {
              return { ...value, decorations: incoming.decorations };
            }
          }
        }
      }

      return value;
    },
    provide: (f: any) => EditorView.decorations.from(f, (s: any) => s.decorations),
  });

  // Editor update listener to forward change events to the host
  const updateListener = EditorView.updateListener.of((update) => {
    if (update.docChanged) {
      const text = update.state.doc.toString();
      const sel = update.state.selection.main;
      opts.onChange(text, { from: sel.from, to: sel.to });
    }
  });

  // Compose extensions (deterministic)
  const extensions = [
    // Theme must be present to guarantee gutter visibility
    zaroxiCodeMirrorTheme,
    // Gutter + folding UI
    lineNumbers(),
    foldGutter(),
    highlightActiveLineGutter(),
    // Selection and caret
    drawSelection(),
    highlightActiveLine(),
    // History + keymaps
    history(),
    keymap.of([...defaultKeymap, ...historyKeymap]),
    // Tree-sitter field (holds decorations + foldRanges + parseVersion)
    treeSitterField,
    // Update listener
    updateListener,
  ];

  // fold service extension will be attached later when folding is implemented.

  return extensions;
}

/**
 * Apply Tree-sitter derived decorations into the EditorView by dispatching
 * a StateEffect. The payload should be an object compatible with the StateField:
 * { decorations: DecorationSet, foldRanges: [...], parseVersion: number, docKey?: string }
 *
 * This is a convenience used by the Tree-sitter bridge once available.
 */
export function applyTreeSitterStateToView(view: EditorView, payload: { decorations: any; foldRanges?: any[]; parseVersion: number; docKey?: string }) {
  if (!_setTreeSitterStateEffect) return;
  try {
    view.dispatch({
      effects: _setTreeSitterStateEffect.of(payload),
    });
  } catch (err) {
    // eslint-disable-next-line no-console
    console.debug('[codemirror] applyTreeSitterStateToView failed', err);
  }
}

/**
 * Create a fresh EditorState for initial mounting. This constructs all base
 * extensions synchronously (deterministic imports) and returns an EditorState.
 */
export function createState(
  initialText: string,
  opts: { onChange: (text: string, selection?: Selection) => void },
  languageId?: string,
  docKey?: string,
) {
  const extensions = createBaseExtensions(opts, languageId, docKey);
  return EditorState.create({
    doc: initialText ?? '',
    extensions,
  });
}

/**
 * Legacy/compat export kept for the existing editor wrapper. It will be used
 * by CodeMirrorEditor.tsx which currently expects applyDecorationsToView.
 */
export const applyDecorationsToView = async (view: any, specs: { from: number; to: number; className: string }[]) => {
  if (!_Decoration) return;
  try {
    // Basic telemetry: how many specs are we asked to apply?
    // eslint-disable-next-line no-console
    console.debug('[codemirror] applyDecorationsToView called; specs.length=', specs ? specs.length : 0);

    const decos = specs.map((s) => _Decoration!.mark({ class: s.className }).range(s.from, s.to));
    const decoSet = (_Decoration as any).set(decos, true);

    // Log the number of decoration nodes created
    // eslint-disable-next-line no-console
    console.debug('[codemirror] created decoration nodes=', decos.length, 'decoSet=', !!decoSet);

    if (!decoSet) return;

    // Dispatch into our state field
    if (_setTreeSitterStateEffect) {
      view.dispatch({
        effects: _setTreeSitterStateEffect.of({ decorations: decoSet, foldRanges: [], parseVersion: Date.now(), docKey: undefined }),
      });
      // eslint-disable-next-line no-console
      console.debug('[codemirror] dispatched tree-sitter decoration effect');
    } else {
      // eslint-disable-next-line no-console
      console.debug('[codemirror] _setTreeSitterStateEffect is not defined; cannot dispatch decorations');
    }
  } catch (err) {
    // eslint-disable-next-line no-console
    console.debug('[codemirror] applyDecorationsToView failed', err);
  }
};
