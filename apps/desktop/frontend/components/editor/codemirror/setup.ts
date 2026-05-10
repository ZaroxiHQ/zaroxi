/**
 * Dynamic CodeMirror setup for the experimental editor path.
 *
 * This module:
 * - dynamically imports CodeMirror packages at runtime,
 * - injects a small theme via injectCmTheme (so gutters and active-line are visible),
 * - exposes a decoration effect/field that allows external code to apply Tree-sitter
 *   decorations asynchronously by calling `applyDecorationsToView(view, specs)`.
 *
 * The implementation intentionally avoids static @codemirror/* imports to keep the
 * integration optional and to avoid Vite failing when deps are not installed.
 */

import { injectCmTheme } from './theme';
import { createFoldServiceExtension } from './folding';

type Selection = { from: number; to: number };

// Module-scoped handles populated when createBaseExtensions first runs.
let _setDecorationsEffect: any = null;
let _Decoration: any = null;
let _DecorationSet: any = null;

/**
 * Map a lightweight set of language identifiers to CodeMirror language packages.
 * Returns an extension instance or null.
 */
async function tryLanguageExtension(languageId?: string) {
  if (!languageId) return null;
  try {
    const id = (languageId || '').toLowerCase();
    if (id === 'javascript' || id === 'js' || id === 'typescript' || id === 'ts') {
      const mod = await import('@codemirror/lang-javascript');
      if (mod.javascript) {
        const isTs = id === 'typescript' || id === 'ts';
        return mod.javascript({ typescript: isTs });
      }
      return null;
    }
    if (id === 'rust') {
      const mod = await import('@codemirror/lang-rust');
      return mod.rust ? mod.rust() : null;
    }
    if (id === 'json') {
      const mod = await import('@codemirror/lang-json');
      return mod.json ? mod.json() : null;
    }
    // Add more mappings as needed.
    return null;
  } catch (err) {
    // eslint-disable-next-line no-console
    console.debug('[codemirror] language package dynamic import failed for', languageId, err);
    return null;
  }
}

/**
 * Build the base extensions for an editor instance.
 * This sets up a StateField/StateEffect pair to receive decorations applied later.
 */
export async function createBaseExtensions(
  opts: { onChange: (text: string, selection?: Selection) => void },
  languageId?: string,
  docKey?: string,
) {
  // Inject theme styles into the document (keeps styling consistent with app variables).
  injectCmTheme();

  // Build import keys to avoid static analyzers resolving them eagerly.
  const viewPkg = '@codemirror' + '/view';
  const commandsPkg = '@codemirror' + '/commands';
  const historyPkg = '@codemirror' + '/history';
  const gutterPkg = '@codemirror' + '/gutter';
  const foldPkg = '@codemirror' + '/fold';
  const statePkg = '@codemirror' + '/state';

  try {
    // Import view & other modules in parallel using literal imports so Vite can analyze them.
    // Note: @codemirror/history may not be separately available on some installs (merged into commands in newer CM versions).
    // We import the commonly-present packages and then attempt to source `history` from the commands package; if absent,
    // we provide safe no-op fallbacks so the editor still mounts.
    const [viewMod, commandsMod, gutterMod, stateMod] = await Promise.all([
      import('@codemirror/view'),
      import('@codemirror/commands'),
      import('@codemirror/gutter'),
      import('@codemirror/state'),
    ]);

    const { EditorView, Decoration, drawSelection, highlightActiveLine, highlightActiveLineGutter, keymap } =
      viewMod as any;
    const { default: defaultKeymap } = commandsMod as any;
    // history() and historyKeymap may be exported from a dedicated package or from commands in newer releases.
    const history = (commandsMod as any).history ?? (() => []);
    const historyKeymap = (commandsMod as any).historyKeymap ?? [];
    const { lineNumbers } = gutterMod as any;
    const { StateEffect, StateField } = stateMod as any;

    // Store references for applyDecorationsToView
    _Decoration = Decoration;
    _DecorationSet = Decoration;

    // Define an effect that external code can dispatch to replace the decoration set.
    _setDecorationsEffect = StateEffect.define<any>();

    const highlightField = StateField.define({
      create() {
        return (Decoration as any).set([], true);
      },
      update(deco: any, tr: any) {
        // Map decorations through document changes
        if (deco && typeof deco.map === 'function') {
          deco = deco.map(tr.changes);
        }
        for (const e of tr.effects) {
          if (e.is(_setDecorationsEffect)) {
            deco = e.value;
          }
        }
        return deco;
      },
      provide: (f: any) => EditorView.decorations.from(f),
    });

    const updateListener = (EditorView as any).updateListener.of((update: any) => {
      if (update.docChanged) {
        const text = update.state.doc.toString();
        const sel = update.state.selection.main;
        opts.onChange(text, { from: sel.from, to: sel.to });
      }
    });

    const extensions: any[] = [
      lineNumbers(),
      drawSelection ? drawSelection() : [],
      highlightActiveLine ? highlightActiveLine() : [],
      highlightActiveLineGutter ? highlightActiveLineGutter() : [],
      history(),
      keymap ? keymap.of([...defaultKeymap, ...historyKeymap]) : [],
      highlightField,
      updateListener,
    ];

    // Try adding fold gutter (if fold module is available)
    try {
      const foldMod = await import('@codemirror/fold');
      const { foldGutter } = foldMod as any;
      if (foldGutter) {
        extensions.unshift(foldGutter());
      }
    } catch {
      // ignore missing fold package
    }

    // Try to add a language extension as a pragmatic fallback while Tree-sitter is added.
    const langExt = await tryLanguageExtension(languageId);
    if (langExt) {
      extensions.push(langExt);
    }

    // Try to attach a Tree-sitter based fold service (factory returns an extension or null)
    try {
      const foldExt = await createFoldServiceExtension(docKey, languageId);
      if (foldExt) {
        extensions.push(foldExt);
      }
    } catch {
      // ignore fold service errors
    }

    return extensions;
  } catch (err) {
    // eslint-disable-next-line no-console
    console.debug('[codemirror] dynamic imports failed; CodeMirror disabled for this session.', err);
    return [];
  }
}

/**
 * Apply decoration specs (from Tree-sitter) to a mounted EditorView instance.
 * The function expects `createBaseExtensions` to have been called previously (so the
 * StateEffect has been defined). If not, the call is a no-op.
 */
export async function applyDecorationsToView(view: any, specs: { from: number; to: number; className: string }[]) {
  if (!_setDecorationsEffect || !_Decoration) return;

  try {
    const decos = specs.map((s) => _Decoration.mark({ class: s.className }).range(s.from, s.to));
    const set = (_Decoration as any).set(decos, true);
    view.dispatch({
      effects: _setDecorationsEffect.of(set),
    });
  } catch (err) {
    // eslint-disable-next-line no-console
    console.debug('[codemirror] applyDecorationsToView failed', err);
  }
}

/**
 * Create a fresh EditorState for initial mounting. This is async because it may dynamically
 * import @codemirror/state and build extension objects.
 *
 * Returns an EditorState instance.
 */
export async function createState(
  initialText: string,
  opts: { onChange: (text: string, selection?: Selection) => void },
  languageId?: string,
  docKey?: string,
) {
  const statePkg = '@codemirror' + '/state';
  try {
    const stateMod = await import('@codemirror/state');
    const extensions = await createBaseExtensions(opts, languageId, docKey);
    return (stateMod as any).EditorState.create({
      doc: initialText ?? '',
      extensions,
    });
  } catch (err) {
    // If we cannot construct an EditorState because codemirror packages are missing,
    // rethrow so the caller can fall back (e.g. to a textarea).
    throw new Error('CodeMirror state unavailable');
  }
}
