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

  // Create a view using a factory that receives the parent DOM element.
  // The host becomes the authoritative owner of the returned view.
  createView(ownerId: string, parent: Element, factory: (parent: Element) => AnyView): AnyView {
    // If a different view is attached, destroy it first to enforce single owner
    if (this.currentView && this.currentOwnerId !== ownerId) {
      try {
        if (typeof this.currentView.destroy === 'function') {
          this.currentView.destroy();
        }
      } catch {
        // swallow
      }
      this.currentView = null;
      this.currentOwnerId = null;
      // instrumentation: update non-reactive stats
      try {
        const w: any = window as any;
        if (w.__zaroxi_cm_debug) {
          w.__zaroxi_cm_stats = w.__zaroxi_cm_stats || {};
          w.__zaroxi_cm_stats.destroyed = (w.__zaroxi_cm_stats.destroyed || 0) + 1;
          w.__zaroxi_cm_stats.live = Math.max(0, (w.__zaroxi_cm_stats.live || 1) - 1);
        }
      } catch {}
    }

    // Create the view. Caller is responsible for providing a stable factory.
    const created = factory(parent);

    this.currentView = created;
    this.currentOwnerId = ownerId;

    // instrumentation: update non-reactive stats
    try {
      const w: any = window as any;
      if (w.__zaroxi_cm_debug) {
        w.__zaroxi_cm_stats = w.__zaroxi_cm_stats || {};
        w.__zaroxi_cm_stats.created = (w.__zaroxi_cm_stats.created || 0) + 1;
        w.__zaroxi_cm_stats.live = (w.__zaroxi_cm_stats.live || 0) + 1;
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
      this.currentView = null;
      this.currentOwnerId = null;
      try {
        const s: any = (window as any).__zaroxi_cm_stats ?? {};
        s.destroyed = (s.destroyed || 0) + 1;
        s.live = Math.max(0, (s.live || 1) - 1);
        (window as any).__zaroxi_cm_stats = s;
      } catch {}
    }
    this.currentView = view;
    this.currentOwnerId = ownerId;
    try {
      const w: any = window as any;
      if (w.__zaroxi_cm_debug) {
        w.__zaroxi_cm_stats = w.__zaroxi_cm_stats || {};
        w.__zaroxi_cm_stats.created = (w.__zaroxi_cm_stats.created || 0) + 1;
        w.__zaroxi_cm_stats.live = (w.__zaroxi_cm_stats.live || 0) + 1;
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
      this.currentView = null;
      this.currentOwnerId = null;
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
      return;
    }

    // Try resolving id as a tabId -> documentId mapping via EditorSessionStore
    try {
      const snap = EditorSessionStore.getSnapshot(id);
      const docId = snap?.documentId;
      if (docId && this.currentOwnerId === String(docId)) {
        this.detach();
        return;
      }
    } catch {
      // ignore resolution errors
    }
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

// Expose a non-reactive inspection helper for debugging (safe no-op if window missing)
try {
  (window as any).__zaroxi_cm_inspect = () => {
    try {
      return editorViewHost.inspect();
    } catch {
      return { live: 0, currentOwnerId: null, stats: null };
    }
  };
} catch {}

export { editorViewHost };
export default editorViewHost;
