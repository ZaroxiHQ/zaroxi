/**
 * Loader primitives for the language registry.
 *
 * These helpers produce async loader functions that return a CodeMirror Extension
 * (LanguageSupport) or null on failure. They deliberately use dynamic imports so Vite
 * can pre-bundle only installed packages; do not reference packages that aren't in
 * package.json.
 */

import type { Extension } from '@codemirror/state';

/**
 * Official package loader helper.
 * Example: officialLoader('@codemirror/lang-yaml', (m) => m.yaml())
 */
export function officialLoader(packageName: string, factory: (mod: any) => Extension | Promise<Extension>) {
  return async (): Promise<Extension | null> => {
    try {
      const mod = await import(/* @vite-ignore */ packageName);
      const res = await factory(mod);
      return res ?? null;
    } catch (err) {
      // eslint-disable-next-line no-console
      console.debug(`[languages][loader] officialLoader failed for ${packageName}`, err);
      return null;
    }
  };
}

/**
 * Lezer grammar loader helper.
 * Imports a Lezer parser package (e.g., @lezer/toml) and wraps it with LRLanguage/LanguageSupport.
 */
export function lezerLoader(lezerPackageName: string, parserExportName = 'parser') {
  return async (): Promise<Extension | null> => {
    try {
      const lezer = await import(/* @vite-ignore */ lezerPackageName);
      const parser = (lezer as any)[parserExportName] ?? (lezer as any).parser ?? null;
      if (!parser) {
        // eslint-disable-next-line no-console
        console.debug('[languages][loader] lezer parser export not found in', lezerPackageName);
        return null;
      }
      const languageMod = await import('@codemirror/language');
      const { LRLanguage, LanguageSupport } = languageMod as any;
      const lang = LRLanguage.define(parser);
      return new LanguageSupport(lang);
    } catch (err) {
      // eslint-disable-next-line no-console
      console.debug(`[languages][loader] lezerLoader failed for ${lezerPackageName}`, err);
      return null;
    }
  };
}

/**
 * Legacy-mode loader (optional). Use only when there is no modern package.
 * This keeps legacy-mode imports out of the main static graph. The caller must
 * decide whether to include legacy-modes in dependencies.
 */
export function legacyLoader(legacyModuleName: string, modePath: string, modeExport = '') {
  return async (): Promise<Extension | null> => {
    try {
      // Compose path at runtime to avoid static analysis by Vite.
      // eslint-disable-next-line no-eval
      const mod = await eval(`import("${legacyModuleName}/${modePath}")`);
      const legacy = await eval(`import("${legacyModuleName}")`);
      const mode = (mod as any)[modeExport] ?? (mod as any).default ?? null;
      const stream = (legacy as any).StreamLanguage;
      if (!mode || !stream) return null;
      return stream.define(mode);
    } catch (err) {
      // eslint-disable-next-line no-console
      console.debug(`[languages][loader] legacyLoader failed for ${legacyModuleName}/${modePath}`, err);
      return null;
    }
  };
}
