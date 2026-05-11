/**
 * Declarative language registry.
 *
 * This file contains metadata for an initial broad set of languages and uses
 * explicit dynamic imports for core language loaders so Vite can statically
 * include the packages during pre-bundling. We avoid computed import paths
 * here to eliminate runtime resolution failures.
 *
 * Policy:
 * - official `@codemirror/lang-*` packages are preferred
 * - lezer packages are used next (e.g., @lezer/toml)
 * - legacy fallback is NOT used in the main runtime path
 * - plaintext fallback (null) is always valid
 */

import type { LanguageMeta } from './types';
import { lezerLoader, officialLoader } from './loaders';

/**
 * Registry map: id -> LanguageMeta
 *
 * Each entry is intentionally small and declarative. Loader functions return
 * a CodeMirror Extension (LanguageSupport) or null on failure.
 *
 * NOTE: only packages that are present in apps/desktop/package.json should be referenced here.
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
    loader: async () => {
      try {
        const m = await import('@codemirror/lang-rust');
        return (m as any).rust();
      } catch (err) {
        // eslint-disable-next-line no-console
        console.debug('[languages][loader] failed to load @codemirror/lang-rust', err);
        return null;
      }
    },
  },

  // TOML - explicit bundler-safe loader. We avoid computed import strings (lezerLoader)
  // and instead import the known package 'lezer-toml' using a static module specifier so Vite
  // can include it in production bundles. This loader returns a real LanguageSupport
  // (or null on failure) and emits precise debug logs.
  toml: {
    id: 'toml',
    name: 'TOML',
    extensions: ['toml'],
    filenames: ['cargo.toml'],
    aliases: ['toml'],
    packageType: 'official',
    loader: async () => {
      const modulePath = 'lezer-toml';
      // eslint-disable-next-line no-console
      console.debug('[languages][toml] loader: attempting to import module:', modulePath);
      try {
        const mod = await import('lezer-toml');
        // Try common export names for the parser
        const parser = (mod as any).parser ?? (mod as any).default ?? null;
        if (!parser) {
          // eslint-disable-next-line no-console
          console.debug('[languages][toml] loader: parser export not found in module:', modulePath, 'moduleKeys=', Object.keys(mod));
          return null;
        }

        // Import codemirror language helpers (static specifier keeps bundler aware)
        const languageMod = await import('@codemirror/language');
        const { LRLanguage, LanguageSupport } = languageMod as any;
        if (!LRLanguage || !LanguageSupport) {
          // eslint-disable-next-line no-console
          console.debug('[languages][toml] loader: codemirror language helpers missing (LRLanguage/LanguageSupport)');
          return null;
        }

        const lang = LRLanguage.define(parser);
        const support = new LanguageSupport(lang);
        // eslint-disable-next-line no-console
        console.debug('[languages][toml] loader: successfully created LanguageSupport from', modulePath);
        return support;
      } catch (err) {
        // eslint-disable-next-line no-console
        console.debug('[languages][toml] loader: failed to load module', modulePath, 'error=', err);
        return null;
      }
    },
  },

  // YAML - official package
  yaml: {
    id: 'yaml',
    name: 'YAML',
    extensions: ['yml', 'yaml'],
    filenames: [],
    aliases: ['yaml', 'yml'],
    packageType: 'official',
    loader: async () => {
      try {
        const mod = await import('@codemirror/lang-yaml');
        if ((mod as any).yaml) return (mod as any).yaml();
        if ((mod as any).default) return (mod as any).default;
        return null;
      } catch (err) {
        // eslint-disable-next-line no-console
        console.debug('[languages][loader] failed to load @codemirror/lang-yaml', err);
        return null;
      }
    },
  },

  // JSON
  json: {
    id: 'json',
    name: 'JSON',
    extensions: ['json'],
    filenames: ['package.json', 'tsconfig.json'],
    aliases: ['json'],
    packageType: 'official',
    loader: async () => {
      try {
        const m = await import('@codemirror/lang-json');
        return (m as any).json();
      } catch (err) {
        // eslint-disable-next-line no-console
        console.debug('[languages][loader] failed to load @codemirror/lang-json', err);
        return null;
      }
    },
  },

  // JavaScript / TypeScript (use lang-javascript with options)
  javascript: {
    id: 'javascript',
    name: 'JavaScript',
    extensions: ['js', 'mjs', 'cjs', 'jsx'],
    filenames: [],
    aliases: ['javascript', 'js'],
    packageType: 'official',
    loader: async () => {
      try {
        const m = await import('@codemirror/lang-javascript');
        return (m as any).javascript({ typescript: false, jsx: true });
      } catch (err) {
        // eslint-disable-next-line no-console
        console.debug('[languages][loader] failed to load @codemirror/lang-javascript', err);
        return null;
      }
    },
  },

  typescript: {
    id: 'typescript',
    name: 'TypeScript',
    extensions: ['ts', 'tsx'],
    filenames: [],
    aliases: ['ts', 'typescript'],
    packageType: 'official',
    loader: async () => {
      try {
        const m = await import('@codemirror/lang-javascript');
        return (m as any).javascript({ typescript: true, jsx: true });
      } catch (err) {
        // eslint-disable-next-line no-console
        console.debug('[languages][loader] failed to load @codemirror/lang-javascript (ts)', err);
        return null;
      }
    },
  },

  // Markdown
  markdown: {
    id: 'markdown',
    name: 'Markdown',
    extensions: ['md', 'markdown'],
    filenames: ['readme.md', 'readme'],
    aliases: ['markdown', 'md'],
    packageType: 'official',
    loader: async () => {
      try {
        const m = await import('@codemirror/lang-markdown');
        return (m as any).markdown();
      } catch (err) {
        // eslint-disable-next-line no-console
        console.debug('[languages][loader] failed to load @codemirror/lang-markdown', err);
        return null;
      }
    },
  },

  // HTML
  html: {
    id: 'html',
    name: 'HTML',
    extensions: ['html', 'htm'],
    filenames: [],
    aliases: ['html'],
    packageType: 'official',
    loader: async () => {
      try {
        const m = await import('@codemirror/lang-html');
        return (m as any).html();
      } catch (err) {
        // eslint-disable-next-line no-console
        console.debug('[languages][loader] failed to load @codemirror/lang-html', err);
        return null;
      }
    },
  },

  // CSS
  css: {
    id: 'css',
    name: 'CSS',
    extensions: ['css'],
    filenames: [],
    aliases: ['css'],
    packageType: 'official',
    loader: async () => {
      try {
        const m = await import('@codemirror/lang-css');
        return (m as any).css();
      } catch (err) {
        // eslint-disable-next-line no-console
        console.debug('[languages][loader] failed to load @codemirror/lang-css', err);
        return null;
      }
    },
  },

  // Python
  python: {
    id: 'python',
    name: 'Python',
    extensions: ['py'],
    filenames: [],
    aliases: ['python'],
    packageType: 'official',
    // Use officialLoader helper so dynamic import is performed via loaders.ts and is bundler-safe.
    loader: officialLoader('@codemirror/lang-python', (m: any) => {
      if (m && typeof (m as any).python === 'function') return (m as any).python();
      return (m && (m as any).default) ?? null;
    }),
  },

  // XML
  xml: {
    id: 'xml',
    name: 'XML',
    extensions: ['xml'],
    filenames: [],
    aliases: ['xml'],
    packageType: 'official',
    loader: officialLoader('@codemirror/lang-xml', (m: any) => {
      if (m && typeof (m as any).xml === 'function') return (m as any).xml();
      return (m && (m as any).default) ?? null;
    }),
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
