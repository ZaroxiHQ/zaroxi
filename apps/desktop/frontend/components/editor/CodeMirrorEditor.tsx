import React, { useEffect, useRef, useState } from 'react';
import { createState, analyzeText, decideProfile, PROFILE_THRESHOLDS } from './codemirror/setup';
import { getLanguageSupportForPath } from './codemirror/languages/index';
import { EditorView } from '@codemirror/view';
import editorViewHost from '@/lib/session/EditorViewHost';
import { isDebug, debug, error as logError, setMountError, incrementStat } from '@/lib/logger';

// Large-file thresholds (tunable)
const LARGE_FILE_BYTES = 5 * 1024 * 1024; // 5 MB
const LARGE_FILE_LINES = 100_000;
const LARGE_FILE_LINE_LENGTH = 50_000;

function cmStatCreated() {
  try {
    incrementStat('created', 1);
    incrementStat('live', 1);
  } catch {}
}
function cmStatDestroyed() {
  try {
    incrementStat('destroyed', 1);
    incrementStat('live', -1);
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
 * CodeMirror 6 React wrapper (simplified).
 *
 * Key properties:
 * - One active EditorView is created per mounted component via editorViewHost.
 * - EditorState is created once on mount or when documentId / language change (not on every text change).
 * - For very large files we enter a stable "large-file" preview mode:
 *   - editor is made read-only
 *   - gutter (line numbers) remains visible
 *   - syntax highlighting is disabled for extreme files to avoid parser/render cost
 *
 * This implementation avoids heavy diagnostics and avoids serializing the full document
 * in hot update paths for large files.
 */
export function CodeMirrorEditor(props: CodeMirrorEditorProps) {
  const { documentId, text, languageId, onChange, readOnly } = props;
  const containerRef = useRef<HTMLDivElement | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const [usingFallback, setUsingFallback] = useState(false);

  // Hold the currently attached EditorView owner via the host (host enforces single live view).
  // We do not keep a local strong reference to allow the host to manage lifecycle.
  const languageExtRef = useRef<any | null>(null);

  // File profile decision is handled centrally by codemirror/setup.ts (analyzeText + decideProfile).
  // We keep this spot intentionally empty so the editor can ask setup for a profile.

  // Create/mount EditorView when documentId or language changes.
  useEffect(() => {
    if (!containerRef.current) return;

    let destroyed = false;

    (async () => {
      try {
        // Measure file metrics and decide a safe profile.
        const metrics = analyzeText(text ?? '');
        const profile = decideProfile(metrics);
        // For extreme single-line files we may want to hide the gutter to avoid per-line bookkeeping.
        const showGutter = !(metrics.maxLineLength > PROFILE_THRESHOLDS.extremeNoGutterLineLength || profile === 'extreme' && metrics.maxLineLength > PROFILE_THRESHOLDS.largeMaxLineLength);

        debug('[codemirror] mount profile', { profile, metrics });

        languageExtRef.current = null;

        // Only load language support for NORMAL profile (avoid heavy parser work for large/extreme)
        if (profile === 'normal') {
          try {
            languageExtRef.current = await getLanguageSupportForPath(documentId ?? undefined, languageId ?? undefined);
          } catch {
            languageExtRef.current = null;
          }
        }

        // Build the EditorState with explicit profile and gutter decision.
        const state = createState(
          text ?? '',
          {
            onChange: (t: string) => {
              // Only forward onChange for the NORMAL profile and when editable.
              if (profile === 'normal' && !readOnly) {
                try {
                  onChange(t);
                } catch {}
              }
            },
          },
          // Pass language extension only for normal profile (or when caller deliberately provided one).
          profile === 'normal' ? languageExtRef.current ?? undefined : undefined,
          String(documentId ?? ''),
          profile,
          showGutter,
        );

        if (destroyed) return;

        // Create the view via the host (host ensures single live view).
        const mountedView = editorViewHost.createView(String(documentId ?? ''), containerRef.current, (parent: Element) => {
          return new EditorView({
            state,
            parent,
          });
        });

        cmStatCreated();

        // Focus editor when possible
        try { mountedView.focus(); } catch {}

      } catch (err) {
        try { logError('[codemirror] mount failed:', String(err)); } catch {}
        try { setMountError(err); } catch {}
        setUsingFallback(true);
      }
    })();

    return () => {
      destroyed = true;
      try {
        editorViewHost.destroyIfFor(String(documentId ?? 'unknown'));
      } catch {}
      cmStatDestroyed();
    };
    // Only recreate when the document identity or language hint changes.
    // Do NOT include `text` here to avoid re-creating the EditorView on every keystroke.
  }, [documentId, languageId, readOnly]);

  // Keep the live EditorView document in sync when `text` prop changes.
  // This dispatch is intentionally minimal: we only replace the document when
  // the incoming text differs from the current view content.
  useEffect(() => {
    if (usingFallback) {
      if (textareaRef.current && textareaRef.current.value !== text) {
        textareaRef.current.value = text ?? '';
      }
      return;
    }

    const view = editorViewHost.getView(String(documentId ?? '')) as any;
    if (!view) return;

    try {
      const current = view.state.doc.toString();
      if (text !== current) {
        // Replace the full document content in a single transaction.
        view.dispatch({
          changes: { from: 0, to: view.state.doc.length, insert: text ?? '' },
        });
      }
    } catch {
      // Defensive: ignore failures (view might be destroyed concurrently).
    }
  }, [text, usingFallback, documentId]);

  // Render fallback textarea if we failed to mount CodeMirror.
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

  return <div ref={containerRef} className="h-full w-full min-h-0 min-w-0" />;
}

export default CodeMirrorEditor;
