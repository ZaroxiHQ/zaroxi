import React, { useEffect, useRef, useState } from 'react';
import { createState, analyzeText, decideProfile, PROFILE_THRESHOLDS, APP_SYNC_ANNOT } from './codemirror/setup';
import { getLanguageSupportForPath } from './codemirror/languages/index';
import { EditorView } from '@codemirror/view';
import editorViewHost from '@/lib/session/EditorViewHost';
import { isDebug, debug, error as logError, setMountError, incrementStat } from '@/lib/logger';
import DocumentStore from '@/stores/DocumentStore';
import EditorSessionStore from '@/stores/EditorSessionStore';

/**
 * Minimal runtime instrumentation to produce an automatic snapshot of
 * the editor-related runtime state. This is intentionally tiny and safe:
 * - does not retain heavy objects
 * - exposes a single report object on window.__zaroxi_runtime_report
 * - logs a compact trace when an uncaught error / unhandled rejection occurs
 *
 * The instrumentation is only for diagnostics (temporary) and is designed
 * to be low-overhead until the crash can be reproduced and analyzed.
 */
try {
  const _w: any = typeof window !== 'undefined' ? (window as any) : undefined;
  if (_w) {
    /**
     * generateRuntimeReport(documentId?, tabId?)
     *
     * Collect best-effort diagnostics about the editor host, timers, store state,
     * and recent events. This function is safe to call anywhere and will never
     * throw (defensive try/catch).
     */
    _w.__zaroxi_generate_runtime_report = function (documentId?: string | null, tabId?: string | null) {
      try {
        const hostInfo = (typeof editorViewHost !== 'undefined' && typeof editorViewHost.inspect === 'function') ? (editorViewHost.inspect ? editorViewHost.inspect() : null) : null;
        const registry = (_w.__zaroxi_editor_view_report && typeof _w.__zaroxi_editor_view_report === 'function') ? _w.__zaroxi_editor_view_report() : { total: (_w.__zaroxi_editor_views || []).length, alive: null, entries: [] };
        const timers = (_w.__zaroxi_timers || []).slice(-300);
        const isScrolling = !!_w.__zaroxi_is_scrolling;
        const lastAdopt = _w.__zaroxi_last_adopt_global_ts || null;
        const lastEmit = _w.__zaroxi_last_editor_emit || null;
        let docRecord = null;
        let sessionRecord = null;
        try {
          if (typeof DocumentStore !== 'undefined' && documentId) {
            docRecord = DocumentStore.get(documentId) ?? null;
          }
        } catch {}
        try {
          if (typeof EditorSessionStore !== 'undefined' && tabId) {
            sessionRecord = EditorSessionStore.getSnapshot(tabId) ?? null;
          }
        } catch {}

        const perfMem = (typeof performance !== 'undefined' && (performance as any).memory) ? (performance as any).memory : null;
        const now = Date.now();

        const report = {
          ts: now,
          host: hostInfo,
          registry,
          timersCount: timers.length,
          timers: timers.slice(-50),
          isScrolling,
          lastAdopt,
          lastEmit,
          documentId: documentId ?? null,
          tabId: tabId ?? null,
          document: docRecord,
          session: sessionRecord,
          perfMemory: perfMem,
        };
        _w.__zaroxi_runtime_report = report;
        try { console.info('[zaroxi-runtime-report]', report); } catch {}
        try { localStorage.setItem('__zaroxi_runtime_report', JSON.stringify(report)); } catch {}
        try { window.dispatchEvent(new CustomEvent('zaroxi-runtime-report', { detail: report })); } catch {}
        return report;
      } catch (err) {
        try { console.error('[zaroxi-runtime-report] failed to collect', String(err)); } catch {}
        return { error: String(err) };
      }
    };

    // Hook global error handlers to emit a runtime report when an uncaught error/rejection happens.
    if (!_w.__zaroxi_runtime_report_hooked) {
      _w.__zaroxi_runtime_report_hooked = true;
      _w.addEventListener('error', (ev: any) => {
        try {
          const r = _w.__zaroxi_generate_runtime_report?.(null, null) ?? {};
          try { console.error('[zaroxi] uncaught error', ev && ev.error ? ev.error : ev, r); } catch {}
        } catch {}
      });
      _w.addEventListener('unhandledrejection', (ev: any) => {
        try {
          const r = _w.__zaroxi_generate_runtime_report?.(null, null) ?? {};
          try { console.error('[zaroxi] unhandledrejection', ev && ev.reason ? ev.reason : ev, r); } catch {}
        } catch {}
      });
      // Periodic low-rate snapshot to observe long-running growth (very cheap).
      try {
        setInterval(() => {
          try {
            _w.__zaroxi_generate_runtime_report?.(null, null);
          } catch {}
        }, 30_000);
      } catch {}
    }
  }
} catch {}

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

 // Dedupe in-flight language support loads to avoid duplicate dynamic imports
 // when the mount effect runs multiple times (e.g. React StrictMode double-mount).
 const languageLoadInflight: Map<string, Promise<any>> = new Map();

 // Minimal op ring buffer for last CM6 operations to help capture the right-before-crash path.
 // Stored on window to be picked up by the persisted runtime report. Kept bounded and non-retaining.
 try {
   const _w: any = typeof window !== 'undefined' ? (window as any) : undefined;
   if (_w) {
     _w.__zaroxi_last_ops = _w.__zaroxi_last_ops || [];
     _w.__zaroxi_push_op = function (op: string, meta?: any) {
       try {
         const entry = { op, ts: Date.now(), meta: meta ?? null };
         _w.__zaroxi_last_ops.push(entry);
         if (_w.__zaroxi_last_ops.length > 200) _w.__zaroxi_last_ops.shift();
       } catch {}
     };
     _w.__zaroxi_last_error = _w.__zaroxi_last_error || null;
   }
 } catch {}

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

        // Early guard: if a live view already exists for this document owner,
        // skip duplicate initialization. This prevents expensive duplicate work
        // when React StrictMode double-invokes mount effects in development.
        try {
          if (editorViewHost.getView(String(documentId ?? ''))) {
            try { debug && debug('[codemirror] skipping duplicate mount; view already exists for', documentId); } catch {}
            return;
          }
        } catch {}

        languageExtRef.current = null;

        // Deduplicate language support loading across concurrent mount runs.
        // Key strategy: prefer languageId when present; otherwise fall back to per-document key.
        if (profile === 'normal') {
          try {
            const langKey = (languageId ?? `doc:${String(documentId ?? '')}`);
            // record language load start
            try { (window as any).__zaroxi_push_op && (window as any).__zaroxi_push_op('language_load_start', { langKey }); } catch {}
            let langPromise = languageLoadInflight.get(langKey);
            if (!langPromise) {
              langPromise = (async () => {
                try {
                  const ext = await getLanguageSupportForPath(documentId ?? undefined, languageId ?? undefined);
                  return ext ?? null;
                } finally {
                  try { languageLoadInflight.delete(langKey); } catch {}
                }
              })();
              languageLoadInflight.set(langKey, langPromise);
            }
            languageExtRef.current = await langPromise;
            // record language load done
            try { (window as any).__zaroxi_push_op && (window as any).__zaroxi_push_op('language_load_done', { langKey }); } catch {}
          } catch {
            languageExtRef.current = null;
            try { (window as any).__zaroxi_push_op && (window as any).__zaroxi_push_op('language_load_failed', { documentId, languageId }); } catch {}
          }
        }

        // Build the EditorState with explicit profile and gutter decision.
        try { (window as any).__zaroxi_push_op && (window as any).__zaroxi_push_op('create_state_start', { documentId, profile }); } catch {}
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
        try { (window as any).__zaroxi_push_op && (window as any).__zaroxi_push_op('create_state_done', { documentId, profile }); } catch {}

        if (destroyed) return;

        // If a concurrent mount already created a live view for this owner while we awaited,
        // avoid creating a second view.
        try {
          if (editorViewHost.getView(String(documentId ?? ''))) {
            try { debug && debug('[codemirror] createView suppressed; another mount created view for', documentId); } catch {}
            return;
          }
        } catch {}

        // Create the view via the host (host ensures single live view).
        try { (window as any).__zaroxi_push_op && (window as any).__zaroxi_push_op('create_view_start', { documentId }); } catch {}
        const mountedView = editorViewHost.createView(String(documentId ?? ''), containerRef.current, (parent: Element) => {
          return new EditorView({
            state,
            parent,
          });
        });
        try { (window as any).__zaroxi_push_op && (window as any).__zaroxi_push_op('create_view_done', { documentId }); } catch {}

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
        try {
          const _w: any = typeof window !== 'undefined' ? (window as any) : undefined;
          if (_w) {
            _w.__zaroxi_push_op && _w.__zaroxi_push_op('mount_error', { documentId, err: String(err) });
            _w.__zaroxi_last_error = { message: (err && err.message) ? err.message : String(err), stack: (err && err.stack) ? err.stack : null, ts: Date.now() };
          }
        } catch {}
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
  // Make this path intentionally cheap: avoid synchronous full-text hashing
  // or calling `view.state.doc.toString()` here. Use only non-allocating guards
  // (length + small prefix via sliceString) and skip fallback to toString.
  // Stronger checks, if necessary, must run later on an idle/deferred path.
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
      const incoming = (text ?? '');

      // Cheap equality guard: compare lengths first.
      let identical = false;
      try {
        const docLen = (view.state.doc as any).length as number;
        const textLen = incoming.length;
        if (docLen === textLen) {
          // If sliceString is available, compare a small prefix only (no full serialization).
          const prefixLen = Math.min(64, docLen);
          if (typeof (view.state.doc as any).sliceString === 'function') {
            const docPrefix = (view.state.doc as any).sliceString(0, prefixLen);
            if (incoming.slice(0, prefixLen) === docPrefix) {
              identical = true;
            }
          } else {
            // Cannot safely sample the document without toString(); be conservative:
            // assume identical when lengths match to avoid forcing a heavy toString().
            // This may defer necessary replacements, but avoids large synchronous work.
            identical = true;
          }
        }
      } catch {
        // If anything fails, keep identical=false and let replacement occur as needed.
        identical = false;
      }

      // Skip if we've recently applied a programmatic update (short window).
      try {
        const lastProgTs = viewAny.__lastProgrammaticTs || 0;
        const RECENT_MS = 8000;
        if ((Date.now() - lastProgTs) < RECENT_MS && identical) {
          return;
        }
      } catch {}

      if (!identical) {
        // Record programmatic replace operation for diagnostics
        try { (window as any).__zaroxi_push_op && (window as any).__zaroxi_push_op('programmatic_replace', { documentId }); } catch {}
        // Programmatic replace: still necessary when incoming content materially differs.
        view.dispatch({
          changes: { from: 0, to: view.state.doc.length, insert: incoming ?? '' },
          annotations: APP_SYNC_ANNOT.of(true),
        });
        // Record programmatic application time to prevent immediate replays.
        try {
          viewAny.__lastProgrammaticTs = Date.now();
          // Clear marker after a reasonable timeout.
          setTimeout(() => {
            try {
              if (viewAny) {
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
