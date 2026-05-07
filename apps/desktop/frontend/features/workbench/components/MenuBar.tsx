import { useState } from 'react';
import { cn } from '@/lib/utils';
import { useWorkbenchStore } from '../store/workbenchStore';
import { invoke } from '@tauri-apps/api/core';
import { WorkspaceService } from '@/features/workspace/services/workspaceService';

interface MenuItem {
  label: string;
  action: () => void;
}

export function MenuBar({ compact = false }: { compact?: boolean }) {
  const [openMenu, setOpenMenu] = useState<string | null>(null);

  const handleOpenWorkspace = async () => {
    try {
      const result: { selected_path: string | null } = await invoke('open_file_dialog');
      if (result?.selected_path) {
        const selectedPath = result.selected_path;
        await WorkspaceService.openWorkspace({ path: selectedPath });
        useWorkbenchStore.getState().togglePanel('explorer');
      }
    } catch (e) {
      console.error('Failed to open workspace:', e);
    }
  };

  const menus: { label: string; items: MenuItem[] }[] = [
    {
      label: 'File',
      items: [
        { label: 'Open Workspace', action: handleOpenWorkspace },
        { label: 'New File', action: () => {} },
        { label: 'Save', action: () => {} },
      ],
    },
    {
      label: 'Edit',
      items: [
        { label: 'Undo', action: () => {} },
        { label: 'Redo', action: () => {} },
      ],
    },
    {
      label: 'View',
      items: [
        { label: 'Toggle Sidebar', action: () => {} },
      ],
    },
    {
      label: 'Tools',
      items: [
        { label: 'Settings', action: () => {} },
      ],
    },
  ];

  const toggleMenu = (label: string) => {
    if (openMenu === label) {
      setOpenMenu(null);
    } else {
      setOpenMenu(label);
    }
  };

  const closeAll = () => setOpenMenu(null);

  // Compact mode: render labels as a single icon that opens a stacked menu (used for popup/hamburger)
  if (compact) {
    return (
      <div className="flex flex-col">
        {menus.map((menu) => (
          <div key={menu.label} className="py-1">
            <div className="text-xs font-semibold text-muted-foreground px-2">{menu.label}</div>
            <div className="flex flex-col">
              {menu.items.map((item) => (
                <button
                  key={item.label}
                  className="w-full px-3 py-1 text-left text-sm hover:bg-hover-bg transition-colors"
                  onClick={() => {
                    item.action();
                    closeAll();
                  }}
                >
                  {item.label}
                </button>
              ))}
            </div>
          </div>
        ))}
      </div>
    );
  }

  // Inline (full) menu bar for wide layouts
  return (
    <nav className="flex items-center h-8 text-title-bar-foreground select-none" onMouseLeave={closeAll} aria-label="Application menu">
      {menus.map((menu) => (
        <div key={menu.label} className="relative">
          <button
            className={cn(
              'px-3 py-1 text-sm font-medium hover:bg-hover-bg rounded-sm transition-colors',
              openMenu === menu.label && 'bg-hover-bg'
            )}
            onClick={() => toggleMenu(menu.label)}
          >
            {menu.label}
          </button>
          {openMenu === menu.label && (
            <div className="absolute top-full left-0 mt-1 bg-panel shadow-lg border border-border rounded-md py-1 z-50 min-w-[180px]">
              {menu.items.map((item) => (
                <button
                  key={item.label}
                  className="w-full px-3 py-1.5 text-left text-sm hover:bg-hover-bg transition-colors"
                  onClick={() => {
                    item.action();
                    closeAll();
                  }}
                >
                  {item.label}
                </button>
              ))}
            </div>
          )}
        </div>
      ))}
    </nav>
  );
}
