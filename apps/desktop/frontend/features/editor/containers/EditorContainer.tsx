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
  
  // Determine active file path:
  // Prefer the active tab's id when the active tab represents a file.
  // Fallback to the workspace explorer active file for other cases.
  const { explorerUI } = useWorkspaceStore();
  const activeFilePath = activeTab?.kind === 'file' ? activeTab.id : explorerUI.activeFilePath;

  useEffect(() => {
    // Only try to load a real file when we have a path to load.
    if (activeFilePath && activeTab?.kind === 'file') {
      loadFile(activeFilePath);
    }
  }, [activeFilePath, activeTab]);

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
    setIsLoading(true);
    // Do not clear content immediately: keep existing visible content until
    // the new content is loaded. Clearing here caused the highlighter to
    // request highlights for an empty buffer which produced flicker and made
    // highlighting appear only after the second load. Preserve the previous
    // content until we have the authoritative response.
    try {
      // First try to get from frontend cache (no IPC call)
      const cached = WorkspaceService.getCachedDocument(path);
      if (cached) {
        setContent(cached.content);
        setLanguage(cached.language ?? undefined);
        setFileName(path.split(/[\\/]/).pop() || 'file');
        setFileInfo({
          lineCount: cached.lineCount,
          charCount: cached.charCount,
          largeFileMode: cached.largeFileMode,
          contentTruncated: cached.contentTruncated,
        });
        setIsLoading(false);
        return;
      }

      // Not in cache, fetch from backend (which will use the Rust cache)
      const response = await WorkspaceService.openFile({ path });
      setContent(response.content);
      setLanguage(response.language ?? undefined);
      setFileName(path.split(/[\\/]/).pop() || 'file');
      setFileInfo({
        lineCount: response.lineCount,
        charCount: response.charCount,
        largeFileMode: response.largeFileMode,
        contentTruncated: response.contentTruncated,
      });
    } catch (error) {
      // Failed to load file
      setContent(`// Error loading file: ${error instanceof Error ? error.message : 'Unknown error'}`);
      setLanguage(undefined);
      setFileName('error.txt');
      setFileInfo({});
    } finally {
      setIsLoading(false);
    }
  };

  const handleEditorChange = (value: string) => {
    setContent(value);
    // Update the frontend cache so that switching away and back doesn't lose edits
    if (activeFilePath) {
      WorkspaceService.updateCachedContent(activeFilePath, value);
      // Mark the tab as dirty
      useTabsStore.getState().markDirty(activeFilePath);
    }
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
        />
      </div>
    </div>
  );
}
