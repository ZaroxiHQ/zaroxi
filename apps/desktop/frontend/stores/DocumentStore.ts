/**
 * DocumentStore - minimal source-of-truth for document content and dirty/version flags.
 *
 * This store is intentionally small and synchronous. It does not own EditorView or UI state.
 */

export type DocumentRecord = {
  documentId: string;
  content?: string | null;
  isDirty?: boolean;
  version?: number | null;
  mtime?: number | null;
};

class DocumentStore {
  private map: Map<string, DocumentRecord>;
  constructor() {
    this.map = new Map();
  }

  set(documentId: string, rec: Partial<DocumentRecord>) {
    const prev = this.map.get(documentId) ?? ({ documentId, content: null, isDirty: false, version: null, mtime: null } as DocumentRecord);
    const merged: DocumentRecord = {
      documentId,
      content: rec.content ?? prev.content ?? null,
      isDirty: rec.isDirty ?? prev.isDirty ?? false,
      version: rec.version ?? prev.version ?? null,
      mtime: rec.mtime ?? prev.mtime ?? null,
    };

    // Avoid writing to the underlying map if nothing meaningful changed.
    // This prevents external observers from reacting to no-op writes and
    // helps break echo loops where identical content is written back repeatedly.
    const same =
      merged.content === prev.content &&
      merged.isDirty === prev.isDirty &&
      merged.version === prev.version &&
      merged.mtime === prev.mtime;

    if (same) return;

    // Guard against editor-origin echoes:
    // If a recent editor emission marker exists for this document and the
    // merged content matches the editor's emitted hash, skip writing. This
    // prevents editor-originated changes from being re-emitted back into the
    // component tree and re-triggering adoption cycles.
    try {
      const lastEmit = (typeof window !== 'undefined') ? (window as any).__zaroxi_last_editor_emit : undefined;
      if (lastEmit && lastEmit.documentId === documentId && typeof merged.content === 'string') {
        const stableHashString = (s: string) => {
          let h = 2166136261 >>> 0;
          for (let i = 0; i < s.length; i++) {
            h ^= s.charCodeAt(i);
            h = Math.imul(h, 16777619) >>> 0;
          }
          return (h >>> 0).toString(16);
        };
        if (stableHashString(merged.content) === lastEmit.hash) {
          // Skip write to avoid echoing editor-originated content back into UI.
          return;
        }
      }
    } catch {
      // Defensive: if anything goes wrong, fall back to writing as before.
    }

    this.map.set(documentId, merged);
  }

  get(documentId: string): DocumentRecord | undefined {
    return this.map.get(documentId);
  }

  clear(documentId: string) {
    this.map.delete(documentId);
  }

  markDirty(documentId: string, dirty = true) {
    const r = this.map.get(documentId) ?? { documentId } as DocumentRecord;
    r.isDirty = dirty;
    this.map.set(documentId, r);
  }
}

const documentStore = new DocumentStore();
export default documentStore;
