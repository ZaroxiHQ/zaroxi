import { foldService as cmFoldService } from '@codemirror/language';
import type { EditorState, Text } from '@codemirror/state';
import { computeFoldRanges } from './treesitterBridge';

/**
 * A simple fold service that consults the Tree-sitter bridge for known ranges.
 * For Phase 1 this will return null (no folds) — Phase 3 will wire the tree results.
 *
 * We keep the adapter shape so later integration is straightforward.
 */
export function foldService(state: EditorState, lineStart: number) {
  // Phase 1: no folding
  return null;
}

/**
 * Placeholder export to show how we'd attach a fold provider later.
 * Example usage:
 *   import { foldGutter } from '@codemirror/fold';
 *   extensions.push(foldGutter(), foldServiceAdapter);
 */
export const foldServiceAdapter = cmFoldService.of((state: EditorState, pos: number) => {
  return foldService(state, pos);
});
