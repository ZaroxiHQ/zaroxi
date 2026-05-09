/**
 * Per-document syntax store.
 *
 * This module provides a simple, global, document‑bound cache for highlight
 * snapshots so editor views can mount/unmount without losing a document's
 * latest valid syntax state.
 *
 * Stored shape:
 *   Map<documentId, DocumentSyntax>
 *
 * DocumentSyntax:
 *   { text: string; map: Map<number, HighlightLine>; version?: number; language?: string; updatedAt?: number }
 *
 * Functions:
 *   getDocumentSyntax(documentId)
 *   setDocumentSyntax(documentId, syntax)
 *   clearDocumentSyntax(documentId)
 *
 * Keep this intentionally small and synchronous: it's an in-memory, single-process
 * store used by the editor components to avoid lifecycle loss on remount/switch.
 */

/* Minimal local types to avoid cross-file circular imports. Consumers should
   keep compatible types (HighlightLine). */
export type HighlightSpan = {
  start: number;
  end: number;
  token_type: string;
  color?: string | null;
};

export type HighlightLine = {
  uid: string;
  index: number;
  text: string;
  spans: HighlightSpan[];
};

export type DocumentSyntax = {
  text: string;
  map: Map<number, HighlightLine>;
  version?: number | string;
  language?: string;
  updatedAt?: number;
};

const STORE: Map<string, DocumentSyntax> = new Map();

export function getDocumentSyntax(documentId: string | null | undefined): DocumentSyntax | undefined {
  if (!documentId) return undefined;
  return STORE.get(documentId);
}

export function setDocumentSyntax(documentId: string, syntax: DocumentSyntax): void {
  syntax.updatedAt = Date.now();
  STORE.set(documentId, syntax);
}

export function clearDocumentSyntax(documentId: string): void {
  STORE.delete(documentId);
}
