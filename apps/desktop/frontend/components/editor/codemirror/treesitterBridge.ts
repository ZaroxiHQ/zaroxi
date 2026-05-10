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
 * Construct a URL to the wasm file.
 *
 * We attempt to be more resilient about where per-language wasm files are placed.
 * Packaging can place grammars under OS-specific subdirectories (e.g.
 * crates/zaroxi-lang-syntax/runtime/treesitter/grammars/linux-x86_64/tree-sitter-<lang>.wasm).
 *
 * getRuntimeBaseCandidates() returns an ordered list of base paths to try,
 * preferring OS-specific grammar/language subdirs. getWasmUrl() returns the
 * highest-priority candidate (the caller still probes many candidates further).
 */
function getWasmUrl(languageId: string) {
  const fname = getWasmFileNameFor(languageId);
  const candidates = getRuntimeBaseCandidates();
  // Return the most likely candidate (the caller will probe other candidates as well).
  return `${candidates[0]}${fname}`;
}

/**
 * Build an ordered list of runtime base path candidates where language wasm files
 * might live. Prefers OS-specific subdirectories (e.g. grammars/linux-x86_64/).
 */
function getRuntimeBaseCandidates(): string[] {
  const base = '/crates/zaroxi-lang-syntax/runtime/treesitter/';
  const candidates: string[] = [];

  // Always start with the canonical runtime root
  candidates.push(base);

  // Attempt to detect platform/arch from navigator; keep it conservative and produce
  // several plausible folder names so we match common packaging layouts.
  const ua = typeof navigator !== 'undefined' ? (navigator.userAgent || '') : '';
  const plat = typeof navigator !== 'undefined' ? (navigator.platform || '') : '';
  const lower = (ua + ' ' + plat).toLowerCase();

  const osVariants: string[] = [];
  if (/windows/i.test(lower) || /win/i.test(lower)) {
    osVariants.push('windows-x86_64', 'windows');
  } else if (/macintosh|mac os x|darwin|mac/i.test(lower)) {
    // try to detect arm vs intel; be forgiving
    if (/arm|aarch64|arm64/i.test(lower) || /applewebkit.*macintosh.*arm64/i.test(lower)) {
      osVariants.push('macos-aarch64', 'macos-arm64', 'macos');
    } else {
      osVariants.push('macos-x86_64', 'macos');
    }
  } else {
    // assume linux-like
    if (/aarch64|arm64|arm/i.test(lower)) {
      osVariants.push('linux-aarch64', 'linux-arm64', 'linux');
    } else {
      osVariants.push('linux-x86_64', 'linux');
    }
  }

  for (const v of osVariants) {
    candidates.push(`${base}grammars/${v}/`);
    candidates.push(`${base}languages/${v}/`);
    candidates.push(`${base}${v}/`);
  }

  // Common non-OS-specific subdirs (preserve previous behavior)
  candidates.push(`${base}languages/`);
  candidates.push(`${base}grammars/`);
  candidates.push('crates/zaroxi-lang-syntax/runtime/treesitter/');
  candidates.push('crates/zaroxi-lang-syntax/runtime/treesitter/languages/');
  candidates.push('crates/zaroxi-lang-syntax/runtime/treesitter/grammars/');
  candidates.push('languages/');
  candidates.push('grammars/');
  candidates.push('');

  // Deduplicate while preserving order
  const seen = new Set<string>();
  const uniq: string[] = [];
  for (const c of candidates) {
    if (!seen.has(c)) {
      seen.add(c);
      uniq.push(c);
    }
  }
  return uniq;
}

/**
 * Initialize web-tree-sitter once.
 */
export async function initTreesitterOnce(): Promise<void> {
  if (inited) {
    // eslint-disable-next-line no-console
    console.debug('[treesitter] initTreesitterOnce: already initialized');
    return;
  }
  try {
    // eslint-disable-next-line no-console
    console.debug('[treesitter] initTreesitterOnce: importing web-tree-sitter');
    const mod = await import('web-tree-sitter');
    WTS = (mod as any).default ? (mod as any).default : mod;

    // Base runtime path we expect in the repo. This is used to form candidate URLs.
    const runtimeBase = '/crates/zaroxi-lang-syntax/runtime/treesitter/';

    // Try to proactively fetch the engine wasm and validate it. If successful we will
    // expose it to web-tree-sitter via a stable object URL so the engine is loaded
    // from the correct bytes (avoids inadvertently loading HTML/index fallback).
    let engineObjectUrl: string | null = null;

    // Candidate locations to try for the engine wasm (ordered).
    const engineCandidates = [
      runtimeBase + 'tree-sitter.wasm',
      '/node_modules/web-tree-sitter/tree-sitter.wasm',
      '/web-tree-sitter/tree-sitter.wasm',
      '/node_modules/web-tree-sitter/dist/tree-sitter.wasm',
      'https://unpkg.com/web-tree-sitter/tree-sitter.wasm',
    ];

    // Probe candidates until we find a valid wasm blob (magic bytes "\0asm")
    for (const cand of engineCandidates) {
      try {
        // eslint-disable-next-line no-console
        console.debug('[treesitter] initTreesitterOnce: probing engine wasm candidate ->', cand);
        const resp = await fetch(cand, { method: 'GET' });
        // eslint-disable-next-line no-console
        console.debug('[treesitter] initTreesitterOnce: probe status=', resp.status, 'content-type=', resp.headers.get('content-type'), 'url=', cand);

        if (!resp.ok) {
          // eslint-disable-next-line no-console
          console.debug('[treesitter] initTreesitterOnce: probe non-ok ->', cand);
          continue;
        }

        // Read as ArrayBuffer and check magic bytes
        const buf = await resp.arrayBuffer();
        const header = new Uint8Array(buf.slice(0, 4));
        const isWasm = header.length === 4 && header[0] === 0x00 && header[1] === 0x61 && header[2] === 0x73 && header[3] === 0x6d;
        if (isWasm) {
          engineObjectUrl = URL.createObjectURL(new Blob([buf], { type: 'application/wasm' }));
          // eslint-disable-next-line no-console
          console.debug('[treesitter] initTreesitterOnce: found valid engine wasm at', cand, 'created object URL');
          break;
        } else {
          // eslint-disable-next-line no-console
          console.debug('[treesitter] initTreesitterOnce: candidate did not contain wasm magic bytes ->', cand);
          continue;
        }
      } catch (e) {
        // eslint-disable-next-line no-console
        console.debug('[treesitter] initTreesitterOnce: probe failed for candidate', cand, e);
        continue;
      }
    }

    if (!engineObjectUrl) {
      // eslint-disable-next-line no-console
      console.warn('[treesitter] initTreesitterOnce: no validated engine wasm found in candidates; web-tree-sitter may still try to load from locateFile. Ensure tree-sitter.wasm exists under crates/zaroxi-lang-syntax/runtime/treesitter or run `npm run prepare-wasm` in apps/desktop.');
    }

    // Provide locateFile so web-tree-sitter can find its runtime wasm.
    // If we created an object URL for a validated engine wasm, return that for 'tree-sitter.wasm'.
    // Otherwise, fall back to returning the runtimeBase + file path so the server middleware can serve it.
    // eslint-disable-next-line no-console
    console.debug('[treesitter] initTreesitterOnce: calling WTS.init with locateFile ->', runtimeBase);
    await WTS.init({
      locateFile: (file: string) => {
        const candidate = runtimeBase + file;
        // eslint-disable-next-line no-console
        console.debug('[treesitter] locateFile requested:', file, '->', candidate);
        if (file === 'tree-sitter.wasm' && engineObjectUrl) {
          // Use object URL to guarantee correct bytes / MIME type for the engine
          // (keeps the object URL alive so web-tree-sitter can fetch it).
          return engineObjectUrl;
        }
        // Last-resort: return the canonical runtime path so server middleware can serve it.
        return candidate;
      },
    });

    inited = true;
    // eslint-disable-next-line no-console
    console.debug('[treesitter] initTreesitterOnce: initialized successfully');
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
    // eslint-disable-next-line no-console
    console.debug('[treesitter] ensureParserFor: initializing WTS before loading parser for', key);
    await initTreesitterOnce();
  }

  // Candidate URLs to try (cover several common layouts under your runtime directory).
  // We try multiple filename permutations and both absolute and relative paths so the dev server
  // or packaged app can serve the wasm regardless of exact layout.
  const rawName = getWasmFileNameFor(key);
  const altNames = [
    rawName,
    `${key}.wasm`,
    `language-${key}.wasm`,
    `tree-sitter-${key}.wasm`,
  ];
  // Use runtime base candidates builder so we include OS-specific grammar directories
  // (e.g. grammars/linux-x86_64/) before falling back to more generic locations.
  const basePaths = getRuntimeBaseCandidates();

  const candidates: string[] = [];
  for (const bp of basePaths) {
    for (const n of altNames) {
      // Avoid duplicate slashes in concatenation
      const url = bp.endsWith('/') || bp === '' ? `${bp}${n}` : `${bp}/${n}`;
      candidates.push(url);
    }
  }

  // Also include the convenience getWasmUrl result first (preserves prior behavior)
  candidates.unshift(getWasmUrl(key)); // highest priority

  for (const url of candidates) {
    try {
      // eslint-disable-next-line no-console
      console.debug('[treesitter] ensureParserFor: trying wasm url', url);

      // Probe the URL first so we can log status and content-type (helps diagnose MIME/404 issues).
      const resp = await fetch(url, { method: 'GET' });
      // eslint-disable-next-line no-console
      console.debug('[treesitter] ensureParserFor: probe response', url, 'status=', resp.status, 'content-type=', resp.headers.get('content-type'));

      if (!resp.ok) {
        // Try next candidate
        // eslint-disable-next-line no-console
        console.debug('[treesitter] ensureParserFor: probe returned non-ok, trying next candidate', url);
        continue;
      }

      // Read the response body as ArrayBuffer so we can safely sniff the first bytes.
      // Some dev servers may return HTML (index fallback) while still responding 200,
      // which would otherwise cause Language.load(url) to attempt to parse non-wasm.
      let ab: ArrayBuffer | null = null;
      try {
        ab = await resp.arrayBuffer();
      } catch (err) {
        // eslint-disable-next-line no-console
        console.debug('[treesitter] ensureParserFor: failed to read arrayBuffer for', url, err);
        continue;
      }

      // Quick magic-number check for WebAssembly ("\0asm")
      const header = new Uint8Array(ab.slice(0, 4));
      const isWasmMagic = header.length === 4 && header[0] === 0x00 && header[1] === 0x61 && header[2] === 0x73 && header[3] === 0x6d;

      if (!isWasmMagic) {
        // If the probe returned something that isn't wasm, skip it to avoid trying to parse HTML.
        // eslint-disable-next-line no-console
        console.debug('[treesitter] ensureParserFor: probe content at', url, 'did not contain wasm magic bytes; skipping candidate');
        continue;
      }

      // At this point we have wasm-like bytes; attempt to load from the ArrayBuffer first.
      try {
        const Lang = await (WTS as any).Language.load(ab);
        const parser = new (WTS as any).Parser();
        parser.setLanguage(Lang);
        const entry: ParserEntry = { parser, language: Lang };
        parsers.set(key, entry);
        // eslint-disable-next-line no-console
        console.debug('[treesitter] ensureParserFor: loaded parser for', key, 'from arrayBuffer', url);
        return entry;
      } catch (err) {
        // eslint-disable-next-line no-console
        console.debug('[treesitter] ensureParserFor: Language.load(arrayBuffer) failed for', url, err);

        // As a last resort, only try Language.load(url) if the server advertises a wasm-like content-type.
        const ct = (resp.headers.get('content-type') || '').toLowerCase();
        if (ct.includes('application/wasm') || ct.includes('application/octet-stream') || ct === '') {
          try {
            const Lang = await (WTS as any).Language.load(url);
            const parser = new (WTS as any).Parser();
            parser.setLanguage(Lang);
            const entry: ParserEntry = { parser, language: Lang };
            parsers.set(key, entry);
            // eslint-disable-next-line no-console
            console.debug('[treesitter] ensureParserFor: loaded parser for', key, 'from url', url);
            return entry;
          } catch (err2) {
            // eslint-disable-next-line no-console
            console.warn('[treesitter] ensureParserFor: Language.load(url) failed for', url, err2);
            // try next candidate
          }
        } else {
          // eslint-disable-next-line no-console
          console.debug('[treesitter] ensureParserFor: skipping Language.load(url) because content-type is not wasm:', ct);
        }
      }
    } catch (err) {
      // Network/probe failure, try next candidate
      // eslint-disable-next-line no-console
      console.debug('[treesitter] ensureParserFor: fetch/probe failed for', url, err);
      continue;
    }
  }

  // If we reach here, none of the candidates worked.
  // eslint-disable-next-line no-console
  console.warn('[treesitter] ensureParserFor: failed to locate/load wasm for', key, 'candidates=', candidates);
  return null;
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
  // eslint-disable-next-line no-console
  console.debug('[treesitter] parseAndComputeDecorations: key=', !!docKey, 'docKey=', docKey, 'languageId=', languageId);

  const parserEntry = await ensureParserFor(languageId);
  if (!parserEntry) {
    // No parser available -> empty results (cache for stability)
    // eslint-disable-next-line no-console
    console.debug('[treesitter] parseAndComputeDecorations: no parser available for', languageId);
    lastParseCache.set(key, { foldRanges: [], decorations: [] });
    return [];
  }

  try {
    // eslint-disable-next-line no-console
    console.debug('[treesitter] parseAndComputeDecorations: parsing text (len=', text ? text.length : 0, ')');
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

    // eslint-disable-next-line no-console
    console.debug('[treesitter] parseAndComputeDecorations: parsed; decorations=', decos.length, 'folds=', folds.length);

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
