/**
 * Declarative language registry.
 *
 * This file contains metadata for an initial broad set of languages and uses
 * loader primitives from loaders.ts to construct safe dynamic loaders.
 *
 * Policy:
 * - official `@codemirror/lang-*` packages are preferred
 * - lezer packages are used next (e.g., @lezer/toml)
 * - legacy fallback is available only if explicitly supplied and installed
 * - plaintext fallback (null) is always valid
 */

import type { LanguageMeta } from './types';
import { officialLoader, lezerLoader, legacyLoader } from './loaders';

/**
 * Registry map: id -> LanguageMeta
 *
 * Each entry is intentionally small and declarative. Adding a language means
 * adding one entry here. Loader functions are safe (catch errors and return null).
 *
 * NOTE: only packages that are present in package.json should be referenced here.
 * We include loaders for the core set required by the acceptance tests.
 */
export const registry: Record<string, LanguageMeta> = {
  // Rust
  rust: {
    id: 'rust',
    name: 'Rust',
    extensions: ['rs'],
    filenames: [],
    aliases: ['rust'],
    packageType: 'official',
    loader: officialLoader('@codemirror/lang-rust', (m) => (m as any).rust()),
  },

  // TOML (legacy fallback - replace with a modern Lezer package if/when available)
  toml: {
    id: 'toml',
    name: 'TOML',
    extensions: ['toml'],
    filenames: ['cargo.toml'],
    aliases: ['toml'],
    packageType: 'legacy',
    loader: legacyLoader('@codemirror/legacy-modes', 'mode/toml', 'toml'),
    note: 'TOML via legacy-modes fallback (install @codemirror/legacy-modes); replace with a Lezer-based loader when available',
  },

  // YAML - official package
  yaml: {
    id: 'yaml',
    name: 'YAML',
    extensions: ['yml', 'yaml'],
    filenames: [],
    aliases: ['yaml', 'yml'],
    packageType: 'official',
    loader: officialLoader('@codemirror/lang-yaml', (m) => (m as any).yaml()),
  },

  // JSON
  json: {
    id: 'json',
    name: 'JSON',
    extensions: ['json'],
    filenames: ['package.json', 'tsconfig.json'],
    aliases: ['json'],
    packageType: 'official',
    loader: officialLoader('@codemirror/lang-json', (m) => (m as any).json()),
  },

  // JavaScript / TypeScript (use lang-javascript with options)
  javascript: {
    id: 'javascript',
    name: 'JavaScript',
    extensions: ['js', 'mjs', 'cjs', 'jsx'],
    filenames: [],
    aliases: ['javascript', 'js'],
    packageType: 'official',
    loader: officialLoader('@codemirror/lang-javascript', (m) => (m as any).javascript({ typescript: false, jsx: true })),
  },

  typescript: {
    id: 'typescript',
    name: 'TypeScript',
    extensions: ['ts', 'tsx'],
    filenames: [],
    aliases: ['ts', 'typescript'],
    packageType: 'official',
    loader: officialLoader('@codemirror/lang-javascript', (m) => (m as any).javascript({ typescript: true, jsx: true })),
  },

  // Markdown
  markdown: {
    id: 'markdown',
    name: 'Markdown',
    extensions: ['md', 'markdown'],
    filenames: ['readme.md', 'readme'],
    aliases: ['markdown', 'md'],
    packageType: 'official',
    loader: officialLoader('@codemirror/lang-markdown', (m) => (m as any).markdown()),
  },

  // HTML
  html: {
    id: 'html',
    name: 'HTML',
    extensions: ['html', 'htm'],
    filenames: [],
    aliases: ['html'],
    packageType: 'official',
    loader: officialLoader('@codemirror/lang-html', (m) => (m as any).html()),
  },

  // CSS
  css: {
    id: 'css',
    name: 'CSS',
    extensions: ['css'],
    filenames: [],
    aliases: ['css'],
    packageType: 'official',
    loader: officialLoader('@codemirror/lang-css', (m) => (m as any).css()),
  },

  // Plaintext fallback (no loader)
  plaintext: {
    id: 'plaintext',
    name: 'Plain Text',
    extensions: [],
    filenames: [],
    aliases: ['text', 'plaintext'],
    packageType: 'plain',
    loader: async () => null,
  },
};

/**
 * Helper: get meta by id; if unknown returns plaintext meta
 */
export function getMeta(id: string) {
  return registry[id] ?? registry['plaintext'];
}
