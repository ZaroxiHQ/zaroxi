import { StatusBar } from './StatusBar';
// import { CommandPalette } from './CommandPalette';
import { EditorContainer } from '@/features/editor/containers/EditorContainer';
import { ActivityRail } from '@/features/workbench/components/ActivityRail';
import { PanelHost } from '@/features/workbench/components/PanelHost';
import { TopBar } from '@/features/workbench/components/TopBar';
import { useWorkbenchStore } from '@/features/workbench/store/workbenchStore';
import { useWorkspaceStore } from '@/features/workspace/stores/useWorkspaceStore';
import { getActivityItem } from '@/features/workbench/config/activityRegistry';
import { useLayoutMode } from '@/hooks/useLayoutMode';
import { Suspense, lazy, useEffect, useRef } from 'react';
import { setupWindowControls } from '@/lib/platform/windowControls';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { LAYOUT } from '@/features/workbench/config/layoutConstants';
import { TabStrip } from '@/features/tabs/TabStrip';

// Lazy load full-width panel components
const SettingsPanel = lazy(() => import('@/features/settings/panel/SettingsPanel'));

export function AppShell() {
  const layoutMode = useLayoutMode();
  const { 
    activeLeftPanel, 
    isLeftPanelVisible, 
    isRightPanelVisible, 
    activeRightPanel,
    togglePanel 
  } = useWorkbenchStore();
  
  // Get activity items for the active panels
  const leftActivity = activeLeftPanel ? getActivityItem(activeLeftPanel) : null;

  // Responsive collapse: auto‑close *only* the left panel when the window gets narrow
  // so the editor always gets enough room. The right (assistant) panel stays open
  // and shrinks via its own responsive min‑width rules.
  const prevWidth = useRef(window.innerWidth);
  useEffect(() => {
    const handleResize = () => {
      const currentWidth = window.innerWidth;
      if (currentWidth < LAYOUT.collapseThreshold && prevWidth.current >= LAYOUT.collapseThreshold) {
        if (isLeftPanelVisible) {
          togglePanel(activeLeftPanel ?? 'explorer');
        }
        // Do NOT close the right panel – let it adapt fluidly.
      }
      prevWidth.current = currentWidth;
    };
    window.addEventListener('resize', handleResize);
    // Run on mount
    handleResize();
    return () => window.removeEventListener('resize', handleResize);
  }, [isLeftPanelVisible, isRightPanelVisible, activeLeftPanel, activeRightPanel, togglePanel]);

  // Determine if we should show full-width panel (only settings)
  const isSettingsActive = leftActivity?.id === 'settings' && isLeftPanelVisible;
  
  // Show left panel when it's visible and not settings
  const showLeftPanel = isLeftPanelVisible && activeLeftPanel && !isSettingsActive;
  // Show right panel when it's visible
  const showRightPanel = isRightPanelVisible && activeRightPanel;
  // Show main content when not showing settings
  const showMainContent = !isSettingsActive;

  // Listen for workspace:opened event and automatically set root path + fetch tree
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    const setup = async () => {
      try {
        unlisten = await listen<{ workspace_id: string; root_path: string }>(
          'workspace:opened',
          (event) => {
            const rootPath = event.payload.root_path;
            const { setRootPath, setTree } = useWorkspaceStore.getState();
            setRootPath(rootPath);
            // Fetch tree for the newly opened workspace
            invoke<{ tree: any[] }>('get_workspace_tree', {
              workspace_id: event.payload.workspace_id,
              root_path: rootPath,
            })
              .then((treeResult) => {
                const treeData = treeResult?.tree ?? [];
                setTree(treeData);
              })
              .catch((e) => console.error('Failed to fetch tree after event:', e));
          }
        );
      } catch {
        // ignore
      }
    };
    setup();

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  useEffect(() => {
    let cleanup: (() => void) | undefined;
    
    const init = async () => {
      cleanup = await setupWindowControls();
    };
    
    init();
    
    return () => {
      if (cleanup && typeof cleanup === 'function') {
        cleanup();
      }
    };
  }, []);

  return (
    <div className="flex flex-col h-screen bg-app text-primary font-sans overflow-hidden" style={{ backgroundColor: 'var(--color-app-background)', position: 'relative' }} data-layout-mode={layoutMode}>
      
      {/* <CommandPalette /> */}
      
      {/* Compact Top Bar */}
      <TopBar />
      
      <div className="flex flex-1 overflow-hidden min-h-0 max-h-full">
        {/* Left Panel (for all left-side panels except settings) */}
        {showLeftPanel && (
          <PanelHost side="left" />
        )}
        
        {/* Settings Panel (full width when active) */}
        {isSettingsActive && (
          <div className="flex-1 flex flex-col overflow-hidden bg-panel relative" style={{ minWidth: 0 }}>
            <Suspense fallback={
              <div className="p-3 bg-panel">
                <div className="space-y-1.5">
                  <div className="h-3 bg-muted rounded animate-pulse w-3/4"></div>
                  <div className="h-3 bg-muted rounded animate-pulse w-1/2"></div>
                  <div className="h-3 bg-muted rounded animate-pulse w-5/6"></div>
                </div>
              </div>
            }>
              <SettingsPanel />
            </Suspense>
          </div>
        )}
        
        {/* Main Content Area (Tab Strip + Editor) */}
        {showMainContent && (
          <div className="flex-1 flex flex-col overflow-hidden min-w-0" style={{ order: 1 }}>
            <TabStrip />
            <div className="flex-1 overflow-hidden bg-editor w-full min-w-0">
              <EditorContainer />
            </div>
          </div>
        )}
        
        {/* Right Panel (Assistant) */}
        {showRightPanel && (
          <PanelHost side="right" />
        )}
      </div>
      

      {/* Global activity rail removed — rail is rendered per‑panel directly under the panel */}

      {/* Status Bar */}
      <StatusBar />
    </div>
  );
}
