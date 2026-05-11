import React, { useEffect, useRef, useState } from 'react';
import { createState } from './codemirror/setup';
import { getCachedState, setCachedState } from './editorEngine';
import { getLanguageSupportForPath } from './codemirror/languages/index';
import { EditorView } from '@codemirror/view';

interface CodeMirrorEditorProps {
  documentId: string | null;
  text: string;
  languageId?: string | null;
  onChange: (text: string) => void;
  onSave?: () => void;
  readOnly?: boolean;
}

/**
 * CodeMirror 6 React wrapper using standard CM6 language packages.
 *
 * - Dynamically loads language support via codemirror/languages.ts and passes
 *   it into the EditorState creation. No Tree-sitter parsing is performed.
 * - If CodeMirror packages are missing, falls back to a simple <textarea>.
 */
export function CodeMirrorEditor(props: CodeMirrorEditorProps) {
  const { documentId, text, languageId, onChange, onSave, readOnly } = props;
  const containerRef = useRef<HTMLDivElement | null>(null);
  const viewRef = useRef<any | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const [usingFallback, setUsingFallback] = useState(false);
  // Hold the loaded language extension so we can re-create/reconfigure EditorState if needed.
  const languageExtRef = useRef<any | null>(null);

  // Mount CodeMirror EditorView (async). If dynamic imports fail, flip to textarea fallback.
  useEffect(() => {
    if (!containerRef.current) return;

    let destroyed = false;

    (async () => {
      try {
        // Try to reuse cached state (opaque) if available.
        let state = undefined;
        if (documentId) {
          state = getCachedState(documentId);
        }

        // Load language support (lazy) and stash it so we can re-create the state later if needed.
        languageExtRef.current = null;
        try {
          languageExtRef.current = await getLanguageSupportForPath(documentId ?? undefined, languageId ?? undefined);
        } catch (e) {
          // ignore language load failures; fallback to no language (plaintext)
          // eslint-disable-next-line no-console
          console.debug('[codemirror] language load failed', e);
        }

        if (!state) {
          // createState constructs a state with the provided language extension.
          // createState is synchronous in this module; use it directly.
          state = createState(
            text ?? '',
            {
              onChange: (t: string) => {
                onChange(t);
              },
            },
            languageExtRef.current ?? undefined,
            documentId ?? undefined,
          );
        }

        if (destroyed) return;

        const mountedView = new EditorView({
          state,
          parent: containerRef.current,
        });

        viewRef.current = mountedView;
        // Focus the editor so caret and wheel interactions behave consistently.
        try {
          mountedView.focus();
        } catch {
          // ignore focus errors in environments that don't support it
        }

        // Sanity check: if the gutter DOM is missing (styles/extensions not applied),
        // recreate the EditorState (using createState which includes lineNumbers()) and set it
        // on the mounted view. This is a small, deterministic fallback that avoids dynamic
        // imports or reliance on appendConfig behavior.
        setTimeout(() => {
          try {
            const dom = mountedView.dom;
            if (!dom.querySelector('.cm-gutters')) {
              // eslint-disable-next-line no-console
              console.debug('[codemirror] gutter missing; reconfiguring state to include lineNumbers extension');
              try {
                const newState = createState(
                  mountedView.state.doc.toString(),
                  {
                    onChange: (t: string) => {
                      onChange(t);
                    },
                  },
                  languageExtRef.current ?? undefined,
                  documentId ?? undefined,
                );
                // Replace the state so the full extension set is applied.
                mountedView.setState(newState);
              } catch (e) {
                // eslint-disable-next-line no-console
                console.debug('[codemirror] failed to reconfigure state for gutter', e);
              }
            }
          } catch (e) {
            // eslint-disable-next-line no-console
            console.debug('[codemirror] gutter recheck failed', e);
          }
        }, 50);
      } catch (err) {
        // Fallback path: show a native textarea bound to the document text.
        // This keeps the app usable when codemirror packages are missing.
        // eslint-disable-next-line no-console
        console.debug('[codemirror] falling back to textarea (dynamic import failed)', err);
        setUsingFallback(true);
      }
    })();

    return () => {
      destroyed = true;
      if (viewRef.current) {
        try {
          if (documentId) {
            setCachedState(documentId, viewRef.current.state);
          }
        } catch {
          // ignore caching errors
        }
        try {
          viewRef.current.destroy();
        } catch {
          // ignore destroy errors
        }
        viewRef.current = null;
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [documentId]);

  // Update document text if changed externally (e.g., reload from workspace).
  useEffect(() => {
    if (usingFallback) {
      if (textareaRef.current && textareaRef.current.value !== text) {
        textareaRef.current.value = text ?? '';
      }
      return;
    }
    const view = viewRef.current;
    if (!view) return;
    const current = view.state.doc.toString();
    if (text !== current) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: text ?? '' },
      });
    }
  }, [text, usingFallback]);

  // Render fallback textarea if CodeMirror isn't available.
  if (usingFallback) {
    return (
      <textarea
        ref={textareaRef}
        className="h-full w-full min-h-0 min-w-0 p-2 bg-editor text-editor-foreground"
        defaultValue={text}
        onChange={(e) => onChange(e.target.value)}
      />
    );
  }

  // Otherwise provide the container for CodeMirror to mount into.
  return <div ref={containerRef} className="h-full w-full min-h-0 min-w-0" />;
}

export default CodeMirrorEditor;
