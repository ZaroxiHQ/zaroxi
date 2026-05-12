import React, { useEffect, useRef, useState } from 'react';
import { createState, analyzeText, decideProfile, PROFILE_THRESHOLDS, APP_SYNC_ANNOT } from './codemirror/setup';
import { getLanguageSupportForPath } from './codemirror/languages/index';
import { EditorView } from '@codemirror/view';
import editorViewHost from '@/lib/session/EditorViewHost';
import { isDebug, debug, error as logError, setMountError, incrementStat } from '@/lib/logger';

/**
 * Stable 32-bit-ish string hash used by several guard checks in the editor.
 * Kept local to avoid importing cross-file utilities and to be fast.
 */
function stableHashString(s: string): string {
  let h = 2166136261 >>> 0;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619) >>> 0;
  }
  return (h >>> 0).toString(16);
}

/**
 * Lightweight, non-invasive runtime instrumentation for EditorView lifecycle.
 *
 * Exposes:
 *  - window.__zaroxi_editor_views: array of weak refs and metadata for created views
 *  - window.__zaroxi_timers: array of recorded timer ids and metadata
 *  - window.__zaroxi_editor_view_report(): returns a compact live report
 *
 * These are diagnostics only and do not change editor behavior.
 *
 * Important: to avoid retaining live EditorView instances we only store a WeakRef
 * when available. On environments without WeakRef we store a null ref to ensure
 * we do not accidentally capture the view in a closure.
 */
try {
  const _w: any = typeof window !== 'undefined' ? (window as any) : undefined;
  if (_w) {
    _w.__zaroxi_editor_views = _w.__zaroxi_editor_views || [];
    _w.__zaroxi_timers = _w.__zaroxi_timers || [];
    _w.__zaroxi_editor_view_report = function () {
      try {
        const out: any = { total: 0, alive: 0, entries: [] };
        const now = Date.now();
        const arr = _w.__zaroxi_editor_views || [];
        out.total = arr.length;
        for (let i = 0; i < arr.length; i++) {
          const e = arr[i];
          const ref = e && e.ref;
          let alive = false;
          try {
            // Consider an entry alive only if it holds a WeakRef and the referent is reachable.
            if (ref && typeof ref.deref === 'function') {
              alive = !!ref.deref();
            } else {
              // No WeakRef present or no ref stored: assume not alive (do not treat as retention).
              alive = false;
            }
          } catch {
            alive = false;
          }
          if (alive) out.alive++;
          out.entries.push({
            documentId: e.documentId,
            createdAt: e.createdAt,
            alive,
            meta: e.meta ?? null,
            lastSeenAt: e.lastSeenAt ?? null,
          });
        }
        // timers snapshot (non-reactive)
        out.timers = (_w.__zaroxi_timers || []).slice(-200);
        out.reportedAt = now;
        return out;
      } catch (err) {
        return { error: String(err) };
      }
    };
  }
} catch {}

// Large-file thresholds (tunable)
const LARGE_FILE_BYTES = 5 * 1024 * 1024; // 5 MB
const LARGE_FILE_LINES = 100_000;
const LARGE_FILE_LINE_LENGTH = 50_000;

function cmStatCreated() {
  // Intentionally no-op in hot path to avoid stats-driven feedback loops and excessive logging.
}
function cmStatDestroyed() {
  // Intentionally no-op.
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
        // Base profile from metrics
        let profile = decideProfile(metrics);

        // Stress-mode override: apply a strict reduced-profile when the document
        // appears likely to trigger allocation / viewport pressure. This single
        // guarded decision keeps heuristics centralized and reversible.
        try {
          const STRESS_EXTREME_BYTES = PROFILE_THRESHOLDS.largeMaxBytes * 2;
          const STRESS_EXTREME_LINE = PROFILE_THRESHOLDS.largeMaxLineLength * 2;
          const lastAdopt = (typeof window !== 'undefined' && (window as any).__zaroxi_last_adopt_global_ts) ? (window as any).__zaroxi_last_adopt_global_ts : 0;
          const recentlyUnstable = (Date.now() - lastAdopt) < 3000;

          // Escalate to 'extreme' for pathological files (very large or very long lines).
          if (metrics.bytes > STRESS_EXTREME_BYTES || metrics.maxLineLength > STRESS_EXTREME_LINE) {
            profile = 'extreme';
          // Use 'large' profile for files that are large, contain long lines, or when the editor
          // has recently undergone adoption (unstable period).
          } else if (
            metrics.bytes > PROFILE_THRESHOLDS.largeMaxBytes ||
            metrics.lines > PROFILE_THRESHOLDS.largeMaxLines ||
            metrics.maxLineLength > PROFILE_THRESHOLDS.largeMaxLineLength ||
            recentlyUnstable
          ) {
            profile = 'large';
          }
        } catch {
          // If anything goes wrong, fall back to the conservative profile previously chosen.
        }

        // For extreme single-line files we may want to hide the gutter to avoid per-line bookkeeping.
        const showGutter = !(metrics.maxLineLength > PROFILE_THRESHOLDS.extremeNoGutterLineLength || profile === 'extreme' && metrics.maxLineLength > PROFILE_THRESHOLDS.largeMaxLineLength);

        debug('[codemirror] mount profile', { profile, metrics });

        languageExtRef.current = null;

        // Load language support for NORMAL profile immediately (restoring pre-188496e behaviour).
        // Keep this synchronous from the mount's perspective so syntax is available promptly.
        languageExtRef.current = null;
        if (profile === 'normal') {
          try {
            const ext = await getLanguageSupportForPath(documentId ?? undefined, languageId ?? undefined);
            languageExtRef.current = ext ?? null;
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

        // Instrumentation: register created view with a global diagnostics list.
        // IMPORTANT: store only a WeakRef where available to avoid retaining the view.
        try {
          const _w: any = typeof window !== 'undefined' ? (window as any) : undefined;
          if (_w) {
            const ref = (typeof WeakRef !== 'undefined') ? new WeakRef(mountedView) : null;
            _w.__zaroxi_editor_views = _w.__zaroxi_editor_views || [];
            _w.__zaroxi_editor_views.push({
              documentId: String(documentId ?? ''),
              ref,
              createdAt: Date.now(),
              lastSeenAt: Date.now(),
              meta: { profile: (typeof profile !== 'undefined' ? profile : null) },
            });
            // Opportunistically prune dead weak refs to keep the registry bounded.
            try {
              _w.__zaroxi_editor_views = (_w.__zaroxi_editor_views || []).filter((entry: any) => {
                try {
                  if (!entry) return false;
                  const r = entry.ref;
                  if (!r) return true; // keep entries without WeakRef (they don't retain view)
                  return !!r.deref();
                } catch {
                  return true;
                }
              });
            } catch {}
          }
        } catch {}

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
        // Instrumentation: mark view destroyed in global diagnostic list before host destroy.
        try {
          const _w: any = typeof window !== 'undefined' ? (window as any) : undefined;
          if (_w && _w.__zaroxi_editor_views) {
            for (const e of _w.__zaroxi_editor_views) {
              try {
                if (e && e.documentId === String(documentId ?? '')) {
                  e.lastSeenAt = Date.now();
                  e._destroyRequestedAt = Date.now();
                }
              } catch {}
            }
          }
        } catch {}

        // Ensure host destroys any view for this document and prunes diagnostics.
        editorViewHost.destroyIfFor(String(documentId ?? ''));

        // After destroyIfFor attempt, mark as likely destroyed (best-effort).
        try {
          const _w: any = typeof window !== 'undefined' ? (window as any) : undefined;
          if (_w && _w.__zaroxi_editor_views) {
            for (const e of _w.__zaroxi_editor_views) {
              try {
                if (e && e.documentId === String(documentId ?? '')) {
                  e.lastSeenAt = Date.now();
                  e._destroyedAt = Date.now();
                }
              } catch {}
            }
          }
        } catch {}
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
      const viewAny: any = view;
      // Compute incoming normalized text and its hash for equality guards.
      const incoming = (text ?? '');
      const normalizeForHash = (s: string) => {
        try {
          let n = s.replace(/\r\n/g, '\n').replace(/\r/g, '\n');
          if (n.endsWith('\n')) n = n.slice(0, -1);
          return n;
        } catch {
          return s;
        }
      };
      const incomingHash = stableHashString(incoming);
      const incomingNormHash = stableHashString(normalizeForHash(incoming));

      // If this view recently applied a programmatic update with the same exact or normalized hash,
      // skip re-applying the same content to avoid ping-pong between persistence and editor.
      try {
        const lastProgHash = viewAny.__lastProgrammaticHash;
        const lastProgNorm = viewAny.__lastProgrammaticNormHash;
        const lastProgTs = viewAny.__lastProgrammaticTs || 0;
        const RECENT_MS = 8000;
        if (lastProgHash && (lastProgHash === incomingHash) && (Date.now() - lastProgTs) < RECENT_MS) {
          return;
        }
        if (lastProgNorm && (lastProgNorm === incomingNormHash) && (Date.now() - lastProgTs) < RECENT_MS) {
          return;
        }
      } catch {}

      // Avoid a full doc.toString() in the hot path. First do a cheap length + prefix check.
      let identical = false;
      try {
        const docLen = (view.state.doc as any).length as number;
        const textLen = (incoming ?? '').length;
        if (docLen === textLen) {
          const prefixLen = Math.min(64, docLen);
          const docPrefix = (view.state.doc as any).sliceString
            ? (view.state.doc as any).sliceString(0, prefixLen)
            : view.state.doc.toString().slice(0, prefixLen);
          if ((incoming ?? '').slice(0, prefixLen) === docPrefix) {
            // Cheap equality likely; treat as identical and avoid a full-replace.
            identical = true;
          }
        }
      } catch {
        // Fallback: if our cheap check failed, fall back to full string compare.
      }

      if (!identical) {
        // Replace the full document content in a single transaction.
        // Mark this transaction with APP_SYNC_ANNOT so the update listener can
        // ignore it and we avoid re-entering the parent change pipeline.
        view.dispatch({
          changes: { from: 0, to: view.state.doc.length, insert: incoming ?? '' },
          annotations: APP_SYNC_ANNOT.of(true),
        });
        // Record that we programmatically applied this exact content so subsequent
        // incoming normalized echoes from persistence can be suppressed briefly.
        try {
          viewAny.__lastProgrammaticHash = incomingHash;
          viewAny.__lastProgrammaticNormHash = incomingNormHash;
          viewAny.__lastProgrammaticTs = Date.now();
          // Clear after a timeout to avoid permanent suppression.
          setTimeout(() => {
            try {
              if (viewAny) {
                viewAny.__lastProgrammaticHash = undefined;
                viewAny.__lastProgrammaticNormHash = undefined;
                viewAny.__lastProgrammaticTs = undefined;
              }
            } catch {}
          }, 10000);
        } catch {}
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
