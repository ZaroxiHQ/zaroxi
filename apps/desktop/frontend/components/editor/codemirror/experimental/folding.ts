/**
 * (ARCHIVAL) Folding adapter moved to experimental directory.
 *
 * The original folding adapter that depended on Tree-sitter cached ranges is
 * stored here for reference. The active editor now relies on language-provided
 * folding and CodeMirror's foldGutter extension.
 */
export * from '../../codemirror/folding_original_fallback_not_used';
