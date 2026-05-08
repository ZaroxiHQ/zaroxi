import { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { CodeEditor } from '@/components/editor/CodeEditor';
import { WorkspaceService } from '@/features/workspace/services/workspaceService';
import { useWorkspaceStore } from '@/features/workspace/stores/useWorkspaceStore';
import { Icon } from '@/components/ui/Icon';
import { useTabsStore } from '@/features/tabs/store';
import { WelcomeView } from '@/features/welcome/WelcomeView';

export function EditorContainer() {
  const { tabs, activeTabId } = useTabsStore();
  // Determine which tab is currently active (if any)
  const activeTab = useMemo(
    () => tabs.find((t) => t.id === activeTabId) ?? null,
    [tabs, activeTabId],
  );

  const [content, setContent] = useState<string>('');
  const [language, setLanguage] = useState<string | undefined>(undefined);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [fileName, setFileName] = useState<string>('editor');
  const [fileInfo, setFileInfo] = useState<{
    lineCount?: number;
    charCount?: number;
    largeFileMode?: string;
    contentTruncated?: boolean;
  }>({});
  const [initialHighlight, setInitialHighlight] = useState<any>(null);

  // Strong identity bindings for the visible editor:
  // - currentDocumentId: authoritative id returned by backend (document_id)
  // - currentRevision: authoritative revision/version for the text
  const [currentDocumentId, setCurrentDocumentId] = useState<string | null>(null);
  const [currentRevision, setCurrentRevision] = useState<number | null>(null);

  // Determine active file path:
  // Prefer the active tab's id when the active tab represents a file.
  // Fallback to the workspace explorer active file for other cases.
  const { explorerUI } = useWorkspaceStore();
  const activeFilePath = activeTab?.kind === 'file' ? activeTab.id : explorerUI.activeFilePath;

  useEffect(() => {
    // Only try to load a real file when we have a path to load.
    if (activeFilePath && activeTab?.kind === 'file') {
      // Pre-clear transient UI immediately and bump load token so any in-flight
      // responses targeted at the previous active file are canceled.
      loadSeqRef.current++;
      setInitialHighlight(null);
      setContent(''); // avoid showing previous-file content
      setLanguage(undefined);
      setFileInfo({});
      setCurrentDocumentId(null);
      setCurrentRevision(null);

      // Now kick off the actual load (async). The request is guarded by the
      // per-load sequence token created inside loadFile().
      loadFile(activeFilePath);
    }
  }, [activeFilePath, activeTab]);

  // When switching active file we clear any seeded highlight and update file metadata.
  // We intentionally do NOT increment the load sequence token here because `loadFile()`
  // itself creates a fresh sequence token at start. Incrementing the sequence from this
  // effect races with `loadFile()` when both effects run on the same update which can
  // cause the in-flight loader to be considered stale and drop its result (leaving the
  // editor blank). Also avoid clearing the visible content immediately to prevent a
  // blank flash while the authoritative content is being loaded.
  useEffect(() => {
    // clear seeded highlight and metadata for the new active file
    setInitialHighlight(null);
    setLanguage(undefined);
    setFileInfo({});
    // NOTE: do not call setContent('') here and do not bump loadSeqRef.
    // `loadFile()` will increment loadSeqRef at the very start of its run,
    // which is the single authoritative cancellation token for in-flight ops.
  }, [activeFilePath]);

  // Keep refs to the latest content and active path so keyboard handler
  // and save callback always use the authoritative values without causing
  // frequent re-registration on every keystroke.
  const contentRef = useRef(content);
  useEffect(() => {
    contentRef.current = content;
  }, [content]);

  const activeFilePathRef = useRef(activeFilePath);
  useEffect(() => {
    activeFilePathRef.current = activeFilePath;
  }, [activeFilePath]);

  // Debounce handle for coalescing frequent keystrokes into a single state update.
  // This prevents setContent/markDirty from running on every character and
  // avoids re-render storms across the app. The timer is cleared on unmount.
  const debounceRef = useRef<number | null>(null);
  useEffect(() => {
    return () => {
      if (debounceRef.current) {
        window.clearTimeout(debounceRef.current);
        debounceRef.current = null;
      }
    };
  }, []);

  // Sequence token to guard against stale async responses when loading files.
  // Incremented for each loadFile() invocation. All async results check the
  // current sequence and the activeFilePathRef before applying updates.
  const loadSeqRef = useRef(0);

  // Stable save handler that reads latest values from refs.
  const handleEditorSave = useCallback(async () => {
    const path = activeFilePathRef.current;
    if (!path) return;
    const contentToSave = contentRef.current ?? '';

    try {
      await WorkspaceService.saveFile({
        path,
        content: contentToSave,
      });
      // File saved successfully: mark as clean.
      useTabsStore.getState().markClean(path);
      WorkspaceService.markDocumentClean(path);

      const saveBtn = document.querySelector('.save-button');
      if (saveBtn) {
        const originalText = saveBtn.textContent;
        saveBtn.textContent = 'Saved!';
        saveBtn.classList.add('bg-green-500');
        setTimeout(() => {
          if (saveBtn.textContent === 'Saved!') {
            saveBtn.textContent = originalText;
            saveBtn.classList.remove('bg-green-500');
          }
        }, 1000);
      }
    } catch (error) {
      const saveBtn = document.querySelector('.save-button');
      if (saveBtn) {
        const originalText = saveBtn.textContent;
        saveBtn.textContent = 'Error!';
        saveBtn.classList.add('bg-red-500');
        setTimeout(() => {
          if (saveBtn.textContent === 'Error!') {
            saveBtn.textContent = originalText;
            saveBtn.classList.remove('bg-red-500');
          }
        }, 1000);
      }
    }
  }, []);

  // Add keyboard shortcut for save (Ctrl+S). The handler is registered once
  // and calls the stable `handleEditorSave` which reads current refs.
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
        e.preventDefault();
        handleEditorSave();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [handleEditorSave]);


  const loadFile = async (path: string) => {
    // create a local sequence token to detect stale async results
    const mySeq = ++loadSeqRef.current;
    setIsLoading(true);

    // Immediately clear the visible UI (no stale content from previous file)
    setInitialHighlight(null);
    setContent('');
    setLanguage(undefined);
    setFileName(path.split(/[\\/]/).pop() || 'file');
    setFileInfo({});

    try {
      // First try to get from frontend cache (no IPC call)
      const cached = WorkspaceService.getCachedDocument(path);
      if (cached) {
        // If the active file changed while we awaited the cache, drop this result.
        if (mySeq !== loadSeqRef.current || activeFilePathRef.current !== path) return;

        // Apply authoritative identity from cache
        setCurrentDocumentId(cached.documentId ?? path);
        setCurrentRevision((cached as any).version ?? null);

        setContent(cached.content);
        setLanguage((cached as any).language ?? undefined);
        setFileInfo({
          lineCount: cached.lineCount,
          charCount: cached.charCount,
          largeFileMode: (cached as any).largeFileMode,
          contentTruncated: cached.contentTruncated,
        });

        // Apply any cached initial highlight synchronously if present.
        if ((cached as any).initialHighlight) {
          if (mySeq === loadSeqRef.current && activeFilePathRef.current === path) {
            setInitialHighlight((cached as any).initialHighlight);
          }
        } else {
          // Otherwise fetch highlights in background but guard application.
          try {
            const h = await WorkspaceService.fetchHighlights(path);
            if (mySeq === loadSeqRef.current && activeFilePathRef.current === path && h) {
              setInitialHighlight(h);
            }
          } catch {
            // non-fatal
          }
        }

        // Final guard before finishing
        if (mySeq === loadSeqRef.current && activeFilePathRef.current === path) {
          setIsLoading(false);
        }
        return;
      }

      // Not in cache: request authoritative document from backend
      const response = await WorkspaceService.openDocument(path);

      // Drop outdated responses
      if (mySeq !== loadSeqRef.current || activeFilePathRef.current !== path) return;

      // Bind the authoritative document identity and revision before applying text.
      setCurrentDocumentId(response.documentId ?? path);
      setCurrentRevision((response as any).version ?? null);

      setContent(response.content ?? '');
      setLanguage(response.language ?? undefined);
      setFileInfo({
        lineCount: (response as any).line_count ?? (response as any).lineCount,
        charCount: (response as any).char_count ?? (response as any).charCount,
        largeFileMode: (response as any).file_class ?? (response as any).largeFileMode,
        contentTruncated: (response as any).content_truncated ?? (response as any).contentTruncated,
      });

      if ((response as any).initial_highlight || (response as any).initialHighlight) {
        const ih = (response as any).initial_highlight ?? (response as any).initialHighlight;
        if (mySeq === loadSeqRef.current && activeFilePathRef.current === path) {
          setInitialHighlight(ih);
        }
      } else {
        try {
          const h = await WorkspaceService.fetchHighlights(path);
          if (mySeq === loadSeqRef.current && activeFilePathRef.current === path && h) {
            setInitialHighlight(h);
          }
        } catch {
          // ignore
        }
      }
    } catch (error) {
      if (mySeq !== loadSeqRef.current || activeFilePathRef.current !== path) return;
      setContent(`// Error loading file: ${error instanceof Error ? error.message : 'Unknown error'}`);
      setLanguage(undefined);
      setFileName('error.txt');
      setFileInfo({});
      setCurrentDocumentId(null);
      setCurrentRevision(null);
    } finally {
      // Only clear loading if this is still the active request.
      if (mySeq === loadSeqRef.current && activeFilePathRef.current === path) {
        setIsLoading(false);
      }
    }
  };

  // Keep refs to the latest identity values so callbacks can check them without re-registration
  const currentDocumentIdRef = useRef<string | null>(currentDocumentId);
  useEffect(() => { currentDocumentIdRef.current = currentDocumentId; }, [currentDocumentId]);
  const currentRevisionRef = useRef<number | null>(currentRevision);
  useEffect(() => { currentRevisionRef.current = currentRevision; }, [currentRevision]);

  const handleEditorChange = (value: string) => {
    // HOT PATH: update local ref immediately so save/shortcuts always read the latest text.
    // This is intentionally cheap and does not cause React re-renders.
    contentRef.current = value;

    // Debounce committing the text into React state and global stores.
    // We avoid calling setContent/useTabsStore.markDirty on each keystroke because
    // that caused wide rerenders and input jank. Instead we flush once per burst.
    if (debounceRef.current) {
      window.clearTimeout(debounceRef.current);
    }

    // 200ms coalescing window: tuned to keep UI responsive while still updating
    // the container/store frequently enough for autosave/preview features.
    debounceRef.current = window.setTimeout(() => {
      // Flush authoritative content into React state (infrequent).
      setContent(contentRef.current);

      // Persist to frontend document cache and mark dirty once per burst.
      if (activeFilePathRef.current) {
        WorkspaceService.updateCachedContent(activeFilePathRef.current, contentRef.current);
        useTabsStore.getState().markDirty(activeFilePathRef.current);
      }

      debounceRef.current = null;
    }, 200);
  };

  // NOTE: `handleEditorSave` has been replaced above with a ref-backed,
  // stable callback so the keyboard handler uses the latest content/path
  // without re-registering on every keystroke. The old implementation is
  // intentionally removed to avoid stale-closure bugs that caused the wrong
  // content to be written on save (e.g. saving the previously active tab).

  // Render the Welcome tab as a completely different view (after hooks so rule of hooks is satisfied)
  if (activeTab?.kind === 'welcome') {
    return (
      <div className="h-full flex flex-col bg-editor min-h-0 w-full min-w-0">
        <WelcomeView />
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-editor min-h-0 w-full min-w-0">
      <div className="flex-1 overflow-hidden code-editor-font min-h-0 bg-editor w-full min-w-0">
        <CodeEditor
          filePath={activeFilePath || undefined}
          initialValue={content}
          onChange={handleEditorChange}
          language={language}
          readOnly={false}
          initialHighlight={initialHighlight}
        />
      </div>
    </div>
  );
}
