/**
 * Language registry and lazy loaders for CodeMirror language support.
 *
 * Responsibilities:
 * - Detect language id from a file path or filename hint.
 * - Lazily import and return a CodeMirror language support extension when available.
 * - Provide a single entry `getLanguageSupportForPath` used by the editor wrapper.
 *
 * Notes:
 * - If a specific language package is not available at runtime, the loader returns null
 *   and the editor will fall back to plain text mode.
 * - Dynamic imports reduce initial bundle size and only load language packages when a file
 *   of that type is opened.
 */

import type { Extension } from '@codemirror/state';

export type LangId =
  | 'rust'
  | 'toml'
  | 'typescript'
  | 'javascript'
  | 'json'
  | 'markdown'
  | 'html'
  | 'css'
  | 'yaml'
  | 'shell'
  | 'dockerfile'
  | 'plaintext'
  | string;

/**
 * Basic filename heuristics + extension map to normalized LangId.
 */
export function detectLanguageFromPath(path?: string | null, languageHint?: string | null): LangId {
  if (!path && languageHint) {
    return normalizeHint(languageHint);
  }
  if (!path) return 'plaintext';

  const name = path.split('/').pop() || path;
  const lower = name.toLowerCase();

  // Special filename heuristics
  if (lower === 'cargo.toml') return 'toml';
  if (lower === 'dockerfile' || lower.startsWith('dockerfile')) return 'dockerfile';
  if (lower === 'makefile') return 'plaintext';
  if (lower === '.gitignore' || lower === 'gitignore') return 'plaintext';
  if (lower === 'editorconfig') return 'plaintext';

  // Try extension
  const seg = name.split('.');
  if (seg.length <= 1) {
    // No extension - fallback to hint or plaintext
    return languageHint ? normalizeHint(languageHint) : 'plaintext';
  }
  const ext = seg[seg.length - 1];

  switch (ext) {
    case 'rs':
      return 'rust';
    case 'toml':
      return 'toml';
    case 'ts':
      return 'typescript';
    case 'tsx':
      return 'typescript';
    case 'js':
      return 'javascript';
    case 'jsx':
      return 'javascript';
    case 'mjs':
    case 'cjs':
      return 'javascript';
    case 'json':
      return 'json';
    case 'md':
    case 'markdown':
      return 'markdown';
    case 'html':
    case 'htm':
      return 'html';
    case 'css':
      return 'css';
    case 'yml':
    case 'yaml':
      return 'yaml';
    case 'sh':
    case 'bash':
      return 'shell';
    case 'dockerfile':
      return 'dockerfile';
    default:
      return languageHint ? normalizeHint(languageHint) : 'plaintext';
  }
}

function normalizeHint(h?: string | null): LangId {
  if (!h) return 'plaintext';
  const lower = h.toLowerCase();
  if (lower.includes('rust')) return 'rust';
  if (lower.includes('toml')) return 'toml';
  if (lower.includes('ts')) return 'typescript';
  if (lower.includes('js')) return 'javascript';
  if (lower.includes('json')) return 'json';
  if (lower.includes('md') || lower.includes('markdown')) return 'markdown';
  if (lower.includes('html')) return 'html';
  if (lower.includes('css')) return 'css';
  if (lower.includes('yaml') || lower.includes('yml')) return 'yaml';
  if (lower.includes('shell') || lower.includes('sh') || lower.includes('bash')) return 'shell';
  if (lower.includes('docker')) return 'dockerfile';
  return lower as LangId;
}

/**
 * Lazy-load a CodeMirror language support extension for the given LangId.
 * Returns null if no loader is available or the dynamic import fails.
 *
 * The returned value is a CodeMirror extension (LanguageSupport or similar) that
 * can be included in EditorState extensions.
 */
export async function loadLanguageSupport(lang: LangId): Promise<Extension | null> {
  try {
    switch (lang) {
      case 'rust': {
        const mod = await import('@codemirror/lang-rust');
        // lang-rust exports `rust()` language support
        return (mod as any).rust();
      }

      case 'typescript': {
        // @codemirror/lang-javascript supports JS/TS/JSX/TSX via options
        const mod = await import('@codemirror/lang-javascript');
        return (mod as any).javascript({ typescript: true, jsx: false });
      }

      case 'javascript': {
        const mod = await import('@codemirror/lang-javascript');
        return (mod as any).javascript({ typescript: false, jsx: true });
      }

      case 'json': {
        const mod = await import('@codemirror/lang-json');
        return (mod as any).json();
      }

      case 'markdown': {
        const mod = await import('@codemirror/lang-markdown');
        // markdown() returns LanguageSupport and wires common code languages automatically
        return (mod as any).markdown();
      }

      case 'html': {
        const mod = await import('@codemirror/lang-html');
        return (mod as any).html();
      }

      case 'css': {
        const mod = await import('@codemirror/lang-css');
        return (mod as any).css();
      }

      case 'yaml': {
        // Try community package first; if not available, fall back to null
        try {
          const mod = await import('@codemirror/legacy-modes/mode/yaml');
          const legacy = await import('@codemirror/legacy-modes');
          return (legacy as any).StreamLanguage.define((mod as any).yaml);
        } catch {
          return null;
        }
      }

      case 'toml': {
        // TOML: try a community lezer package if available; otherwise null -> plaintext fallback
        try {
          const mod = await import('@lezer/toml');
          // There's no official language wrapper in many setups; try community wrapper
          try {
            const wrap = await import('@codemirror/legacy-modes');
            return (wrap as any).StreamLanguage.define((mod as any).toml);
          } catch {
            // If no wrapper, return null
            return null;
          }
        } catch {
          return null;
        }
      }

      case 'shell': {
        try {
          const mod = await import('@codemirror/legacy-modes/mode/shell');
          const legacy = await import('@codemirror/legacy-modes');
          return (legacy as any).StreamLanguage.define((mod as any).shell);
        } catch {
          return null;
        }
      }

      case 'dockerfile': {
        // Try to use a simple dockerfile mode via legacy-modes if available
        try {
          const mod = await import('@codemirror/legacy-modes/mode/dockerfile');
          const legacy = await import('@codemirror/legacy-modes');
          return (legacy as any).StreamLanguage.define((mod as any).dockerfile);
        } catch {
          return null;
        }
      }

      case 'plaintext':
      default:
        return null;
    }
  } catch (err) {
    // Dynamic import failed or package missing; fall back to plaintext.
    // eslint-disable-next-line no-console
    console.debug('[languages] failed to load language', lang, err);
    return null;
  }
}

/**
 * Convenience: detect + load language support for a given file path.
 */
export async function getLanguageSupportForPath(path?: string | null, languageHint?: string | null): Promise<Extension | null> {
  const lang = detectLanguageFromPath(path ?? undefined, languageHint ?? undefined);
  return loadLanguageSupport(lang);
}
