/**
 * Public facade for language detection and loading.
 *
 * Exports:
 *  - detectLanguage(path?: string, hint?: string) -> LanguageId ('plaintext' if unknown)
 *  - getLanguageSupportForPath(path?: string, hint?: string) -> Promise<Extension | null>
 *
 * The registry is declarative and loaders are dynamic imports that return CM6 LanguageSupport
 * or null. The editor wrapper should call getLanguageSupportForPath() and pass the extension
 * into the createState() / createBaseExtensions functions.
 *
 * Added explicit runtime debug logging for the core language load path so we can
 * diagnose resolution failures during development.
 */

import type { Extension } from '@codemirror/state';
import { detectLanguageForRegistry } from './detect';
import { registry, getMeta } from './registry';
import type { LanguageMeta, LanguageId } from './types';

/**
 * Detect a language id for a path/hint using the registry metadata.
 * Logs the detected id for runtime diagnostics.
 */
export function detectLanguage(path?: string | null, hint?: string | null): LanguageId {
  const id = detectLanguageForRegistry(path ?? undefined, hint ?? undefined, registry);
  // eslint-disable-next-line no-console
  console.debug('[languages] detectLanguage:', { path, hint, id });
  return id;
}

/**
 * Load language support for a given normalized id.
 * Returns the loaded Extension or null.
 *
 * This function logs start/finish/failure so runtime module resolution can be observed.
 */
export async function loadLanguage(id: string): Promise<Extension | null> {
  const meta: LanguageMeta = getMeta(id);
  if (!meta || !meta.loader) {
    // eslint-disable-next-line no-console
    console.debug('[languages] loadLanguage: no meta/loader for id=', id);
    return null;
  }
  // eslint-disable-next-line no-console
  console.debug('[languages] loadLanguage: starting loader for', id, 'packageType=', meta.packageType);
  try {
    const ext = await meta.loader();
    // eslint-disable-next-line no-console
    console.debug('[languages] loadLanguage: loader result for', id, ext ? 'EXTENSION' : 'null');
    return ext ?? null;
  } catch (err) {
    // eslint-disable-next-line no-console
    console.debug('[languages] loadLanguage failed for', id, err);
    return null;
  }
}

/**
 * Convenience: detect by path/hint then load the language support.
 * This is the single function the editor wrapper should use.
 */
export async function getLanguageSupportForPath(path?: string | null, hint?: string | null): Promise<Extension | null> {
  const id = detectLanguage(path ?? undefined, hint ?? undefined);
  // eslint-disable-next-line no-console
  console.debug('[languages] getLanguageSupportForPath: detected', { path, hint, id });
  return loadLanguage(id);
}

export { registry, getMeta };
