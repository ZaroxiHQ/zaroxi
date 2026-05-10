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
 *
 * This file now:
 * - Accepts an optional languageId so we can wire in the appropriate language package
 *   (CodeMirror language support) at runtime as a pragmatic fallback until Tree-sitter
 *   decorations are integrated in Phase 2.
 * - Adds fold gutter support via dynamic import of @codemirror/fold when available.
 */

import { themeExtension } from './theme';

type Selection = { from: number; to: number };

/**
 * Map a lightweight set of language identifiers to CodeMirror language packages.
 * Returns an extension instance or null.
 */
async function tryLanguageExtension(languageId?: string) {
  if (!languageId) return null;
  try {
    const id = (languageId || '').toLowerCase();
    if (id === 'javascript' || id === 'js' || id === 'typescript' || id === 'ts') {
      const pkg = '@codemirror' + '/lang-javascript';
      const mod = await import(pkg);
      // modern package exports `javascript` and `typescript` helpers
      if (mod.javascript) {
        // For TS vs JS, CodeMirror's javascript() supports typescript option
        const isTs = id === 'typescript' || id === 'ts';
        return mod.javascript({ typescript: isTs });
      }
      return null;
    }
    if (id === 'rust') {
      const pkg = '@codemirror' + '/lang-rust';
      const mod = await import(pkg);
      return mod.rust ? mod.rust() : null;
    }
    if (id === 'json') {
      const pkg = '@codemirror' + '/lang-json';
      const mod = await import(pkg);
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

export async function createBaseExtensions(
  opts: { onChange: (text: string, selection?: Selection) => void },
  languageId?: string,
) {
  // Build import keys in a way that prevents static resolvers from trying to resolve them.
  const viewPkg = '@codemirror' + '/view';
  const commandsPkg = '@codemirror' + '/commands';
  const historyPkg = '@codemirror' + '/history';
  const gutterPkg = '@codemirror' + '/gutter';
  const foldPkg = '@codemirror' + '/fold';

  try {
    const [{ EditorView, drawSelection, highlightActiveLine, highlightActiveLineGutter, keymap }, commandsMod, historyMod, gutterMod] =
      await Promise.all([import(viewPkg), import(commandsPkg), import(historyPkg), import(gutterPkg)]);

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

    const extensions: any[] = [
      lineNumbers(),
      drawSelection(),
      highlightActiveLine(),
      highlightActiveLineGutter(),
      history(),
      (keymap as any).of([...defaultKeymap, ...historyKeymap]),
      themeExtension(),
      updateListener,
    ];

    // Try to add fold gutter if available
    try {
      const foldMod = await import(foldPkg);
      const { foldGutter } = foldMod as any;
      if (foldGutter) {
        extensions.unshift(foldGutter()); // gutter near the left
      }
    } catch {
      // ignore fold gutter absence
    }

    // Try to add a language extension as a pragmatic fallback while Tree-sitter is added.
    const langExt = await tryLanguageExtension(languageId);
    if (langExt) {
      extensions.push(langExt);
    }

    return extensions;
  } catch (err) {
    // If dynamic imports fail (packages not installed), return a safe empty extension list.
    // The caller should handle the absence of a real EditorView (fallback to textarea).
    // eslint-disable-next-line no-console
    console.debug('[codemirror] dynamic imports failed; CodeMirror disabled for this session.', err);
    return [];
  }
}

/**
 * Create a fresh EditorState for initial mounting. This is async because it may dynamically
 * import @codemirror/state and build extension objects.
 */
export async function createState(
  initialText: string,
  opts: { onChange: (text: string, selection?: Selection) => void },
  languageId?: string,
) {
  const statePkg = '@codemirror' + '/state';
  try {
    const stateMod = await import(statePkg);
    const extensions = await createBaseExtensions(opts, languageId);
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
