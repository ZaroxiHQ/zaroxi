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
    const prev = this.map.get(documentId) ?? { documentId } as DocumentRecord;
    const merged: DocumentRecord = {
      documentId,
      content: rec.content ?? prev.content ?? null,
      isDirty: rec.isDirty ?? prev.isDirty ?? false,
      version: rec.version ?? prev.version ?? null,
      mtime: rec.mtime ?? prev.mtime ?? null,
    };
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
