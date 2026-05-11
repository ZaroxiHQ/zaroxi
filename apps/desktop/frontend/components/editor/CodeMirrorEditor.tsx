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

        // Load language support (lazy)
        let languageExt = null;
        try {
          languageExt = await getLanguageSupportForPath(documentId ?? undefined, languageId ?? undefined);
        } catch (e) {
          // ignore language load failures; fallback to no language (plaintext)
          // eslint-disable-next-line no-console
          console.debug('[codemirror] language load failed', e);
        }

        if (!state) {
          // createState constructs a state with the provided language extension.
          state = await createState(
            text ?? '',
            {
              onChange: (t: string) => {
                onChange(t);
              },
            },
            languageExt ?? undefined,
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
        // attempt to append the lineNumbers() extension at runtime as a minimal fallback.
        // This helps when extension ordering or race conditions prevent the gutter from appearing.
        setTimeout(async () => {
          try {
            const dom = mountedView.dom;
            if (!dom.querySelector('.cm-gutters')) {
              // Dynamically import to avoid changing the static bundle.
              const viewMod = await import('@codemirror/view');
              const stateMod = await import('@codemirror/state');
              const lineNumFactory = (viewMod as any).lineNumbers ?? (viewMod as any).default?.lineNumbers;
              const StateEffect = (stateMod as any).StateEffect;
              if (typeof lineNumFactory === 'function' && StateEffect && mountedView) {
                // Append the lineNumbers extension to the existing state.
                try {
                  mountedView.dispatch({
                    effects: StateEffect.appendConfig.of([lineNumFactory()]),
                  });
                } catch (e) {
                  // eslint-disable-next-line no-console
                  console.debug('[codemirror] failed to append lineNumbers extension', e);
                }
              }
            }
          } catch (e) {
            // eslint-disable-next-line no-console
            console.debug('[codemirror] gutter fallback append failed', e);
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
