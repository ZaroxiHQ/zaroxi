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

  // TOML - explicit bundler-safe loader with robust runtime inspection.
  //
  // MANDATORY DEBUGGING NOTES (recorded here for auditability):
  // 1) Exact registry file: apps/desktop/frontend/components/editor/codemirror/languages/registry.ts
  // 2) Observed runtime failing expression (from logs): spec.parser.configure
  //    - This loader must NOT assume `spec.parser.configure` exists on the imported module.
  // 3) TOML package attempted: 'lezer-toml' (static import path below).
  //
  // DEBUGGING BEHAVIOR:
  // - At runtime this loader will log the imported module's keys and a short summary of
  //   the shapes it contains so you can see the real export names exactly.
  // - Based on what is actually exported, it will:
  //     A) If the module exposes a CodeMirror Language factory (e.g. a `toml()` function or
  //        a default LanguageSupport), call/return it directly.
  //     B) If the module exports a raw Lezer parser object (`parser` or default parser),
  //        wrap it using LRLanguage.define(parser) and return a LanguageSupport.
  //     C) Otherwise return null but include precise diagnostic logs so we can decide next.
  //
  // This removes the broken assumption `spec.parser.configure` and performs a safe, logged
  // detection at runtime. The import path is explicit and bundler-safe (`lezer-toml`).
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
        const mod = await import('lezer-toml') as any;

        // STEP 1: Inspect module shape and log keys/types for exact diagnostics.
        const keys = Object.keys(mod);
        const hasDefault = Object.prototype.hasOwnProperty.call(mod, 'default');
        // eslint-disable-next-line no-console
        console.debug('[languages][toml] imported module keys:', keys, 'hasDefault:', hasDefault);

        // Provide a quick dump of the top-level property types to aid debugging (non-blocking).
        try {
          const summary: Record<string, string> = {};
          keys.slice(0, 50).forEach((k) => {
            const v = mod[k];
            summary[k] = v === null ? 'null' : typeof v;
            // Try to detect common parser/language shapes
            if (v && typeof v === 'object') {
              if (v.parse) summary[k] = 'lezer-parser-like';
              if (v.getNodeProp || v.nodeSet) summary[k] = 'lezer-parser-like';
            }
            if (typeof v === 'function') {
              summary[k] = 'function';
            }
          });
          // eslint-disable-next-line no-console
          console.debug('[languages][toml] imported module key types (preview):', summary);
        } catch (inner) {
          // eslint-disable-next-line no-console
          console.debug('[languages][toml] failed to summarize module keys', inner);
        }

        // STEP 2: Integration heuristics (try safe options in order)

        // CASE A: Module provides a ready-made CodeMirror LanguageSupport factory or instance.
        // Common shapes:
        // - export function toml() { ... } -> call it
        // - export default LanguageSupport instance or factory -> return/call it
        try {
          if (typeof mod.toml === 'function') {
            // eslint-disable-next-line no-console
            console.debug('[languages][toml] detected named factory `toml()`; invoking it');
            const res = await Promise.resolve(mod.toml());
            return res ?? null;
          }

          if (hasDefault) {
            const def = mod.default;
            if (def) {
              // If default looks like a LanguageSupport instance (best-effort check)
              if (def.constructor && def.constructor.name === 'LanguageSupport') {
                // eslint-disable-next-line no-console
                console.debug('[languages][toml] default export appears to be LanguageSupport; returning it');
                return def;
              }
              // If default is a function factory, try calling it (some modules export default factory)
              if (typeof def === 'function') {
                try {
                  // eslint-disable-next-line no-console
                  console.debug('[languages][toml] default export is function; invoking default() to obtain support');
                  const maybe = await Promise.resolve(def());
                  if (maybe) return maybe;
                } catch (e) {
                  // ignore and fallthrough to parser handling
                }
              }
              // If default looks like a parser object (has parse/nodeSet), prefer wrapping it below
            }
          }
        } catch (e) {
          // eslint-disable-next-line no-console
          console.debug('[languages][toml] early attempt to use factory/default failed', e);
        }

        // CASE B: Module exports a raw Lezer parser (common: `parser` named export or default parser object)
        let parser: any = null;
        if (mod.parser) parser = mod.parser;
        else if (mod.default && (mod.default.parser || mod.default.parse || mod.default.nodeSet)) parser = mod.default.parser ?? mod.default;
        else if (mod.parse || mod.nodeSet) parser = mod; // module itself looks like parser-like

        if (parser) {
          // eslint-disable-next-line no-console
          console.debug('[languages][toml] detected parser-like export; preparing to wrap with LRLanguage');

          // Import codemirror language helpers (explicit import keeps bundler aware)
          const languageMod = await import('@codemirror/language') as any;
          const { LRLanguage, LanguageSupport } = languageMod ?? {};
          if (!LRLanguage || !LanguageSupport) {
            // eslint-disable-next-line no-console
            console.debug('[languages][toml] loader: codemirror language helpers missing (LRLanguage/LanguageSupport)');
            return null;
          }

          // Log whether the parser exposes a configure method
          const hasConfigure = !!(parser && typeof (parser as any).configure === 'function');
          // eslint-disable-next-line no-console
          console.debug('[languages][toml] parser export detected; has .configure:', hasConfigure);

          try {
            // If parser has a configure method, attempt to configure it safely; otherwise use as-is.
            let parserForLang: any = parser;
            if (hasConfigure) {
              try {
                parserForLang = (parser as any).configure({});
                // eslint-disable-next-line no-console
                console.debug('[languages][toml] parser.configure() succeeded; using configured parser');
              } catch (cfgErr) {
                // If configure fails, fall back to the raw parser but log the issue.
                // eslint-disable-next-line no-console
                console.debug('[languages][toml] parser.configure() threw, falling back to raw parser', cfgErr);
                parserForLang = parser;
              }
            }

            // IMPORTANT: LRLanguage.define expects a spec object with a `parser` field.
            const lang = LRLanguage.define({ parser: parserForLang });
            const support = new LanguageSupport(lang);
            // eslint-disable-next-line no-console
            console.debug('[languages][toml] loader: successfully created LanguageSupport from parser export; returning extension');
            return support;
          } catch (e) {
            // eslint-disable-next-line no-console
            console.debug('[languages][toml] LRLanguage.define failed for parser export', e);
            return null;
          }
        }

        // CASE C: Nothing usable detected - emit detailed diagnostics and return null.
        // eslint-disable-next-line no-console
        console.debug('[languages][toml] loader: could not detect usable export in module:', modulePath, 'moduleKeys=', keys);
        return null;
      } catch (err) {
        // eslint-disable-next-line no-console
        console.debug('[languages][toml] loader: failed to import module', modulePath, 'error=', err);
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
