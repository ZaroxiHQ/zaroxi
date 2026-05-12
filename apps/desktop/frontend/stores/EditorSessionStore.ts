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
    const prev = this.store.get(tabId) ?? { tabId };
    const merged: EditorSessionSnapshot = {
      tabId,
      documentId: snap.documentId ?? prev.documentId,
      text: snap.text ?? prev.text,
      selection: snap.selection ?? prev.selection ?? null,
      scrollTop: snap.scrollTop ?? prev.scrollTop ?? null,
      language: snap.language ?? prev.language ?? null,
      isDirty: snap.isDirty ?? prev.isDirty ?? false,
      version: snap.version ?? prev.version ?? null,
      lastActiveAt: snap.lastActiveAt ?? now,
      tier: snap.tier ?? prev.tier ?? 'cold',
      historyToken: snap.historyToken ?? prev.historyToken ?? null,
    };
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
