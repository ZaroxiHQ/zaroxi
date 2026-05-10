/**
 * Dynamic CodeMirror setup for the experimental editor path.
 *
 * Goals:
 * - Avoid static top-level imports of `@codemirror/*` so Vite won't fail the entire build when
 *   the optional Codemirror deps are not installed yet.
 * - Perform runtime dynamic imports of packages (using a non-literal import key so build-time
 *   import analysis can't eagerly resolve them).
 * - Gracefully fall back to no-op extensions when imports fail.
 *
 * Note: `createState` is async now and consumers must await it. The CodeMirror wrapper already
 * handles this by falling back to a textarea when the dynamic imports aren't present.
 */

import { themeExtension } from './theme';

type Selection = { from: number; to: number };

export async function createBaseExtensions(opts: { onChange: (text: string, selection?: Selection) => void }) {
  // Build import keys in a way that prevents static resolvers from trying to resolve them.
  const viewPkg = '@codemirror' + '/view';
  const statePkg = '@codemirror' + '/state';
  const commandsPkg = '@codemirror' + '/commands';
  const historyPkg = '@codemirror' + '/history';
  const gutterPkg = '@codemirror' + '/gutter';

  try {
    const [{ EditorView, drawSelection, highlightActiveLine, highlightActiveLineGutter, keymap }, commandsMod, historyMod, gutterMod] = await Promise.all([
      // `import(viewPkg)` is intentionally non-literal to avoid build-time resolution when packages are absent.
      // Vite's static analysis won't try to pre-resolve these strings.
      // At runtime, if packages exist, they'll be loaded; otherwise we catch and return safe defaults.
      import(viewPkg),
      import(commandsPkg),
      import(historyPkg),
      import(gutterPkg),
    ]);

    const { default: defaultKeymap } = commandsMod as any;
    const { history, historyKeymap } = historyMod as any;
    const { lineNumbers } = gutterMod as any;

    const updateListener = (EditorView as any).updateListener.of((update: any) => {
      if (update.docChanged) {
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
      (keymap as any).of([...defaultKeymap, ...historyKeymap]),
      themeExtension(),
      updateListener,
    ];
  } catch (err) {
    // If dynamic imports fail (packages not installed), return a safe empty extension list.
    // The caller should handle the absence of a real EditorView (fallback to textarea).
    // Log at debug level to help during development.
    // eslint-disable-next-line no-console
    console.debug('[codemirror] dynamic imports failed; CodeMirror disabled for this session.', err);
    return [];
  }
}

/**
 * Create a fresh EditorState for initial mounting. This is async because it may dynamically
 * import @codemirror/state and build extension objects.
 */
export async function createState(initialText: string, opts: { onChange: (text: string, selection?: Selection) => void }) {
  const statePkg = '@codemirror' + '/state';
  try {
    const stateMod = await import(statePkg);
    const extensions = await createBaseExtensions(opts);
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
