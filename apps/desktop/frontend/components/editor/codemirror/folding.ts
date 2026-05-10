/**
 * Folding adapter for CodeMirror using Tree-sitter cached fold ranges.
 *
 * This module exports an async factory `createFoldServiceExtension` which returns
 * a CodeMirror extension implementing the foldService. The factory dynamically
 * imports CodeMirror's `@codemirror/language` to avoid static resolution issues.
 *
 * The fold service consults `treesitterBridge.getLastFoldRanges(docKey)` synchronously,
 * so consumers must ensure `treesitterBridge.parseAndComputeDecorations` has been
 * called for the current document to populate the cache (CodeMirrorEditor will do that).
 */

import { getLastFoldRanges } from './treesitterBridge';

export async function createFoldServiceExtension(docKey?: string, languageId?: string) {
  try {
    const langMod = await import('@codemirror/language');
    const { foldService } = langMod as any;

    // Provider: given a state and position, return a fold range or null.
    const provider = (state: any, pos: number) => {
      // Attempt to use docKey (preferred) or fallback to current document text as key.
      const key = docKey ?? state.field?.docKey ?? undefined;
      const docText = state.doc.toString();
      const ranges = getLastFoldRanges(key ?? docText);

      if (!ranges || ranges.length === 0) return null;

      // Find a range that includes pos or whose start is on the same line as pos but extends further.
      // Prefer the smallest enclosing range.
      let best: { from: number; to: number } | null = null;
      for (const r of ranges) {
        if (r.from <= pos && pos <= r.to) {
          if (!best || (r.to - r.from) < (best.to - best.from)) {
            best = r;
          }
        }
      }
      if (best) return { from: best.from, to: best.to };
      return null;
    };

    return (foldService as any).of(provider);
  } catch (err) {
    // eslint-disable-next-line no-console
    console.debug('[codemirror] fold service extension unavailable', err);
    return null;
  }
}
