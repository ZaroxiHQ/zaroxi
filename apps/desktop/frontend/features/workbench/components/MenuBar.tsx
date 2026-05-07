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

  // Compact mode: stacked labels (used in hamburger popup)
  if (compact) {
    return (
      <div style={{ minWidth: 220 }}>
        {menus.map((menu) => (
          <div key={menu.label} style={{ padding: '6px 0' }}>
            <div style={{ fontSize: 12, fontWeight: 400, color: 'var(--color-text-muted)', padding: '4px 8px' }}>{menu.label}</div>
            <div>
              {menu.items.map((item) => (
                <button
                  key={item.label}
                  style={{ width: '100%', textAlign: 'left', padding: '8px 12px', background: 'transparent', border: 'none', color: 'var(--color-text-primary)' }}
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

  // Inline (full) menu bar for wide layouts — compact height and aligned center
  return (
    <nav style={{ display: 'flex', alignItems: 'center', height: 32, gap: 4 }} onMouseLeave={closeAll} aria-label="Application menu">
      {menus.map((menu) => (
        <div key={menu.label} style={{ position: 'relative' }}>
          <button
            style={{
              padding: '6px 10px',
              fontSize: 12,
              fontWeight: 400,
              background: openMenu === menu.label ? 'var(--color-panel-header-background)' : 'transparent',
              borderRadius: 6,
              color: 'var(--color-text-primary)',
              border: 'none',
            }}
            onClick={() => toggleMenu(menu.label)}
          >
            {menu.label}
          </button>
          {openMenu === menu.label && (
            <div style={{ position: 'absolute', top: '100%', left: 0, marginTop: 6, background: 'var(--color-panel-background)', boxShadow: 'var(--shadow-subtle)', border: '1px solid var(--color-border)', borderRadius: 8, padding: 6, zIndex: 60, minWidth: 180 }}>
              {menu.items.map((item) => (
                <button
                  key={item.label}
                  style={{ width: '100%', textAlign: 'left', padding: '8px 10px', background: 'transparent', border: 'none', color: 'var(--color-text-primary)' }}
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
