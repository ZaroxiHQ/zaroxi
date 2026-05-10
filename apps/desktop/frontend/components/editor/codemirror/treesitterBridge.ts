/**
 * Lightweight Tree-sitter bridge (stub) for the trial.
 *
 * Phase 1: this module provides a minimal API surface so the CodeMirror wrapper can
 * call into it. Initially it returns empty decoration/folding results. In Phase 2
 * we'll wire web-tree-sitter, load WASM grammars on demand, perform (start with full)
 * reparses and return decorations + fold ranges.
 *
 * Keep the functions async to make it straightforward to swap in real parsing later.
 */

export type DecorationSpec = {
  from: number;
  to: number;
  className: string;
};

export type FoldRange = { from: number; to: number };

export async function initTreesitterOnce(): Promise<void> {
  // Placeholder: real initialization (web-tree-sitter) will go here.
  return;
}

/**
 * Parse the provided text for the given language and return decoration specs.
 * For Phase 1 this returns an empty array; Phase 2 will implement actual parsing.
 */
export async function parseAndComputeDecorations(text: string, languageId: string): Promise<DecorationSpec[]> {
  // TODO: implement full parsing and capture->decoration translation
  return [];
}

/**
 * Compute fold ranges from the current parse tree. Phase 1 returns empty array.
 */
export async function computeFoldRanges(text: string, languageId: string): Promise<FoldRange[]> {
  // TODO: implement fold range extraction from tree-sitter nodes
  return [];
}
