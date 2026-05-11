import React, { useEffect, useRef, useState } from 'react';
import { createState, createBaseExtensions } from './codemirror/setup';
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
        // Do NOT reuse cached EditorState here. Cached states may have been created by older
        // code paths without the mandatory lineNumbers() extension which causes the gutter
        // to be invisible. Always create a fresh EditorState via createState so the active
        // extensions (including lineNumbers) are present.
        let state = undefined;

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
          // Attempt to inspect the base extensions before creating the EditorState so we can
          // surface any errors originating from extension construction (e.g. theming/style code).
          try {
            const debugExts = createBaseExtensions(
              { onChange: (_t: string) => {} },
              languageExtRef.current ?? undefined,
              documentId ?? undefined,
            );
            // eslint-disable-next-line no-console
            console.debug('[codemirror] createBaseExtensions debug', {
              documentId,
              languageLoaded: !!languageExtRef.current,
              extensionsCount: Array.isArray(debugExts) ? debugExts.length : 'unknown',
            });
            try {
              if (Array.isArray(debugExts)) {
                debugExts.forEach((ext, i) => {
                  // eslint-disable-next-line no-console
                  console.debug('[codemirror] ext[' + i + ']:', {
                    type: typeof ext,
                    toString: ext && (ext as any).toString ? (ext as any).toString() : undefined,
                    inspect: ext && typeof ext === 'object' ? Object.keys(ext as any).slice(0, 10) : undefined,
                  });
                });
              }
            } catch (inner) {
              // eslint-disable-next-line no-console
              console.debug('[codemirror] extension inspection failed', inner);
            }
          } catch (e) {
            // eslint-disable-next-line no-console
            console.error('[codemirror] createBaseExtensions threw before state creation', e);
          }

          // createState constructs a state with the provided language extension.
          // createState is synchronous in this module; use it directly.
          try {
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
          } catch (e) {
            // eslint-disable-next-line no-console
            console.error('[codemirror] createState threw', e);
            throw e; // rethrow so outer catch triggers fallback behavior
          }
        }

        // Debugging: report state creation details so we can inspect why gutters may not appear.
        try {
          // Avoid strict type assumptions by using `as any` for diagnostics.
          // eslint-disable-next-line no-console
          console.debug('[codemirror] state created', {
            documentId,
            languageLoaded: !!languageExtRef.current,
            docLength: (state as any).doc?.length ?? 'unknown',
            // extensions count is an internal detail; attempt best-effort inspection
            extensionsCount: (state as any).config?.extensions?.length ?? (state as any).extensions?.length ?? 'unknown',
          });
        } catch (e) {
          // eslint-disable-next-line no-console
          console.debug('[codemirror] state debug info failed', e);
        }

        if (destroyed) return;

        let mountedView: any = null;
        try {
          mountedView = new EditorView({
            state,
            parent: containerRef.current,
          });
          viewRef.current = mountedView;
        } catch (e) {
          // Capture and report detailed diagnostics if EditorView construction fails.
          try {
            // eslint-disable-next-line no-console
            console.error('[codemirror] EditorView constructor threw', e, {
              documentId,
              languageLoaded: !!languageExtRef.current,
              stateSnapshot: {
                docLength: state ? (state as any).doc?.length ?? 'unknown' : 'no-state',
                // Best effort: inspect extensions array if present on the state
                extensionsCount: state && (state as any).config
                  ? (state as any).config.extensions?.length ?? 'unknown'
                  : (state as any).extensions?.length ?? 'unknown',
              },
            });
          } catch (diagErr) {
            // eslint-disable-next-line no-console
            console.error('[codemirror] failed to log EditorView diagnostics', diagErr);
          }
          throw e;
        }

        // Focus the editor so caret and wheel interactions behave consistently.
        try {
          mountedView.focus();
        } catch {
          // ignore focus errors in environments that don't support it
        }

        // Debugging: inspect DOM and computed styles immediately after mount.
        try {
          const hasGutters = !!mountedView.dom.querySelector('.cm-gutters');
          const guttersEl = mountedView.dom.querySelector('.cm-gutters');
          const containerClass = containerRef.current?.className ?? null;
          const containerStyle = containerRef.current ? window.getComputedStyle(containerRef.current) : null;
          const guttersStyle = guttersEl ? window.getComputedStyle(guttersEl as Element) : null;
          // eslint-disable-next-line no-console
          console.debug('[codemirror] mounted EditorView', {
            documentId,
            languageLoaded: !!languageExtRef.current,
            hasGutters,
            containerClass,
            containerWidth: containerStyle?.width ?? null,
            containerOverflow: containerStyle?.overflow ?? null,
            guttersDisplay: guttersStyle?.display ?? null,
            guttersWidth: guttersStyle?.width ?? null,
          });
        } catch (e) {
          // eslint-disable-next-line no-console
          console.debug('[codemirror] post-mount DOM inspection failed', e);
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
              console.debug('[codemirror] gutter missing on initial mount; reconfiguring state to include lineNumbers extension');
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
                // Post-reconfiguration check: report whether gutters are now present.
                const nowHas = !!mountedView.dom.querySelector('.cm-gutters');
                // eslint-disable-next-line no-console
                console.debug('[codemirror] after reconfigure hasGutters=', nowHas);
              } catch (e) {
                // eslint-disable-next-line no-console
                console.debug('[codemirror] failed to reconfigure state for gutter', e);
              }
            } else {
              // eslint-disable-next-line no-console
              console.debug('[codemirror] gutter present on initial mount; no reconfigure needed');
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
