import { useState, useEffect, useMemo } from 'react';
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
  const [language, setLanguage] = useState<string>('plaintext');
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

  // Add keyboard shortcut for save (Ctrl+S) - separate effect to avoid re-registering on every file change
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        if (activeFilePath) {
          handleEditorSave();
        }
      }
    };
    
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [activeFilePath]);


  const loadFile = async (path: string) => {
    setIsLoading(true);
    setContent('');  // Clear previous content to avoid ghosting
    try {
      // First try to get from frontend cache (no IPC call)
      const cached = WorkspaceService.getCachedDocument(path);
      if (cached) {
        setContent(cached.content);
        setLanguage(cached.language || 'plaintext');
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
      setLanguage(response.language || 'plaintext');
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
      setLanguage('plaintext');
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

  const handleEditorSave = async () => {
    if (!activeFilePath) {
      // No file path to save to
      return;
    }
    
    try {
      await WorkspaceService.saveFile({
        path: activeFilePath,
        content: content,
      });
      // File saved successfully
      // Mark tab as clean after successful save
      if (activeFilePath) {
        useTabsStore.getState().markClean(activeFilePath);
        WorkspaceService.markDocumentClean(activeFilePath);
      }
      // Show a temporary success message
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
      // Failed to save file
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
  };

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
