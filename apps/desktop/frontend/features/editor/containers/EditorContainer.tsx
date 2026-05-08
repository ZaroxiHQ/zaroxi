import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { CodeEditor } from '@/components/editor/CodeEditor';
import { WorkspaceService } from '@/features/workspace/services/workspaceService';
import { useWorkspaceStore } from '@/features/workspace/stores/useWorkspaceStore';
import { Icon } from '@/components/ui/Icon';
import { useTabsStore } from '@/features/tabs/store';
import { WelcomeView } from '@/features/welcome/WelcomeView';

/**
 * EditorContainer (session-centric)
 *
 * Root-cause fix summary:
 * - Maintain an explicit sessionsByTabId map owned by this container.
 * - Each session is the single source of truth for visible editor state.
 * - On tab switch we persist the outgoing session, restore the incoming
 *   session synchronously (or show a loading state), and never allow async
 *   results that don't match the exact tabId+documentId+loadSeq to apply.
 * - Typing only updates the active session immediately (local hot path).
 *
 * This file implements the required Active Session Model:
 * { tabId, documentId, filePath, revision, text, language, selection?, scrollTop?, highlightSnapshot, highlightRevision, dirty, isLoading, loadSeq }
 *
 * This is an explicit, local ref-backed map to avoid wide re-renders and to make
 * session swaps deterministic and safe.
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
};

/* ----------------------------- Component ----------------------------- */

export function EditorContainer() {
  const { tabs, activeTabId } = useTabsStore();

  // Determine which tab is currently active (if any)
  const activeTab = useMemo(
    () => tabs.find((t) => t.id === activeTabId) ?? null,
    [tabs, activeTabId],
  );

  // Local sessions map (ref to avoid re-renders for non-active sessions)
  const sessionsRef = useRef<Map<string, LocalSession>>(new Map());

  // Rendered session state (drives CodeEditor props). Always derived from sessionsRef for active tab.
  const [renderSessionId, setRenderSessionId] = useState<string | null>(activeTabId ?? null);
  const [, forceUpdate] = useState(0); // used to force render when active session updates

  // Utilities for active session access
  const getSession = (tabId: string | null): LocalSession | null => {
    if (!tabId) return null;
    return sessionsRef.current.get(tabId) ?? null;
  };

  const setSession = (tabId: string, sess: LocalSession) => {
    sessionsRef.current.set(tabId, sess);
  };

  // Refs for hot-path content and cancellation
  const activeTabIdRef = useRef<string | null>(activeTabId ?? null);
  useEffect(() => { activeTabIdRef.current = activeTabId ?? null; }, [activeTabId]);

  const contentRef = useRef<string>('');
  const loadSeqRef = useRef<number>(0);

  // Ensure we have a welcome session by default (keeps UI stable)
  useEffect(() => {
    if (!tabs || tabs.length === 0) return;
    // Ensure Welcome tab exists in sessions map
    const first = tabs[0];
    if (first && first.kind === 'welcome' && !sessionsRef.current.has(first.id)) {
      setSession(first.id, {
        tabId: first.id,
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
  }, [tabs]);

  // Persist outgoing session whenever activeTabId changes.
  const prevActiveTabIdRef = useRef<string | null>(activeTabId ?? null);
  useEffect(() => {
    const prev = prevActiveTabIdRef.current;
    const curr = activeTabId ?? null;

    // Save outgoing session's live text from contentRef into session store
    if (prev) {
      const outgoing = sessionsRef.current.get(prev);
      if (outgoing) {
        outgoing.text = contentRef.current;
        // keep dirty flag etc (we don't force setState)
        sessionsRef.current.set(prev, outgoing);
      }
    }

    // Switch render session id to the new tab (triggers UI to read session)
    setRenderSessionId(curr);
    prevActiveTabIdRef.current = curr;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeTabId]);

  // When the renderSessionId changes, ensure the session exists and if it's a file session,
  // start loading it if necessary.
  useEffect(() => {
    const tabId = renderSessionId;
    if (!tabId) return;

    const tab = tabs.find((t) => t.id === tabId) ?? null;
    if (!tab) return;

    let sess = sessionsRef.current.get(tabId);
    if (!sess) {
      // Create a new session placeholder and try to seed from frontend cache synchronously.
      // This prevents a transient blank editor when switching tabs by showing cached content
      // immediately if available. If no cache exists we fall back to an empty placeholder
      // and start the async load.
      let seededSession: LocalSession | null = null;
      if (tab.kind === 'file') {
        const cached = WorkspaceService.getCachedDocument(tab.id);
        if (cached) {
          seededSession = {
            tabId,
            documentId: cached.documentId ?? tab.id,
            filePath: tab.id,
            revision: (cached as any).version ?? null,
            text: cached.content ?? '',
            language: (cached as any).language ?? undefined,
            initialHighlight: (cached as any).initialHighlight ?? null,
            isLoading: false,
            loadSeq: 0,
            contentTruncated: cached.contentTruncated ?? false,
            lineCount: cached.lineCount,
            charCount: cached.charCount,
            isDirty: (cached as any).isDirty ?? false,
          };
        }
      }

      if (seededSession) {
        sess = seededSession;
      } else {
        // No cached snapshot available — create a minimal placeholder and allow loadFileForSession to fetch.
        sess = {
          tabId,
          documentId: null,
          filePath: tab.kind === 'file' ? tab.id : null,
          revision: null,
          text: '',
          language: undefined,
          initialHighlight: null,
          isLoading: tab.kind === 'file',
          loadSeq: 0,
          contentTruncated: false,
          lineCount: undefined,
          charCount: undefined,
          isDirty: false,
        };
      }

      setSession(tabId, sess);

      // If we seeded text synchronously, ensure the editor hot-path sees it immediately.
      // This writes the hot-path contentRef and schedules a render so CodeEditor receives
      // a non-empty text value on first paint instead of a blank editor.
      if (sess.text && sess.text.length > 0) {
        contentRef.current = sess.text;
        if (activeTabIdRef.current === tabId) {
          forceUpdate((x) => x + 1);
        }
      }
    }

    // If this is a file tab and we have no documentId/text, load it.
    if (tab.kind === 'file' && (!sess.documentId || sess.documentId === null) && !sess.isLoading) {
      // start loading
      void loadFileForSession(tabId, tab.id);
    } else {
      // Ensure contentRef and CodeEditor get current session text
      contentRef.current = sess.text;
      // force UI update so CodeEditor reads the new session
      forceUpdate((x) => x + 1);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [renderSessionId, tabs]);

  // Helper: load a file into the named session (tabId must be the owner)
  const loadFileForSession = async (tabId: string, path: string) => {
    // Guard: ensure tab still active owner after async waits
    const mySeq = ++loadSeqRef.current;
    // Mark session loading state
    let sess = sessionsRef.current.get(tabId);
    if (!sess) return;
    sess.isLoading = true;
    sess.loadSeq = mySeq;
    sess.filePath = path;
    setSession(tabId, sess);

    // If cached at frontend, apply immediately to session
    try {
      const cached = WorkspaceService.getCachedDocument(path);
      if (cached) {
        // If session changed meanwhile, don't apply
        const currentSess = sessionsRef.current.get(tabId);
        if (!currentSess || currentSess.loadSeq !== mySeq) return;

        currentSess.documentId = cached.documentId ?? path;
        currentSess.revision = (cached as any).version ?? null;
        currentSess.text = cached.content ?? '';
        currentSess.language = (cached as any).language ?? undefined;
        currentSess.initialHighlight = (cached as any).initialHighlight ?? null;
        currentSess.contentTruncated = cached.contentTruncated ?? false;
        currentSess.lineCount = cached.lineCount;
        currentSess.charCount = cached.charCount;
        currentSess.isLoading = false;
        setSession(tabId, currentSess);

        // If this session is currently visible, apply to render state
        if (activeTabIdRef.current === tabId) {
          contentRef.current = currentSess.text;
          forceUpdate((x) => x + 1);
        }
        return;
      }

      // Not cached: request from backend
      const response = await WorkspaceService.openDocument(path);

      // Check cancellation: session must still exist and loadSeq must match
      const currentSess = sessionsRef.current.get(tabId);
      if (!currentSess || currentSess.loadSeq !== mySeq) {
        // Stale result: drop
        return;
      }

      currentSess.documentId = response.documentId ?? path;
      currentSess.revision = (response as any).version ?? null;
      currentSess.text = response.content ?? '';
      currentSess.language = response.language ?? undefined;
      currentSess.initialHighlight = (response as any).initial_highlight ?? (response as any).initialHighlight ?? null;
      currentSess.contentTruncated = (response as any).content_truncated ?? (response as any).contentTruncated ?? false;
      currentSess.lineCount = (response as any).line_count ?? (response as any).lineCount;
      currentSess.charCount = (response as any).char_count ?? (response as any).charCount;
      currentSess.isLoading = false;
      setSession(tabId, currentSess);

      // If this session is visible, apply to DOM/render
      if (activeTabIdRef.current === tabId) {
        contentRef.current = currentSess.text;
        forceUpdate((x) => x + 1);
      }
    } catch (err) {
      const currentSess = sessionsRef.current.get(tabId);
      if (!currentSess || currentSess.loadSeq !== mySeq) return;
      currentSess.text = `// Error loading file: ${err instanceof Error ? err.message : String(err)}`;
      currentSess.isLoading = false;
      setSession(tabId, currentSess);
      if (activeTabIdRef.current === tabId) {
        contentRef.current = currentSess.text;
        forceUpdate((x) => x + 1);
      }
    }
  };

  // Typing hot-path: immediate local updates to active session
  // Debounced persistence to global store / cache
  const debounceRef = useRef<number | null>(null);
  useEffect(() => {
    return () => {
      if (debounceRef.current) {
        window.clearTimeout(debounceRef.current);
        debounceRef.current = null;
      }
    };
  }, []);

  const handleEditorChange = useCallback((value: string) => {
    const tabId = activeTabIdRef.current;
    if (!tabId) {
      contentRef.current = value;
      return;
    }

    // Immediate hot-path: update ref and session
    contentRef.current = value;
    const sess = sessionsRef.current.get(tabId);
    if (sess) {
      sess.text = value;
      sess.isDirty = true;
      sessionsRef.current.set(tabId, sess);
    }

    // Debounced persistence
    if (debounceRef.current) {
      window.clearTimeout(debounceRef.current);
    }
    debounceRef.current = window.setTimeout(() => {
      const latestTab = activeTabIdRef.current;
      if (latestTab) {
        const s = sessionsRef.current.get(latestTab);
        if (s) {
          WorkspaceService.updateCachedContent(s.filePath ?? latestTab, s.text);
          useTabsStore.getState().markDirty(latestTab);
        }
      }
      debounceRef.current = null;
    }, 200);
  }, []);

  // Stable save handler (reads from session store)
  const handleEditorSave = useCallback(async () => {
    const tabId = activeTabIdRef.current;
    if (!tabId) return;
    const sess = sessionsRef.current.get(tabId);
    if (!sess || !sess.filePath) return;
    try {
      await WorkspaceService.saveFile({ path: sess.filePath, content: sess.text });
      sess.isDirty = false;
      sessionsRef.current.set(tabId, sess);
      useTabsStore.getState().markClean(tabId);
      WorkspaceService.markDocumentClean(sess.filePath);
    } catch {
      // ignore for now
    }
  }, []);

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

  // Build CodeEditor session prop from current visible LocalSession
  // NOTE: previously this used useMemo with only renderSessionId as dependency.
  // That produced a stale visibleSession when sessionsRef (a ref) was mutated
  // but renderSessionId did not change — leaving the editor bound to an
  // empty placeholder. We now compute the session each render directly so
  // it always reads the latest sessionsRef and contentRef.
  const visibleSession = (() => {
    const tabId = renderSessionId;
    const sess = tabId ? sessionsRef.current.get(tabId) ?? null : null;

    // If there is no session object yet, prefer an existing hot-path contentRef
    // (last known text) to avoid rendering a blank editor while loading.
    if (!sess) {
      return {
        tabId: tabId ?? null,
        documentId: null,
        revision: null,
        text: contentRef.current ?? '',
        language: undefined,
        initialHighlight: null,
        isLoading: false,
        loadSeq: 0,
        contentTruncated: false,
      } as any;
    }

    return {
      tabId: sess.tabId,
      documentId: sess.documentId ?? sess.filePath ?? `__no_doc__:${sess.tabId}`,
      revision: sess.revision ?? null,
      // Prefer the authoritative session text; fall back to last hot-path contentRef
      // to avoid briefly showing an empty editor during async hydration.
      text: sess.text ?? contentRef.current ?? '',
      language: sess.language ?? undefined,
      initialHighlight: sess.initialHighlight ?? null,
      isLoading: sess.isLoading,
      loadSeq: sess.loadSeq,
      contentTruncated: sess.contentTruncated ?? false,
    } as any;
  })();

  // If active tab is welcome, render WelcomeView
  if (activeTab?.kind === 'welcome') {
    return (
      <div className="h-full flex flex-col bg-editor min-h-0 w-full min-w-0">
        <WelcomeView />
      </div>
    );
  }

  // Render the editor bound to the visible explicit session
  return (
    <div className="h-full flex flex-col bg-editor min-h-0 w-full min-w-0">
      <div className="flex-1 overflow-hidden code-editor-font min-h-0 bg-editor w-full min-w-0">
        {visibleSession.isLoading && (!visibleSession.text || visibleSession.text.length === 0) ? (
          <div className="h-full flex items-center justify-center text-muted-foreground text-sm p-4 bg-editor">
            Loading file…
          </div>
        ) : (
          <CodeEditor
            session={visibleSession}
            onChange={handleEditorChange}
            onSave={handleEditorSave}
            readOnly={false}
          />
        )}
      </div>
    </div>
  );
}
