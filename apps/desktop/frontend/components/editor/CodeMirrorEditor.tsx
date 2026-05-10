import React, { useEffect, useRef, useState } from 'react';
import { createState, applyDecorationsToView } from './codemirror/setup';
import { getCachedState, setCachedState } from './editorEngine';
import { initTreesitterOnce, parseAndComputeDecorations } from './codemirror/treesitterBridge';

interface CodeMirrorEditorProps {
  documentId: string | null;
  text: string;
  languageId?: string | null;
  onChange: (text: string) => void;
  onSave?: () => void;
  readOnly?: boolean;
}

/**
 * CodeMirror 6 React wrapper with a safe runtime fallback:
 * - Attempts to dynamically construct an EditorState and EditorView at runtime.
 * - If CodeMirror packages are not available, falls back to a simple <textarea>.
 *
 * This variant additionally wires Tree-sitter parsing:
 * - when a document mounts or the text prop changes, it triggers an async parse
 *   (full reparse) and applies decorations via applyDecorationsToView.
 *
 * Using dynamic imports avoids Vite failing when optional deps are not installed.
 */
export function CodeMirrorEditor(props: CodeMirrorEditorProps) {
  const { documentId, text, languageId, onChange, onSave, readOnly } = props;
  const containerRef = useRef<HTMLDivElement | null>(null);
  const viewRef = useRef<any | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const [usingFallback, setUsingFallback] = useState(false);

  // Initialize treesitter runtime in background (may throw if wasm not reachable).
  useEffect(() => {
    void initTreesitterOnce().catch(() => {
      // treesitter optional — errors are handled by parse functions
    });
  }, []);

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

        if (!state) {
          // createState is async and will dynamically import codemirror modules.
          state = await createState(
            text ?? '',
            {
              onChange: (t: string) => {
                onChange(t);
              },
            },
            languageId ?? undefined,
            documentId ?? undefined,
          );
        }

        if (destroyed) return;

        // Import EditorView at runtime using a literal import so Vite can resolve it.
        const viewModule = await import('@codemirror/view');
        if (!viewModule || !containerRef.current) {
          throw new Error('EditorView not available');
        }

        const mountedView = new viewModule.EditorView({
          state,
          parent: containerRef.current,
        });

        viewRef.current = mountedView;

        // Trigger an initial parse & decoration application (async).
        try {
          const specs = await parseAndComputeDecorations(text ?? '', languageId ?? '', documentId ?? text ?? '');
          if (!destroyed && viewRef.current) {
            await applyDecorationsToView(viewRef.current, specs);
          }
        } catch (err) {
          // parse errors are non-fatal for the editor
          // eslint-disable-next-line no-console
          console.debug('[codemirror] initial parse failed', err);
        }
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

  // Debounced parse effect: when the document text changes (prop or editing),
  // reparse and reapply decorations. Parsing is async and debounced to avoid churn.
  useEffect(() => {
    if (!viewRef.current) return;
    let active = true;
    const timer = window.setTimeout(async () => {
      try {
        const specs = await parseAndComputeDecorations(text ?? '', languageId ?? '', documentId ?? text ?? '');
        if (!active) return;
        await applyDecorationsToView(viewRef.current, specs);
      } catch (err) {
        // eslint-disable-next-line no-console
        console.debug('[codemirror] parse-and-decorate failed', err);
      }
    }, 200);

    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [text, languageId, documentId]);

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
