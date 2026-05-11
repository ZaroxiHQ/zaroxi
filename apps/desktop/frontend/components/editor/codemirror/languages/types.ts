/**
 * Language registry types for CodeMirror loader system.
 *
 * This file defines the canonical metadata shape used by the registry.
 * The registry is declarative: adding a language means adding a LanguageMeta
 * entry with detection hints and a loader (if available).
 */

import type { Extension } from '@codemirror/state';

export type LanguageId = string;

export type PackageType = 'official' | 'modern' | 'legacy' | 'plain';

export type LanguageLoader = () => Promise<Extension | null>;

/**
 * Declarative metadata for a language supported by the registry.
 *
 * - id: normalized id (e.g., 'rust', 'toml')
 * - name: human readable name
 * - extensions: file extensions (without leading dot) matched by extension rule
 * - filenames: special filenames (case-insensitive) matched before extension
 * - aliases: other ids or language hints that map here
 * - packageType: policy classification (official | modern | legacy | plain)
 * - loader: async loader that returns a CodeMirror Extension (LanguageSupport) or null
 * - note: optional developer note
 */
export interface LanguageMeta {
  id: LanguageId;
  name: string;
  extensions: string[];
  filenames?: string[];
  aliases?: string[];
  packageType: PackageType;
  loader?: LanguageLoader;
  note?: string;
}
