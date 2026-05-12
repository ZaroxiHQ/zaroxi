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

class EditorViewHost {
  private currentView: AnyView | null = null;
  private currentTabId: string | null = null;

  attach(tabId: string, view: AnyView) {
    // If a different view is attached, destroy it first.
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
    }
    this.currentView = view;
    this.currentTabId = tabId;
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
export default editorViewHost;
