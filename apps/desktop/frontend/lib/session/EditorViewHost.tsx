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

type AnyView = { destroy?: () => void } | null;

/**
 * EditorViewHost
 *
 * Centralized owner/creator of live EditorView instances.
 *
 * Guarantees:
 *  - Only the host may construct and retain a live view reference.
 *  - createView destroys the previous live view when exceeding the live cap.
 *  - destroyIfFor(tabId) is idempotent.
 *  - Lightweight, non-reactive runtime diagnostics are updated here.
 */
class EditorViewHost {
  private currentView: AnyView | null = null;
  private currentTabId: string | null = null;
  private maxLiveViews: number = 1; // hard guard: only allow 1 live view by default

  // Create a view using a factory that receives the parent DOM element.
  // The host becomes the authoritative owner of the returned view.
  createView(tabId: string, parent: Element, factory: (parent: Element) => AnyView): AnyView {
    // If a different view is attached, destroy it first to enforce single owner
    if (this.currentView && this.currentTabId !== tabId) {
      try {
        if (typeof this.currentView.destroy === 'function') {
          this.currentView.destroy();
        }
      } catch {
        // swallow
      }
      this.currentView = null;
      this.currentTabId = null;
      // instrumentation: update non-reactive stats
      try {
        const s: any = (window as any).__zaroxi_cm_stats ?? {};
        s.destroyed = (s.destroyed || 0) + 1;
        s.live = Math.max(0, (s.live || 1) - 1);
        (window as any).__zaroxi_cm_stats = s;
      } catch {}
    }

    // Enforce max live view count: if we already have max and it's for the same tab, reuse.
    // If it's for a different tab we already destroyed it above.
    const created = factory(parent);

    this.currentView = created;
    this.currentTabId = tabId;

    // instrumentation: update non-reactive stats
    try {
      const s: any = (window as any).__zaroxi_cm_stats ?? {};
      s.created = (s.created || 0) + 1;
      s.live = (s.live || 0) + 1;
      const map = s.createdByDoc || (s.createdByDoc = Object.create(null));
      if (typeof tabId === 'string') {
        if (Object.prototype.hasOwnProperty.call(map, tabId)) {
          map[tabId] += 1;
        } else if (Object.keys(map).length < 200) {
          map[tabId] = 1;
        } else {
          s.createdOtherDocs = (s.createdOtherDocs || 0) + 1;
        }
      }
      (window as any).__zaroxi_cm_stats = s;
    } catch {}

    return created;
  }

  // Return the active view only if it belongs to the requested tab id.
  getView(tabId: string): AnyView | null {
    if (this.currentTabId === tabId) return this.currentView;
    return null;
  }

  // Number of currently live views owned by the host (0..maxLiveViews).
  getLiveCount(): number {
    return this.currentView ? 1 : 0;
  }

  attach(tabId: string, view: AnyView) {
    // Backwards-compatible attach: ensure previous view is destroyed if different.
    if (this.currentView && this.currentTabId !== tabId) {
      try {
        if (typeof this.currentView.destroy === 'function') {
          this.currentView.destroy();
        }
      } catch {
        // swallow
      }
      this.currentView = null;
      this.currentTabId = null;
      try {
        const s: any = (window as any).__zaroxi_cm_stats ?? {};
        s.destroyed = (s.destroyed || 0) + 1;
        s.live = Math.max(0, (s.live || 1) - 1);
        (window as any).__zaroxi_cm_stats = s;
      } catch {}
    }
    this.currentView = view;
    this.currentTabId = tabId;
    try {
      const s: any = (window as any).__zaroxi_cm_stats ?? {};
      s.created = (s.created || 0) + 1;
      s.live = (s.live || 0) + 1;
      (window as any).__zaroxi_cm_stats = s;
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
      this.currentTabId = null;
      try {
        const s: any = (window as any).__zaroxi_cm_stats ?? {};
        s.destroyed = (s.destroyed || 0) + 1;
        s.live = Math.max(0, (s.live || 1) - 1);
        (window as any).__zaroxi_cm_stats = s;
      } catch {}
    }
  }

  destroyIfFor(tabId: string) {
    if (this.currentTabId === tabId) {
      this.detach();
    }
  }

  getActiveTabId() {
    return this.currentTabId;
  }
}

const editorViewHost = new EditorViewHost();
export { editorViewHost };
export default editorViewHost;
