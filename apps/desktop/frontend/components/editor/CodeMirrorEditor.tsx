import React, { useEffect, useRef } from 'react';
import { EditorView } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { createState } from './codemirror/setup';
import { getCachedState, setCachedState } from './editorEngine';
import { initTreesitterOnce } from './codemirror/treesitterBridge';

interface CodeMirrorEditorProps {
  documentId: string | null;
  text: string;
  languageId?: string | null;
  onChange: (text: string) => void;
  onSave?: () => void;
  readOnly?: boolean;
}

/**
 * Minimal CodeMirror 6 React wrapper for the trial.
 *
 * Responsibilities:
 * - mount a single EditorView into a container
 * - create/load EditorState (from cache if available)
 * - wire update listener to call onChange(text)
 * - persist EditorState to cache on unmount or document switch
 *
 * This intentionally keeps the logic small so we can iterate on Tree-sitter integration.
 */
export function CodeMirrorEditor(props: CodeMirrorEditorProps) {
  const { documentId, text, languageId, onChange, onSave, readOnly } = props;
  const containerRef = useRef<HTMLDivElement | null>(null);
  const viewRef = useRef<EditorView | null>(null);

  // Initialize treesitter runtime in background (no-op for Phase 1).
  useEffect(() => {
    void initTreesitterOnce();
  }, []);

  // Mount EditorView once.
  useEffect(() => {
    if (!containerRef.current) return;

    // Try to reuse cached EditorState for this document
    let state: EditorState | undefined = undefined;
    if (documentId) {
      state = getCachedState(documentId) as EditorState | undefined;
    }

    if (!state) {
      state = createState(text ?? '', {
        onChange: (t) => {
          onChange(t);
        },
      });
    }

    const view = new EditorView({
      state,
      parent: containerRef.current,
    });

    viewRef.current = view;

    return () => {
      // Save state to cache for the document
      if (documentId && viewRef.current) {
        try {
          setCachedState(documentId, viewRef.current.state);
        } catch {
          // ignore cache errors
        }
      }
      view.destroy();
      viewRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [documentId]); // remount when documentId changes (we persist state to cache above)

  // Update document text if changed externally (e.g., reload from workspace).
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    const current = view.state.doc.toString();
    if (text !== current) {
      // Replace entire document in a single transaction to avoid selection surprises.
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: text ?? '' },
      });
    }
  }, [text]);

  // Expose a minimal DOM container.
  return <div ref={containerRef} className="h-full w-full min-h-0 min-w-0" />;
}

export default CodeMirrorEditor;
