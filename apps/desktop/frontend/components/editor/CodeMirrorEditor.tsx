import React, { useEffect, useRef, useState } from 'react';
import { createState, createBaseExtensions } from './codemirror/setup';
import { getLanguageSupportForPath } from './codemirror/languages/index';
import { EditorView } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { lineNumbers } from '@codemirror/gutter';
import editorViewHost from '@/lib/session/EditorViewHost';

// Large-file thresholds (tunable)
const LARGE_FILE_BYTES = 5 * 1024 * 1024; // 5 MB
const LARGE_FILE_LINES = 100_000;
const LARGE_FILE_LINE_LENGTH = 50_000;

/**
 * Small runtime instrumentation helpers exposed on window.__zaroxi_cm_stats:
 *  - created: total EditorView constructed
 *  - destroyed: total EditorView destroyed
 *  - live: currently live EditorView instances
 *  - createdByDoc / destroyedByDoc: per-document counters
 *  - cachedStates: number of cached engine states (updated by editorEngine.setCachedState)
 *
 * These counters are intended for debugging and verification only and are written
 * in a best-effort, non-throwing manner.
 */
function ensureCmStats() {
  try {
    const w: any = window as any;
    if (!w.__zaroxi_cm_stats) {
      w.__zaroxi_cm_stats = {
        created: 0,
        destroyed: 0,
        live: 0,
        // bounded per-document summaries to avoid unbounded growth in long-running sessions
        createdByDoc: Object.create(null),
        destroyedByDoc: Object.create(null),
        // counters for creations/destroys that exceed the per-doc key cap
        createdOtherDocs: 0,
        destroyedOtherDocs: 0,
        tabSwitches: 0,
        cachedStates: 0,
      };
    }
    return w.__zaroxi_cm_stats;
  } catch {
    return {
      created: 0,
      destroyed: 0,
      live: 0,
      createdByDoc: Object.create(null),
      destroyedByDoc: Object.create(null),
      createdOtherDocs: 0,
      destroyedOtherDocs: 0,
      tabSwitches: 0,
      cachedStates: 0,
    };
  }
}

// Cap how many unique document keys we will keep detailed counters for.
// Keeping this finite prevents unbounded memory growth when many different files are opened.
const CM_STATS_DOC_KEY_LIMIT = 200;

function cmStatCreated(key: string) {
  try {
    const s: any = ensureCmStats();
    s.created += 1;
    s.live += 1;

    const map = s.createdByDoc;
    if (Object.prototype.hasOwnProperty.call(map, key)) {
      map[key] += 1;
    } else if (Object.keys(map).length < CM_STATS_DOC_KEY_LIMIT) {
      map[key] = 1;
    } else {
      s.createdOtherDocs = (s.createdOtherDocs || 0) + 1;
    }

    // Do not log on every creation to avoid hot-path noise; if needed, inspect
    // window.__zaroxi_cm_stats from the console or add on-demand diagnostics.
  } catch {}
}

function cmStatDestroyed(key: string) {
  try {
    const s: any = ensureCmStats();
    s.destroyed += 1;
    s.live = Math.max(0, (s.live || 1) - 1);

    const map = s.destroyedByDoc;
    if (Object.prototype.hasOwnProperty.call(map, key)) {
      map[key] += 1;
    } else if (Object.keys(map).length < CM_STATS_DOC_KEY_LIMIT) {
      map[key] = 1;
    } else {
      s.destroyedOtherDocs = (s.destroyedOtherDocs || 0) + 1;
    }
  } catch {}
}

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
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const [usingFallback, setUsingFallback] = useState(false);
  // Hold the loaded language extension so we can re-create/reconfigure EditorState if needed.
  const languageExtRef = useRef<any | null>(null);

  // Mount CodeMirror EditorView (async). If dynamic imports fail, flip to textarea fallback.
  useEffect(() => {
    if (!containerRef.current) return;

    let destroyed = false;
    let gutterTimer: number | null = null;

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
        // Determine whether to enable a lightweight "large-file" safe mode.
        let largeFileMode = false;
        try {
          // Compute simple heuristics from the provided text to avoid heavy work on open.
          const bytes = (typeof text === 'string') ? (new TextEncoder().encode(text).length) : 0;
          const lines = (typeof text === 'string') ? (text.split('\n').length) : 0;
          let maxLine = 0;
          if (typeof text === 'string') {
            let idx = 0;
            while (idx < text.length) {
              const next = text.indexOf('\n', idx);
              const end = next === -1 ? text.length : next;
              const len = end - idx;
              if (len > maxLine) maxLine = len;
              if (next === -1) break;
              idx = next + 1;
            }
          }
          if (bytes > LARGE_FILE_BYTES || lines > LARGE_FILE_LINES || maxLine > LARGE_FILE_LINE_LENGTH) {
            largeFileMode = true;
            languageExtRef.current = null;
            // eslint-disable-next-line no-console
            console.info('[codemirror] largeFileMode enabled', { documentId, bytes, lines, maxLine });
          } else {
            languageExtRef.current = await getLanguageSupportForPath(documentId ?? undefined, languageId ?? undefined);
          }
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
          // For extremely large files we build a minimal, safe EditorState to avoid expensive processing.
          try {
            if (largeFileMode) {
              // Minimal extension set for large-file safe mode.
              const extensions = [
                lineNumbers(),
                // Make editor read-only for extremely large files to avoid heavy onChange serialisation.
                EditorView.editable.of(false),
                // Minimal update listener (no heavy doc serialization).
                EditorView.updateListener.of((v: any) => {
                  // Intentionally minimal: do not call onChange; avoid full doc.toString() on every edit.
                  // Keep listener sparse for potential future lightweight diagnostics.
                  if (v.docChanged) {
                    try {
                      const s: any = (window as any).__zaroxi_cm_stats || {};
                      s.largeFileDocChanges = (s.largeFileDocChanges || 0) + 1;
                      (window as any).__zaroxi_cm_stats = s;
                    } catch {}
                  }
                }),
              ];
              state = EditorState.create({ doc: text ?? '', extensions });
            } else {
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
          // Create the EditorView via the centralized host so it becomes the single owner.
          mountedView = editorViewHost.createView(String(documentId ?? ''), containerRef.current!, (parent: Element) => {
            return new EditorView({
              state,
              parent,
            });
          });
          // The host owns the live view; do not retain a local ref to the EditorView.
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

        // Debugging: inspect DOM, extensions and computed styles immediately after mount.
        try {
          const hasGutters = !!mountedView.dom.querySelector('.cm-gutters');
          const guttersEl = mountedView.dom.querySelector('.cm-gutters');
          const containerClass = containerRef.current?.className ?? null;
          const containerStyle = containerRef.current ? window.getComputedStyle(containerRef.current) : null;
          const guttersStyle = guttersEl ? window.getComputedStyle(guttersEl as Element) : null;

          // Inspect state extensions (best-effort)
          let extDiagnostics: any = null;
          let hasSyntaxHighlightingExtension = false;
          let hasLanguageExtensionPresent = false;
          try {
            const exts = (mountedView.state as any)?.config?.extensions ?? (mountedView.state as any)?.extensions ?? null;
            if (Array.isArray(exts)) {
              const mapped = exts.slice(0, 50).map((ext: any, i: number) => {
                let name = (ext && (ext as any).constructor && (ext as any).constructor.name) || typeof ext;
                let str = undefined;
                try {
                  str = (ext && typeof ext.toString === 'function') ? (ext as any).toString().slice(0, 200) : undefined;
                } catch {}
                let keys = undefined;
                try {
                  if (ext && typeof ext === 'object') keys = Object.keys(ext).slice(0, 10);
                } catch {}
                const lowerName = (name || '').toString().toLowerCase();
                const lowerStr = (str || '').toString().toLowerCase();
                // Heuristics for detecting highlight / language extensions
                const likelyLineNumbers = !!(lowerStr && lowerStr.includes('linenumber')) || (lowerName && lowerName.includes('linenumber'));
                const likelyHighlight = !!(lowerStr && (lowerStr.includes('highlight') || lowerStr.includes('highlightstyle') || lowerStr.includes('syntaxhighlighting'))) || (lowerName && (lowerName.includes('highlight') || lowerName.includes('highlightstyle')));
                const likelyLanguageSupport = !!(lowerName && (lowerName.includes('languagesupport') || lowerName.includes('language'))) || (lowerStr && lowerStr.includes('language'));
                if (likelyHighlight) hasSyntaxHighlightingExtension = true;
                if (likelyLanguageSupport) hasLanguageExtensionPresent = true;
                return { index: i, name, likelyLineNumbers, likelyHighlight, likelyLanguageSupport, keys, toString: str };
              });
              extDiagnostics = mapped;
            } else {
              extDiagnostics = { type: typeof exts, value: exts };
            }
          } catch (e) {
            extDiagnostics = { error: String(e) };
          }

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
            extensionsPreview: extDiagnostics,
            hasSyntaxHighlightingExtension,
            hasLanguageExtension: hasLanguageExtensionPresent,
          });

          // If gutters are missing, capture a snapshot of the DOM and state to help debugging.
          if (!hasGutters) {
            try {
              const domSnapshot = mountedView.dom.outerHTML?.slice(0, 2000);
              // Also attempt to detect whether the lineNumbers() extension exists in the base extensions factory.
              let baseExtPreview = null;
              try {
                const dbg = createBaseExtensions(
                  { onChange: (_t: string) => {} },
                  languageExtRef.current ?? undefined,
                  documentId ?? undefined,
                );
                if (Array.isArray(dbg)) {
                  baseExtPreview = dbg.slice(0, 50).map((ext: any, i: number) => {
                    let name = (ext && (ext as any).constructor && (ext as any).constructor.name) || typeof ext;
                    let str = undefined;
                    try {
                      str = (ext && typeof ext.toString === 'function') ? (ext as any).toString().slice(0, 200) : undefined;
                    } catch {}
                    const likelyLineNumbers = !!(str && str.toLowerCase().includes('linenumber')) || (name && name.toLowerCase().includes('linenumber'));
                    return { index: i, name, likelyLineNumbers, toString: str };
                  });
                } else {
                  baseExtPreview = { type: typeof dbg, value: dbg };
                }
              } catch (e) {
                baseExtPreview = { error: String(e) };
              }

              // eslint-disable-next-line no-console
              console.debug('[codemirror] GUTTER MISSING: mountedView DOM snapshot (truncated), baseExtensions preview:', {
                domSnapshot,
                baseExtPreview,
                languageExtRef: !!languageExtRef.current,
              });
            } catch (e) {
              // eslint-disable-next-line no-console
              console.debug('[codemirror] failed to capture DOM/state snapshot when gutter missing', e);
            }
          } else {
            // If gutters exist, provide a small content preview for verification.
            try {
              const gutterHTML = (guttersEl as Element)?.innerHTML?.slice(0, 1000) ?? null;
              const gutterChildren = guttersEl ? (guttersEl.querySelectorAll('.cm-gutterElement')?.length ?? 0) : 0;
              const firstLabels: string[] = [];
              if (guttersEl) {
                const els = guttersEl.querySelectorAll('.cm-gutterElement');
                for (let i = 0; i < Math.min(5, els.length); i++) {
                  firstLabels.push((els[i] as Element).textContent?.trim() ?? '');
                }
              }
              // eslint-disable-next-line no-console
              console.debug('[codemirror] GUTTERS PRESENT preview', { gutterChildren, firstLabels, gutterHTML });
            } catch (e) {
              // eslint-disable-next-line no-console
              console.debug('[codemirror] failed to snapshot gutter content', e);
            }
          }

          // Ensure the gutter column is visible even if global CSS attempts to collapse it.
          // Apply a minimal set of inline styles derived from theme CSS variables with safe fallbacks.
          const applyGutterStyles = (el: Element | null) => {
            if (!el || !(el as HTMLElement).style) return;
            try {
              const htmlRoot = document.documentElement;
              const rootStyle = window.getComputedStyle(htmlRoot);
              const gutterBg = (rootStyle.getPropertyValue('--color-editor-gutter-background') || rootStyle.getPropertyValue('--color-editor-background') || '#1E1F24').trim();
              const gutterColor = (rootStyle.getPropertyValue('--color-text-faint') || rootStyle.getPropertyValue('--color-text-on-surface') || '#7E8794').trim();

              const style = (el as HTMLElement).style;
              style.minWidth = '40px';
              style.width = 'auto';
              style.overflow = 'visible';
              style.display = 'block';
              style.background = gutterBg;
              style.color = gutterColor;
              // Ensure gutter elements that are children also have visible color/padding
              const gutterElems = el.querySelectorAll('.cm-gutterElement');
              gutterElems.forEach((ge) => {
                try {
                  const s = (ge as HTMLElement).style;
                  if (!s.paddingRight) s.paddingRight = '6px';
                  if (!s.color) s.color = gutterColor;
                } catch {
                  // ignore per-element errors
                }
              });
            } catch (e) {
              // eslint-disable-next-line no-console
              console.debug('[codemirror] applyGutterStyles failed', e);
            }
          };

          applyGutterStyles(guttersEl);
        } catch (e) {
          // eslint-disable-next-line no-console
          console.debug('[codemirror] post-mount DOM inspection failed', e);
        }

        // Post-mount gutter sanity: log presence but do not auto-reconfigure or mutate DOM.
        gutterTimer = window.setTimeout(() => {
          try {
            const hasGutters = !!mountedView.dom.querySelector('.cm-gutters');
            // eslint-disable-next-line no-console
            console.debug('[codemirror] gutter check after mount', { documentId, hasGutters });
          } catch (e) {
            // eslint-disable-next-line no-console
            console.debug('[codemirror] gutter recheck failed', e);
          }
        }, 50);
      } catch (err) {
        // Fallback path: show a native textarea bound to the document text.
        // This keeps the app usable when codemirror packages are missing.
        // Make failures explicit and expose debug info for diagnostics.
        // eslint-disable-next-line no-console
        console.error('[codemirror] falling back to textarea (dynamic import failed)', err);
        try {
          (window as any).__codemirror_mount_error = { message: String((err as any)?.message ?? err), stack: (err as any)?.stack ?? null };
        } catch {}
        setUsingFallback(true);
      }
    })();

    return () => {
      destroyed = true;
      try { editorViewHost.destroyIfFor(String(documentId ?? 'unknown')); } catch {}
      // Clear any scheduled gutter sanity timer
      if (gutterTimer) {
        try {
          window.clearTimeout(gutterTimer);
        } catch {}
        gutterTimer = null;
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
    const view = editorViewHost.getView(documentId ?? '') as any;
    if (!view) return;
    try {
      const current = view.state.doc.toString();
      if (text !== current) {
        view.dispatch({
          changes: { from: 0, to: view.state.doc.length, insert: text ?? '' },
        });
      }
    } catch {
      // view might not be a CodeMirror view (defensive)
    }
  }, [text, usingFallback, documentId]);

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
