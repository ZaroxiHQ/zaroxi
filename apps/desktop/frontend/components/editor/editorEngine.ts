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

let engine: EditorEngine = (typeof window !== 'undefined' && (window.localStorage.getItem(LS_KEY) as EditorEngine)) || 'codemirror';

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
 *
 * Keys are canonical documentIds.
 */
export const stateCache: Map<string, any> = new Map();

export function getCachedState(docId: string) {
  return stateCache.get(docId);
}

const MAX_CACHED_STATES = 20;

/**
 * Store opaque per-document editor state with an LRU eviction policy.
 * - If `state` is `null` or `undefined`, the entry is removed.
 * - Otherwise the state is inserted/updated and we evict oldest entries if the cache exceeds MAX_CACHED_STATES.
 */
export function setCachedState(docId: string, state: any) {
  if (state == null) {
    stateCache.delete(docId);
  } else {
    // If updating existing key, delete first to move it to the back (most-recently-used)
    if (stateCache.has(docId)) {
      stateCache.delete(docId);
    }
    stateCache.set(docId, state);
    // Evict oldest entries until under limit
    while (stateCache.size > MAX_CACHED_STATES) {
      const firstKey = stateCache.keys().next().value;
      try {
        stateCache.delete(firstKey);
      } catch {
        break;
      }
    }
  }

  // Update runtime instrumentation if present
  try {
    // Only update runtime instrumentation when explicit debug flag is set.
    const w: any = window as any;
    if (w.__zaroxi_cm_debug) {
      w.__zaroxi_cm_stats = w.__zaroxi_cm_stats || {};
      w.__zaroxi_cm_stats.cachedStates = stateCache.size;
    }
  } catch {}
}

/**
 * Delete cached EditorState for a given document id (safe no-op).
 * This helper ensures callers remove by documentId (not tabId).
 */
export function deleteCachedState(docId: string | null | undefined) {
  if (!docId) return;
  try {
    if (stateCache.has(docId)) stateCache.delete(docId);
    try {
      const w: any = window as any;
      if (w.__zaroxi_cm_debug) {
        w.__zaroxi_cm_stats = w.__zaroxi_cm_stats || {};
        w.__zaroxi_cm_stats.cachedStates = stateCache.size;
      }
    } catch {}
  } catch {}
}
