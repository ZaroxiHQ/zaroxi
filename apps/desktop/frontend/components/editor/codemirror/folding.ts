/**
 * Folding adapter removed from active editor flow.
 *
 * The active editor now uses CodeMirror's built-in foldGutter + language-provided folding.
 * This placeholder prevents accidental runtime usage; archival copy placed under experimental/.
 */

export async function createFoldServiceExtension(): Promise<null> {
  // Folding via Tree-sitter is no longer used. Return null so callers can fall back to
  // the default CodeMirror folding behavior.
  return null;
}
