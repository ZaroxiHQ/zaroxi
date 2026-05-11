/**
 * Tree-sitter bridge removed from main editor flow.
 *
 * The original implementation was moved to the experimental directory.
 * This placeholder remains to prevent accidental runtime usage; importers
 * should be updated to use the standard CM6 language registry instead.
 */

export async function initTreesitterOnce(): Promise<void> {
  throw new Error('Tree-sitter integration has been disabled; use CodeMirror language packages instead.');
}

export async function parseAndComputeDecorations(): Promise<any[]> {
  return [];
}

export function getLastFoldRanges(): any[] {
  return [];
}

export async function computeFoldRanges(): Promise<any[]> {
  return [];
}
