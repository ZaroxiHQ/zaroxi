/**
 * EditorSessionStore
 *
 * Lightweight, serializable per-tab session snapshots. Intentionally minimal:
 * - No DOM references
 * - No EditorView instances
 * - Stores simple primitives: selection, scroll, language, dirty flag, version, lastActiveAt, tier
 *
 * External actors (EditorViewHost / SessionCachePolicy) may call touch/promote/demote.
 */

export type CacheTier = 'hot' | 'warm' | 'cold';

export type EditorSessionSnapshot = {
  tabId: string;
  documentId?: string | null;
  text?: string;
  selection?: { anchor: number; head: number } | null;
  scrollTop?: number | null;
  language?: string | null;
  isDirty?: boolean;
  version?: number | null;
  lastActiveAt?: number | null;
  tier?: CacheTier;
  // Optionally store a compact undo/redo metadata token if available (opaque)
  historyToken?: string | null;
};

class EditorSessionStore {
  private store: Map<string, EditorSessionSnapshot>;

  constructor() {
    this.store = new Map();
  }

  setSnapshot(tabId: string, snap: Partial<EditorSessionSnapshot>) {
    const now = Date.now();
    const prev = this.store.get(tabId) ?? ({ tabId, documentId: undefined, text: undefined, selection: null, scrollTop: null, language: null, isDirty: false, version: null, lastActiveAt: now, tier: 'cold', historyToken: null } as EditorSessionSnapshot);

    // Decide what text to persist:
    // - If caller provided text but it equals existing text, avoid overwriting to prevent echo.
    // - If caller provided a very large text payload, avoid persisting it into the session store (stabilization mode).
    const incomingText = typeof snap.text === 'string' ? snap.text : prev.text;
    const MAX_PERSIST_TEXT = 10000; // do not persist giant texts in the session snapshot during hot-path

    const textToStore =
      typeof snap.text === 'string'
        ? (snap.text === prev.text ? prev.text : (snap.text.length > MAX_PERSIST_TEXT ? prev.text : snap.text))
        : prev.text;

    const merged: EditorSessionSnapshot = {
      tabId,
      documentId: snap.documentId ?? prev.documentId,
      text: textToStore,
      selection: snap.selection ?? prev.selection ?? null,
      scrollTop: snap.scrollTop ?? prev.scrollTop ?? null,
      language: snap.language ?? prev.language ?? null,
      isDirty: snap.isDirty ?? prev.isDirty ?? false,
      version: snap.version ?? prev.version ?? null,
      lastActiveAt: snap.lastActiveAt ?? now,
      tier: snap.tier ?? prev.tier ?? 'cold',
      historyToken: snap.historyToken ?? prev.historyToken ?? null,
    };

    // Guard against editor-origin echoes:
    // If a recent editor emission marker exists and the merged.text matches the editor's hash,
    // avoid overwriting the store entry in order to prevent re-emitting the same content back into the UI.
    try {
      const lastEmit = (typeof window !== 'undefined') ? (window as any).__zaroxi_last_editor_emit : undefined;
      if (lastEmit && lastEmit.documentId === merged.documentId && typeof merged.text === 'string') {
        const stableHashString = (s: string) => {
          let h = 2166136261 >>> 0;
          for (let i = 0; i < s.length; i++) {
            h ^= s.charCodeAt(i);
            h = Math.imul(h, 16777619) >>> 0;
          }
          return (h >>> 0).toString(16);
        };
        if (stableHashString(merged.text) === lastEmit.hash) {
          // If only lastActiveAt changed, update it in-place to keep recency info but avoid changing object identity.
          if (this.store.has(tabId)) {
            const existing = this.store.get(tabId)!;
            existing.lastActiveAt = merged.lastActiveAt;
            this.store.set(tabId, existing);
          } else {
            // No previous entry, set the merged snapshot as this is the first observation.
            this.store.set(tabId, merged);
          }
          return;
        }
      }
    } catch {
      // Defensive: fall through to normal behavior if hashing/marker inspection fails
    }

    // If the merged snapshot is identical to the previous stored snapshot, avoid touching the map
    // to prevent emitting change events / rerenders in consumers that would re-feed the editor.
    const unchanged =
      merged.documentId === prev.documentId &&
      merged.text === prev.text &&
      JSON.stringify(merged.selection) === JSON.stringify(prev.selection) &&
      merged.scrollTop === prev.scrollTop &&
      merged.language === prev.language &&
      merged.isDirty === prev.isDirty &&
      merged.version === prev.version &&
      merged.tier === prev.tier &&
      merged.historyToken === prev.historyToken;

    if (unchanged) {
      // Still update lastActiveAt in-place without replacing the map entry to avoid causing observers to receive
      // a new object reference while keeping the stored snapshot stable.
      if (this.store.has(tabId)) {
        const existing = this.store.get(tabId)!;
        existing.lastActiveAt = merged.lastActiveAt;
        this.store.set(tabId, existing);
      } else {
        // No previous entry, set the merged snapshot.
        this.store.set(tabId, merged);
      }
      return;
    }

    this.store.set(tabId, merged);
  }

  getSnapshot(tabId: string): EditorSessionSnapshot | undefined {
    return this.store.get(tabId);
  }

  touch(tabId: string) {
    const s = this.store.get(tabId);
    const now = Date.now();
    if (s) {
      s.lastActiveAt = now;
      this.store.set(tabId, s);
    } else {
      this.store.set(tabId, { tabId, lastActiveAt: now, tier: 'warm' } as EditorSessionSnapshot);
    }
  }

  setTier(tabId: string, tier: CacheTier) {
    const s = this.store.get(tabId) ?? ({ tabId } as EditorSessionSnapshot);
    s.tier = tier;
    this.store.set(tabId, s);
  }

  getTier(tabId: string): CacheTier {
    const s = this.store.get(tabId);
    return s?.tier ?? 'cold';
  }

  remove(tabId: string) {
    this.store.delete(tabId);
  }

  // Compact a warm session into a cold-only snapshot to free memory.
  compactToCold(tabId: string) {
    const s = this.store.get(tabId);
    if (!s) return;
    const compact: EditorSessionSnapshot = {
      tabId,
      documentId: s.documentId,
      text: s.text ? s.text : undefined, // keep text if small; clients can decide later
      selection: undefined,
      scrollTop: undefined,
      language: s.language ?? null,
      isDirty: s.isDirty ?? false,
      version: s.version ?? null,
      lastActiveAt: s.lastActiveAt ?? null,
      tier: 'cold',
      historyToken: null,
    };
    this.store.set(tabId, compact);
  }
}

const editorSessionStore = new EditorSessionStore();
export default editorSessionStore;
