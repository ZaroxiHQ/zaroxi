/**
 * Tree‑sitter bridge using web-tree-sitter (browser WASM).
 *
 * - Loads web-tree-sitter dynamically.
 * - Loads language wasm modules from the runtime path (you stated your runtime lives under
 *   crates/zaroxi-lang-syntax/runtime/treesitter). The loader constructs URLs like:
 *     /crates/zaroxi-lang-syntax/runtime/treesitter/tree-sitter-<lang>.wasm
 *   Adjust `getWasmUrl` if you host WASM files elsewhere.
 * - Performs full reparses (async, debounced by consumer) and returns decoration specs and fold ranges.
 *
 * NOTE: This implementation uses full reparses for correctness. Incremental parsing can be added
 * later (Phase 3) if you want better throughput.
 */

export type DecorationSpec = {
  from: number; // JS string index (UTF-16 code units)
  to: number;
  className: string;
};

export type FoldRange = { from: number; to: number };

type ParserEntry = {
  parser: any;
  language: any;
};

let inited = false;
let WTS: any = null; // web-tree-sitter module
const parsers: Map<string, ParserEntry> = new Map();

// Cache last parse results keyed by a document key (prefer documentId, fallback to text)
const lastParseCache: Map<string, { foldRanges: FoldRange[]; decorations: DecorationSpec[] }> = new Map();

/**
 * Utility: map languageId (like "rust", "typescript", "toml") to wasm filename.
 * Adjust if your wasm filenames differ.
 */
function getWasmFileNameFor(languageId: string) {
  const id = (languageId || '').toLowerCase();
  if (id === 'rust') return 'tree-sitter-rust.wasm';
  if (id === 'toml') return 'tree-sitter-toml.wasm';
  if (id === 'markdown' || id === 'md') return 'tree-sitter-markdown.wasm';
  if (id === 'typescript' || id === 'ts' || id === 'javascript' || id === 'js') return 'tree-sitter-typescript.wasm';
  // default fallback
  return `tree-sitter-${id}.wasm`;
}

/**
 * Construct a URL to the wasm file. You said your runtime is at:
 *   crates/zaroxi-lang-syntax/runtime/treesitter
 * This will attempt to load: /crates/zaroxi-lang-syntax/runtime/treesitter/<wasm>
 *
 * If you host wasm elsewhere, update this function.
 */
function getWasmUrl(languageId: string) {
  const fname = getWasmFileNameFor(languageId);
  return `/crates/zaroxi-lang-syntax/runtime/treesitter/${fname}`;
}

/**
 * Initialize web-tree-sitter once.
 */
export async function initTreesitterOnce(): Promise<void> {
  if (inited) return;
  try {
    // Import web-tree-sitter using a literal import so Vite can analyze the dependency.
    const mod = await import('web-tree-sitter');
    WTS = (mod as any).default ? (mod as any).default : mod;
    await WTS.init();
    inited = true;
  } catch (err) {
    // eslint-disable-next-line no-console
    console.error('[treesitter] failed to initialize web-tree-sitter', err);
    throw err;
  }
}

/**
 * Ensure we have a Parser for the given language.
 */
async function ensureParserFor(languageId: string) {
  if (!languageId) return null;
  const key = languageId.toLowerCase();
  if (parsers.has(key)) return parsers.get(key)!;

  if (!inited) {
    await initTreesitterOnce();
  }
  const wasmUrl = getWasmUrl(key);
  try {
    const Lang = await (WTS as any).Language.load(wasmUrl);
    const parser = new (WTS as any).Parser();
    parser.setLanguage(Lang);
    const entry: ParserEntry = { parser, language: Lang };
    parsers.set(key, entry);
    return entry;
  } catch (err) {
    // eslint-disable-next-line no-console
    console.warn('[treesitter] failed to load language wasm for', key, wasmUrl, err);
    return null;
  }
}

/**
 * Convert a (row, column) pair to a JS string index.
 * Splits text on LF and sums lengths — intentionally simple and reliable.
 */
function posToIndex(text: string, row: number, column: number) {
  if (row === 0) return column;
  const lines = text.split('\n');
  let idx = 0;
  for (let r = 0; r < row && r < lines.length; r++) {
    idx += lines[r].length + 1; // include newline
  }
  idx += column;
  return idx;
}

/**
 * Walk the tree and produce decoration specs and fold ranges.
 * This is intentionally conservative and maps basic node types to classes.
 */
function walkTreeAndCollect(node: any, text: string, decos: DecorationSpec[], folds: FoldRange[]) {
  const type: string = node.type;
  const start = node.startPosition;
  const end = node.endPosition;
  const from = posToIndex(text, start.row, start.column);
  const to = posToIndex(text, end.row, end.column);

  // Decoration heuristics - only push valid non-empty ranges.
  if (to > from) {
    // Strings & comments
    if (type.includes('string')) {
      decos.push({ from, to, className: 'ts-string' });
    } else if (type.includes('comment')) {
      decos.push({ from, to, className: 'ts-comment' });
    }
    // Numbers / constants
    else if (type === 'number' || type === 'float' || type === 'integer') {
      decos.push({ from, to, className: 'ts-number' });
    } else if (type === 'true' || type === 'false' || type === 'null') {
      decos.push({ from, to, className: 'ts-constant' });
    }
    // Functions / methods
    else if (type.includes('function') || type === 'function_definition' || type === 'function_item' || type === 'method_declaration') {
      decos.push({ from, to, className: 'ts-function' });
    }
    // Types
    else if (type.includes('type') || type === 'type_identifier') {
      decos.push({ from, to, className: 'ts-type' });
    }
    // Operators
    else if (type.includes('operator') || type === 'binary_operator' || type === 'unary_operator') {
      decos.push({ from, to, className: 'ts-operator' });
    }
    // Attributes / macros / properties
    else if (type.includes('attribute') || type === 'attribute_item' || type === 'attribute_declaration') {
      decos.push({ from, to, className: 'ts-attribute' });
    } else if (type.includes('macro')) {
      decos.push({ from, to, className: 'ts-macro' });
    } else if (type === 'property' || type === 'property_identifier' || type.endsWith('_property')) {
      decos.push({ from, to, className: 'ts-property' });
    }
    // Identifiers as variables by default
    else if (type === 'identifier' || type.endsWith('_identifier')) {
      decos.push({ from, to, className: 'ts-variable' });
    }
  }

  // Folding heuristics — common block node types
  const foldableTypes = new Set([
    'block',
    'object',
    'array',
    'function',
    'function_item',
    'function_definition',
    'impl_item',
    'struct_item',
    'enum_item',
    'module',
    'class',
    'table',
    'comment',
  ]);

  // Only add fold ranges that span more than one line and have a reasonable size
  if (foldableTypes.has(type) && end.row > start.row) {
    folds.push({ from, to });
  }

  // Recurse children
  if (node.children && node.children.length) {
    for (const c of node.children) {
      walkTreeAndCollect(c, text, decos, folds);
    }
  }
}

/**
 * Parse text with Tree‑sitter and return decoration specs.
 * Stores latest fold ranges & decorations in an in-memory cache keyed by docKey.
 *
 * docKey: a stable key identifying the document — prefer documentId; if null,
 *         fall back to the text content (not ideal for memory but OK for trial).
 */
export async function parseAndComputeDecorations(text: string, languageId: string, docKey?: string): Promise<DecorationSpec[]> {
  const key = docKey ?? text;
  const parserEntry = await ensureParserFor(languageId);
  if (!parserEntry) {
    // no parser available -> empty results
    lastParseCache.set(key, { foldRanges: [], decorations: [] });
    return [];
  }

  try {
    const tree = parserEntry.parser.parse(text);
    const decos: DecorationSpec[] = [];
    const folds: FoldRange[] = [];

    // Walk root's children
    const root = tree.rootNode;
    if (root) {
      for (const child of root.children) {
        walkTreeAndCollect(child, text, decos, folds);
      }
    }

    // Cache results
    lastParseCache.set(key, { foldRanges: folds, decorations: decos });

    return decos;
  } catch (err) {
    // eslint-disable-next-line no-console
    console.error('[treesitter] parse failed', err);
    lastParseCache.set(key, { foldRanges: [], decorations: [] });
    return [];
  }
}

/**
 * Synchronous fold-range accessor used by the foldService adapter.
 * Returns last cached fold ranges for the provided docKey (documentId preferred).
 */
export function getLastFoldRanges(docKey?: string): FoldRange[] {
  const key = docKey ?? '';
  const r = lastParseCache.get(key);
  return r ? r.foldRanges : [];
}

/**
 * Convenience: compute fold ranges on demand (async).
 */
export async function computeFoldRanges(text: string, languageId: string, docKey?: string): Promise<FoldRange[]> {
  await parseAndComputeDecorations(text, languageId, docKey);
  return getLastFoldRanges(docKey);
}
