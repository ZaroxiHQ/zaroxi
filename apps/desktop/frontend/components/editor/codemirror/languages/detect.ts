/**
 * Detection helpers for the language registry.
 *
 * Exports a single function detectLanguageForRegistry(path, hint, registry)
 * which returns the matched LanguageId or 'plaintext'.
 *
 * Detection order:
 * 1. explicit hint (if provided)
 * 2. filename match (exact, case-insensitive)
 * 3. extension match (last segment)
 * 4. fallback to 'plaintext'
 */

import type { LanguageMeta, LanguageId } from './types';

export function normalizeFileName(name: string | undefined | null): string {
  if (!name) return '';
  const parts = name.split(/[\\/]/);
  const base = parts[parts.length - 1];
  return base.toLowerCase();
}

export function getExtension(name: string | undefined | null): string {
  if (!name) return '';
  const base = name.split(/[\\/]/).pop() || '';
  const idx = base.lastIndexOf('.');
  if (idx === -1) return '';
  return base.slice(idx + 1).toLowerCase();
}

/**
 * Detect language by scanning registry metadata.
 *
 * @param path optional path or filename
 * @param hint optional explicit language hint (e.g., from LSP or file metadata)
 * @param registry mapping of id -> LanguageMeta
 */
export function detectLanguageForRegistry(
  path: string | undefined | null,
  hint: string | undefined | null,
  registry: Record<string, LanguageMeta>
): LanguageId {
  // 1) explicit hint
  if (hint) {
    const h = hint.toLowerCase();
    // Try exact id
    if (registry[h]) return h;
    // Try alias matching
    for (const id of Object.keys(registry)) {
      const meta = registry[id];
      if (meta.aliases && meta.aliases.map(a => a.toLowerCase()).includes(h)) {
        return id;
      }
      if (meta.name && meta.name.toLowerCase() === h) {
        return id;
      }
    }
  }

  // 2) filename match
  const fname = normalizeFileName(path);
  if (fname) {
    for (const id of Object.keys(registry)) {
      const meta = registry[id];
      if (meta.filenames) {
        for (const f of meta.filenames) {
          if (f.toLowerCase() === fname) return id;
        }
      }
    }
  }

  // 3) extension match
  const ext = getExtension(path);
  if (ext) {
    for (const id of Object.keys(registry)) {
      const meta = registry[id];
      if (meta.extensions && meta.extensions.map(e => e.toLowerCase()).includes(ext)) {
        return id;
      }
    }
  }

  // 4) fallback
  return 'plaintext';
}
