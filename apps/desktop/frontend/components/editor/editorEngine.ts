/**
 * Lightweight editor engine switch + EditorState cache for the experimental CodeMirror path.
 *
 * - getEditorEngine / setEditorEngine control which engine is active ("custom" | "codemirror").
 * - stateCache stores per-document EditorState (opaque `any` here to avoid coupling).
 *
 * This module is intentionally small and non-opinionated. It persists the engine choice to
 * localStorage so you can toggle the experiment in the browser.
 */

export type EditorEngine = 'custom' | 'codemirror';

const LS_KEY = 'editorEngine:v1';

let engine: EditorEngine = (typeof window !== 'undefined' && (window.localStorage.getItem(LS_KEY) as EditorEngine)) || 'custom';

export function getEditorEngine(): EditorEngine {
  return engine;
}

export function setEditorEngine(v: EditorEngine) {
  engine = v;
  try {
    window.localStorage.setItem(LS_KEY, v);
  } catch {
    // ignore
  }
}

/**
 * Opaque per-document EditorState cache. The CodeMirror wrapper will store EditorState
 * instances here so switching tabs can restore state/selection without full re-creation.
 */
export const stateCache: Map<string, any> = new Map();

export function getCachedState(docId: string) {
  return stateCache.get(docId);
}

export function setCachedState(docId: string, state: any) {
  stateCache.set(docId, state);
}
