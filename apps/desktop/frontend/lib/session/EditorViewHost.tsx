/**
 * EditorViewHost
 *
 * Small helper that centralizes lifetime of editor view instances.
 * Consumers can ask the host to attach a newly-created view and the host
 * will ensure previous view is destroyed, and provide a safe destroy() call.
 *
 * The host intentionally does not construct EditorView internals here.
 * Instead, callers create a view (or factory) and pass it to attach().
 *
 * This enforces a single place to `destroy()` view instances on demotion.
 */

import EditorSessionStore from '@/stores/EditorSessionStore';
import { isDebug, incrementStat, setInspect } from '@/lib/logger';

type AnyView = { destroy?: () => void } | null;

/**
 * EditorViewHost
 *
 * Centralized owner/creator of live EditorView instances.
 *
 * Guarantees:
 *  - Only the host may construct and retain a live view reference.
 *  - createView destroys the previous live view when exceeding the live cap.
 *  - destroyIfFor(id) is idempotent and accepts either a tabId or a documentId.
 *  - Lightweight, non-reactive runtime diagnostics are updated here.
 */
class EditorViewHost {
  private currentView: AnyView | null = null;
  private currentOwnerId: string | null = null; // canonical id supplied by creator (often documentId)
  private maxLiveViews: number = 1; // hard guard: only allow 1 live view by default

  // Prune diagnostic registry entries that may hold strong references to destroyed views.
  // This is defensive: the diagnostics registry is useful for debugging but must not
  // itself keep EditorView instances alive. Remove entries for the specified owner id.
  private _pruneDiagnosticsForOwner(ownerId: string | null) {
    try {
      if (!ownerId) return;
      const _w: any = (typeof window !== 'undefined') ? (window as any) : undefined;
      if (!_w) return;
      if (!_w.__zaroxi_editor_views) return;
      try {
        _w.__zaroxi_editor_views = (_w.__zaroxi_editor_views || []).filter((e: any) => {
          try {
            return !(e && String(e.documentId) === String(ownerId));
          } catch {
            return true;
          }
        });
      } catch {}
    } catch {}
  }

  // Create a view using a factory that receives the parent DOM element.
  // The host becomes the authoritative owner of the returned view.
  createView(ownerId: string, parent: Element, factory: (parent: Element) => AnyView): AnyView {
    // If a different view is attached, or an existing view exists for the same owner,
    // destroy it first to enforce single owner and to free any retained resources.
    if (this.currentView) {
      if (this.currentOwnerId !== ownerId) {
        try {
          if (typeof this.currentView.destroy === 'function') {
            this.currentView.destroy();
          }
        } catch {
          // swallow
        }
        const oldOwner = this.currentOwnerId;
        this.currentView = null;
        this.currentOwnerId = null;
        // Prune diagnostics for the old owner to avoid the registry keeping references.
        this._pruneDiagnosticsForOwner(oldOwner);
        // instrumentation: update non-reactive stats (debug-only)
        try {
          if (isDebug()) {
            incrementStat('destroyed', 1);
            incrementStat('live', -1);
          }
        } catch {}
      } else {
        // Same owner: replace existing view deterministically.
        try {
          if (typeof this.currentView.destroy === 'function') {
            this.currentView.destroy();
          }
        } catch {}
        this._pruneDiagnosticsForOwner(ownerId);
        this.currentView = null;
        this.currentOwnerId = null;
        try {
          if (isDebug()) {
            incrementStat('destroyed', 1);
            incrementStat('live', -1);
          }
        } catch {}
      }
    }

    // Create the view. Caller is responsible for providing a stable factory.
    const created = factory(parent);

    this.currentView = created;
    this.currentOwnerId = ownerId;

    // instrumentation: update non-reactive stats (debug-only)
    try {
      if (isDebug()) {
        incrementStat('created', 1);
        incrementStat('live', 1);
      }
    } catch {}

    return created;
  }

  // Return the active view only if it belongs to the requested owner id.
  // The API is defensive: callers may provide either a tabId or a documentId.
  getView(ownerId: string): AnyView | null {
    if (this.currentOwnerId === ownerId) return this.currentView;
    return null;
  }

  // Number of currently live views owned by the host (0..maxLiveViews).
  getLiveCount(): number {
    return this.currentView ? 1 : 0;
  }

  attach(ownerId: string, view: AnyView) {
    // Backwards-compatible attach: ensure previous view is destroyed if different.
    if (this.currentView && this.currentOwnerId !== ownerId) {
      try {
        if (typeof this.currentView.destroy === 'function') {
          this.currentView.destroy();
        }
      } catch {
        // swallow
      }
      const oldOwner = this.currentOwnerId;
      this.currentView = null;
      this.currentOwnerId = null;
      // Prune diagnostics for the old owner to avoid holding strong refs.
      this._pruneDiagnosticsForOwner(oldOwner);
      try {
        if (isDebug()) {
          incrementStat('destroyed', 1);
          incrementStat('live', -1);
        }
      } catch {}
    }

    // Attach the provided view and record owner.
    this.currentView = view;
    this.currentOwnerId = ownerId;

    // Prune any stale diagnostics that might duplicate this owner entry.
    this._pruneDiagnosticsForOwner(ownerId);

    try {
      if (isDebug()) {
        incrementStat('created', 1);
        incrementStat('live', 1);
      }
    } catch {}
  }

  detach() {
    if (this.currentView) {
      try {
        if (typeof this.currentView.destroy === 'function') {
          this.currentView.destroy();
        }
      } catch {
        // swallow
      }
      // Capture owner before clearing so we can prune diagnostics.
      const oldOwner = this.currentOwnerId;
      this.currentView = null;
      this.currentOwnerId = null;

      // Prune diagnostics for the destroyed owner to ensure the debug registry
      // does not retain a strong reference to the view.
      try {
        this._pruneDiagnosticsForOwner(oldOwner);
      } catch {}

      try {
        const s: any = (window as any).__zaroxi_cm_stats ?? {};
        s.destroyed = (s.destroyed || 0) + 1;
        s.live = Math.max(0, (s.live || 1) - 1);
        (window as any).__zaroxi_cm_stats = s;
      } catch {}
    }
  }

  /**
   * destroyIfFor(id)
   *
   * Id can be:
   *  - a documentId (the common ownerId used by CodeMirrorEditor)
   *  - a tabId (EditorContainer and SessionCachePolicy pass tabIds)
   *
   * We defensively resolve tabId -> documentId using EditorSessionStore when needed.
   */
  destroyIfFor(id: string) {
    // Direct match: owner id equals provided id
    if (this.currentOwnerId === id) {
      this.detach();
      // Ensure diagnostic records are pruned for this id.
      try { this._pruneDiagnosticsForOwner(id); } catch {}
      return;
    }

    // Try resolving id as a tabId -> documentId mapping via EditorSessionStore
    try {
      const snap = EditorSessionStore.getSnapshot(id);
      const docId = snap?.documentId;
      if (docId && this.currentOwnerId === String(docId)) {
        this.detach();
        try { this._pruneDiagnosticsForOwner(String(docId)); } catch {}
        return;
      }
    } catch {
      // ignore resolution errors
    }

    // Also defensively prune diagnostics for the provided id to avoid registry retention
    try { this._pruneDiagnosticsForOwner(id); } catch {}
  }

  getActiveOwnerId() {
    return this.currentOwnerId;
  }

  // Inspect runtime host state (non-reactive). Returns a small serializable summary.
  inspect() {
    return {
      live: this.getLiveCount(),
      currentOwnerId: this.currentOwnerId,
      // Provide best-effort local stats snapshot if available.
      stats: (typeof (window as any).__zaroxi_cm_stats !== 'undefined') ? (window as any).__zaroxi_cm_stats : null,
    };
  }
}

const editorViewHost = new EditorViewHost();

if (isDebug()) {
  // Expose a non-reactive inspection helper for debugging (debug-only).
  try {
    (window as any).__zaroxi_cm_inspect = () => {
      try {
        return editorViewHost.inspect();
      } catch {
        return { live: 0, currentOwnerId: null, stats: null };
      }
    };
  } catch {}
}

export { editorViewHost };
export default editorViewHost;
