import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { CodeEditor } from '@/components/editor/CodeEditor';
import { CodeMirrorEditor } from '@/components/editor/CodeMirrorEditor';
import { getEditorEngine, stateCache } from '@/components/editor/editorEngine';
import { WorkspaceService } from '@/features/workspace/services/workspaceService';
import { useWorkspaceStore } from '@/features/workspace/stores/useWorkspaceStore';
import { Icon } from '@/components/ui/Icon';
import { useTabsStore } from '@/features/tabs/store';
import { WelcomeView } from '@/features/welcome/WelcomeView';
import sessionCache from '@/lib/session/SessionCachePolicy';
import EditorSessionStore from '@/stores/EditorSessionStore';
import documentStore from '@/stores/DocumentStore';
import editorViewHost from '@/lib/session/EditorViewHost';

/**
 * EditorContainer - Simplified, deterministic session owner.
 *
 * Key principles implemented:
 * - sessions state (Map) is owned in React state for deterministic renders.
 * - each session has loadSeq to gate async results.
 * - active session is derived directly from sessions state keyed by activeTabId.
 * - while a file is loading and has no text we show an explicit loading UI.
 * - typing updates session.text immediately and container persists debounced.
 */

/* ----------------------------- Types --------------------------------- */

type LocalSession = {
  tabId: string;
  documentId: string | null;
  filePath: string | null;
  revision: number | null;
  text: string;
  language?: string | undefined;
  initialHighlight?: any | null;
  isLoading: boolean;
  loadSeq: number;
  contentTruncated?: boolean;
  lineCount?: number;
  charCount?: number;
  isDirty?: boolean;
  // lastAccess is used for LRU eviction (ms since epoch). Updated on access/upsert.
  lastAccess?: number;
};

/* ----------------------------- Component ----------------------------- */

export function EditorContainer() {
  const { tabs, activeTabId } = useTabsStore();

  // Sessions are stored in React state (Map) so updates cause renders.
  const [sessions, setSessions] = useState<Map<string, LocalSession>>(() => {
    const m = new Map<string, LocalSession>();
    // seed welcome tab if present in initial tabs
    const welcome = tabs.find((t) => t.id && t.kind === 'welcome');
    if (welcome) {
      m.set(welcome.id, {
        tabId: welcome.id,
        documentId: null,
        filePath: null,
        revision: null,
        text: '',
        language: undefined,
        initialHighlight: null,
        isLoading: false,
        loadSeq: 0,
        contentTruncated: false,
        isDirty: false,
        lastAccess: Date.now(),
      });
    }
    return m;
  });

  // Simple load sequence counter (global) - incremented per load request and stored on session
  const globalLoadSeq = useRef(0);

  // Helper to get/set a session immutably
  const getSession = useCallback(
    (tabId: string | null): LocalSession | null => {
      if (!tabId) return null;
      return sessions.get(tabId) ?? null;
    },
    [sessions],
  );

  // LRU session upsert with eviction of heavy payloads for inactive tabs.
  const MAX_CACHED_SESSIONS = 60;

  const upsertSession = useCallback((tabId: string, patch: Partial<LocalSession>) => {
    setSessions((prev) => {
      const next = new Map(prev);
      const existing = next.get(tabId) ?? {
        tabId,
        documentId: null,
        filePath: null,
        revision: null,
        text: '',
        language: undefined,
        initialHighlight: null,
        isLoading: false,
        loadSeq: 0,
        contentTruncated: false,
        lineCount: undefined,
        charCount: undefined,
        isDirty: false,
        lastAccess: Date.now(),
      } as LocalSession;

      const merged: LocalSession = { ...existing, ...patch, lastAccess: patch.lastAccess ?? Date.now() };
      next.set(tabId, merged);

      // Evict/shrink least-recently-used non-dirty sessions when exceeding cache size.
      try {
        if (next.size > MAX_CACHED_SESSIONS) {
          const entries = Array.from(next.entries());
          entries.sort((a, b) => (a[1].lastAccess || 0) - (b[1].lastAccess || 0));
          let toEvict = next.size - MAX_CACHED_SESSIONS;
          for (const [k, s] of entries) {
            if (toEvict <= 0) break;
            if (k === activeTabId) continue; // don't evict active tab
            if (s.isDirty) continue; // preserve dirty sessions

            // Persist heavier fields to a document-level store before shrinking.
            // This ensures the in-memory session becomes lightweight while the
            // authoritative document content is retained for later restore.
            try {
              if (s.documentId && typeof s.text === 'string' && s.text.length > 0) {
                documentStore.set(s.documentId, { documentId: s.documentId, content: s.text, version: s.revision ?? undefined });
              }
            } catch {}

            // Also persist a minimal editor session snapshot (selection/scroll) so
            // we can restore view position without keeping EditorView instances.
            try {
              if (s.tabId) {
                EditorSessionStore.setSnapshot(s.tabId, {
                  tabId: s.tabId,
                  documentId: s.documentId ?? null,
                  text: undefined,
                  selection: undefined,
                  scrollTop: undefined,
                  language: s.language ?? null,
                  isDirty: s.isDirty ?? false,
                  version: s.revision ?? null,
                  lastActiveAt: s.lastAccess ?? Date.now(),
                  tier: 'warm',
                } as any);
              }
            } catch {}

            // Shrink heavy payload while preserving minimal metadata
            const shrunk: LocalSession = {
              ...s,
              text: '',
              initialHighlight: null,
              lineCount: undefined,
              charCount: undefined,
              lastAccess: s.lastAccess,
            };
            next.set(k, shrunk);

            // Also drop any engine-level cached EditorState to free memory
            try {
              if ((stateCache as Map<string, any>).has(k)) {
                (stateCache as Map<string, any>).delete(k);
              }
            } catch {}

            // Ensure any live EditorView for this tab is destroyed and released.
            try {
              editorViewHost.destroyIfFor(k);
            } catch {}

            toEvict--;
          }
        }
      } catch (e) {
        // Non-fatal: eviction best-effort
        // eslint-disable-next-line no-console
        console.debug('[editor-container] eviction error', e);
      }

      return next;
    });
  }, [activeTabId]);

  // Ensure welcome tab exists when tabs change and sessions missing
  useEffect(() => {
    const welcome = tabs.find((t) => t.kind === 'welcome');
    if (welcome && !sessions.has(welcome.id)) {
      upsertSession(welcome.id, {
        tabId: welcome.id,
        documentId: null,
        filePath: null,
        revision: null,
        text: '',
        language: undefined,
        initialHighlight: null,
        isLoading: false,
        loadSeq: 0,
        contentTruncated: false,
        isDirty: false,
      });
    }
  }, [tabs, sessions, upsertSession]);

  // When activeTabId changes, ensure a session exists and kick off load if needed.
  useEffect(() => {
    const tabId = activeTabId;
    if (!tabId) return;
    const tab = tabs.find((t) => t.id === tabId);
    if (!tab) return;

    // Notify session cache that the tab became active and update session last-active timestamp.
    try { sessionCache.onActivate(tabId); } catch {}
    try { EditorSessionStore.touch(tabId); } catch {}

    const sess = sessions.get(tabId);
    if (!sess) {
      // Create placeholder session and possibly seed sync-cached content
      let seeded: Partial<LocalSession> | null = null;
      if (tab.kind === 'file') {
        const cached = WorkspaceService.getCachedDocument(tab.id);
        if (cached) {
          seeded = {
            documentId: cached.documentId ?? tab.id,
            filePath: tab.id,
            revision: (cached as any).version ?? null,
            text: cached.content ?? '',
            language: (cached as any).language ?? undefined,
            initialHighlight: (cached as any).initialHighlight ?? null,
            isLoading: false,
            contentTruncated: cached.contentTruncated ?? false,
            lineCount: cached.lineCount,
            charCount: cached.charCount,
            isDirty: (cached as any).isDirty ?? false,
          };
        }
      }

      upsertSession(tabId, {
        tabId,
        documentId: seeded?.documentId ?? null,
        filePath: seeded?.filePath ?? (tab.kind === 'file' ? tab.id : null),
        revision: seeded?.revision ?? null,
        text: seeded?.text ?? '',
        language: seeded?.language,
        initialHighlight: seeded?.initialHighlight ?? null,
        isLoading: seeded ? false : tab.kind === 'file',
        loadSeq: seeded ? 0 : 0,
        contentTruncated: seeded?.contentTruncated ?? false,
        lineCount: seeded?.lineCount,
        charCount: seeded?.charCount,
        isDirty: seeded?.isDirty ?? false,
      });

      // If not seeded and file tab, start async load
      if (!seeded && tab.kind === 'file') {
        void loadFileForSession(tabId, tab.id);
      }
      return;
    }

    // If session exists but is a file with no documentId and not loading, start load
    if (tab.kind === 'file' && (!sess.documentId || sess.documentId === null) && !sess.isLoading) {
      void loadFileForSession(tabId, tab.id);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeTabId, tabs, sessions, upsertSession]);

  // Load helper with loadSeq gating (stale responses ignored)
  const loadFileForSession = useCallback(
    async (tabId: string, path: string) => {
      const mySeq = ++globalLoadSeq.current;
      upsertSession(tabId, { isLoading: true, loadSeq: mySeq, filePath: path });

      // Try frontend cache first
      const cached = WorkspaceService.getCachedDocument(path);
      if (cached) {
        // Apply cached snapshot synchronously
        setSessions((prev) => {
          const next = new Map(prev);
          const base = next.get(tabId) ?? ({} as LocalSession);
          const updated: LocalSession = {
            ...base,
            tabId,
            documentId: cached.documentId ?? path,
            filePath: path,
            revision: (cached as any).version ?? null,
            text: cached.content ?? '',
            language: cached.language ?? undefined,
            initialHighlight: cached.initialHighlight ?? null,
            isLoading: false,
            loadSeq: mySeq,
            contentTruncated: cached.contentTruncated ?? false,
            lineCount: cached.lineCount,
            charCount: cached.charCount,
            isDirty: cached.isDirty ?? false,
          };
          next.set(tabId, updated);
          return next;
        });
        return;
      }

      // Otherwise fetch from backend
      try {
        const response = await WorkspaceService.openDocument(path);

        // Ensure session still expects this loadSeq
        setSessions((prev) => {
          const existing = prev.get(tabId);
          if (!existing) return prev;
          if (existing.loadSeq !== mySeq) return prev; // stale
          const next = new Map(prev);
          const updated: LocalSession = {
            ...existing,
            tabId,
            documentId: response.documentId ?? path,
            filePath: path,
            revision: (response as any).version ?? null,
            text: response.content ?? '',
            language: response.language ?? undefined,
            initialHighlight: response.initialHighlight ?? null,
            isLoading: false,
            contentTruncated: response.contentTruncated ?? false,
            lineCount: response.lineCount,
            charCount: response.charCount,
            isDirty: false,
            loadSeq: mySeq,
          };
          next.set(tabId, updated);
          return next;
        });
      } catch (err) {
        // Apply error text only if still the expected load
        setSessions((prev) => {
          const existing = prev.get(tabId);
          if (!existing) return prev;
          if (existing.loadSeq !== mySeq) return prev;
          const next = new Map(prev);
          next.set(tabId, {
            ...existing,
            text: `// Error loading file: ${err instanceof Error ? err.message : String(err)}`,
            isLoading: false,
            loadSeq: mySeq,
          });
          return next;
        });
      }
    },
    [upsertSession],
  );

  // Typing hot-path: update session.text immediately and debounce persist
  const debounceRef = useRef<number | null>(null);

  const handleEditorChange = useCallback(
    (value: string) => {
      const tabId = activeTabId;
      if (!tabId) return;
      setSessions((prev) => {
        const next = new Map(prev);
        const sess = next.get(tabId) ?? {
          tabId,
          documentId: null,
          filePath: null,
          revision: null,
          text: '',
          language: undefined,
          initialHighlight: null,
          isLoading: false,
          loadSeq: 0,
          contentTruncated: false,
          isDirty: false,
        } as LocalSession;
        sess.text = value;
        sess.isDirty = true;
        next.set(tabId, sess);
        return next;
      });

      if (debounceRef.current) {
        window.clearTimeout(debounceRef.current);
      }
      debounceRef.current = window.setTimeout(() => {
        const s = sessions.get(activeTabId ?? '');
        if (s && s.filePath) {
          WorkspaceService.updateCachedContent(s.filePath, s.text);
          useTabsStore.getState().markDirty(activeTabId ?? '');
        }
        debounceRef.current = null;
      }, 200);
    },
    [activeTabId, sessions],
  );

  // Save handler
  const handleEditorSave = useCallback(async () => {
    const tabId = activeTabId;
    if (!tabId) return;
    const s = sessions.get(tabId);
    if (!s || !s.filePath) return;
    try {
      await WorkspaceService.saveFile({ path: s.filePath, content: s.text });
      setSessions((prev) => {
        const next = new Map(prev);
        const cur = next.get(tabId);
        if (!cur) return prev;
        cur.isDirty = false;
        next.set(tabId, cur);
        return next;
      });
      useTabsStore.getState().markClean(tabId);
      WorkspaceService.markDocumentClean(s.filePath);
    } catch {
      // ignore
    }
  }, [activeTabId, sessions]);

  // Keyboard shortcut registration (Ctrl/Cmd+S)
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
        e.preventDefault();
        void handleEditorSave();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [handleEditorSave]);

  // Ensure any pending debounce timer is cleared when the container unmounts.
  useEffect(() => {
    return () => {
      try {
        if (debounceRef.current) {
          window.clearTimeout(debounceRef.current);
          debounceRef.current = null;
        }
      } catch {}
    };
  }, []);

  // Determine visible session (authoritative)
  const activeSession = useMemo(() => {
    if (!activeTabId) return null;
    return sessions.get(activeTabId) ?? null;
  }, [activeTabId, sessions]);

  // Render
  if (activeSession && activeSession.documentId === null && activeSession.isLoading) {
    // If file is loading and we have no content to show, render explicit loading UI.
    return (
      <div className="h-full flex flex-col bg-editor min-h-0 w-full min-w-0">
        <div className="h-full flex items-center justify-center text-muted-foreground text-sm p-4 bg-editor">
          Loading file…
        </div>
      </div>
    );
  }

  if (activeTabId && !activeSession && tabs.find((t) => t.id === activeTabId)?.kind === 'welcome') {
    return (
      <div className="h-full flex flex-col bg-editor min-h-0 w-full min-w-0">
        <WelcomeView />
      </div>
    );
  }

  // If active tab is welcome
  const activeTab = tabs.find((t) => t.id === activeTabId) ?? null;
  if (activeTab?.kind === 'welcome') {
    return (
      <div className="h-full flex flex-col bg-editor min-h-0 w-full min-w-0">
        <WelcomeView />
      </div>
    );
  }

  // If there's no active session yet, show a neutral placeholder
  if (!activeSession) {
    return (
      <div className="h-full flex flex-col bg-editor min-h-0 w-full min-w-0">
        <div className="h-full flex items-center justify-center text-muted-foreground text-sm p-4 bg-editor">
          No file selected
        </div>
      </div>
    );
  }

  const engine = getEditorEngine();

  if (engine === 'codemirror') {
    return (
      <div className="h-full flex flex-col bg-editor min-h-0 w-full min-w-0">
        <div className="flex-1 overflow-hidden code-editor-font min-h-0 bg-editor w-full min-w-0">
          <CodeMirrorEditor
            documentId={activeSession.documentId ?? activeSession.tabId}
            text={activeSession.text}
            languageId={activeSession.language}
            onChange={(value: string) => handleEditorChange(value)}
            onSave={handleEditorSave}
            readOnly={false}
          />
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-editor min-h-0 w-full min-w-0">
      <div className="flex-1 overflow-hidden code-editor-font min-h-0 bg-editor w-full min-w-0">
        <CodeEditor
          session={{
            tabId: activeSession.tabId,
            documentId: activeSession.documentId,
            revision: activeSession.revision,
            text: activeSession.text,
            language: activeSession.language,
            initialHighlight: activeSession.initialHighlight,
            isLoading: activeSession.isLoading,
            loadSeq: activeSession.loadSeq,
            contentTruncated: activeSession.contentTruncated ?? false,
          } as any}
          onChange={handleEditorChange}
          onSave={handleEditorSave}
          readOnly={false}
        />
      </div>
    </div>
  );
}
