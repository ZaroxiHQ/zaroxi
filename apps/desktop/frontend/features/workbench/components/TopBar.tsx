import { Icon } from '@/components/ui/Icon';
import { cn } from '@/lib/utils';
import { useWorkbenchStore } from '../store/workbenchStore';
import { useEffect, useState } from 'react';
import { isTauri, getWindowInstance, windowControlActions } from '@/lib/platform/windowControls';
import { useLayoutMode } from '@/hooks/useLayoutMode';
import { MenuBar } from './MenuBar';
import { TabStrip } from '@/features/tabs/TabStrip';

interface TopBarProps {
  className?: string;
}

/**
 * TopBar - compact and premium
 * - Uses canonical --color-* tokens
 * - Tab strip visually connects to editor surface
 */
export function TopBar({ className }: TopBarProps) {
  const layoutMode = useLayoutMode();
  const { togglePanel, activityRailDock, setActivityRailDock, activateLeftPanel } = useWorkbenchStore();
  const [isMaximized, setIsMaximized] = useState(false);
  const [isTauriEnv, setIsTauriEnv] = useState(false);

  const isMac = typeof navigator !== 'undefined' && navigator.platform?.toLowerCase().includes('mac');

  useEffect(() => {
    const checkTauri = async () => {
      const tauriCheck = await isTauri();
      setIsTauriEnv(tauriCheck);
      if (tauriCheck) {
        try {
          const currentWindow = await getWindowInstance();
          if (!currentWindow) return;
          const updateMaximized = async () => {
            setIsMaximized(await currentWindow.isMaximized());
          };
          await updateMaximized();

          const unlisten = await currentWindow.onResized(() => {
            updateMaximized();
          });

          return () => {
            if (unlisten) unlisten();
          };
        } catch (err) {
          console.error('TopBar: window listeners error', err);
        }
      }
    };
    checkTauri();
  }, []);

  const handleMinimize = async () => { if (isTauriEnv) await windowControlActions.minimize(); };
  const handleMaximize = async () => { if (isTauriEnv) await windowControlActions.maximize(); };
  const handleClose = async () => { if (isTauriEnv) await windowControlActions.close(); };

  return (
    <header
      className={cn('flex items-center px-3 select-none', className)}
      style={{
        height: 44,
        backgroundColor: 'var(--color-title-bar-background, var(--color-outer-shell))',
        borderBottom: '1px solid var(--color-divider-subtle)',
        boxShadow: 'inset 0 -1px 0 rgba(255,255,255,0.02)',
        alignItems: 'center',
        gap: 12,
        paddingLeft: 12,
        paddingRight: 12,
        boxSizing: 'border-box',
        // Allow overflow so right-side controls never get visually clipped;
        // keep layout on a single line but allow inner items to shrink.
        overflow: 'visible',
        flexWrap: 'nowrap',
      }}
      {...(isTauriEnv ? { 'data-tauri-drag-region': 'true' } : {})}
    >
      {/* Left: responsive compact brand */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, minWidth: 0 }}>
        {/* Logo square — fixed size and non-shrinking so it remains visible on narrow screens */}
        <div
          style={{
            width: 32,
            height: 32,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: 8,
            background: 'linear-gradient(180deg, rgba(255,255,255,0.02), rgba(0,0,0,0.03))',
            border: '1px solid var(--color-border)',
          }}
          aria-hidden
        >
          <Icon name="star" size={14} />
        </div>

        {/* Brand text — allow shrinking and ellipsising on narrow layouts.
            We intentionally remove the "Workspace" subtitle per the request. */}
        <div style={{ display: 'flex', flexDirection: 'column', lineHeight: 1, minWidth: 0 }}>
          <div
            style={{
              fontWeight: 600,
              fontSize: 14,
              color: 'var(--color-text-primary)',
              whiteSpace: 'nowrap',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              maxWidth: layoutMode === 'narrow' ? 120 : 220,
            }}
          >
            Zaroxi Studio
          </div>
        </div>

        {/* MenuBar hidden on narrow layouts to keep the top bar compact and responsive */}
        {!isMac && layoutMode !== 'narrow' && <MenuBar />}
      </div>

      {/* Center: empty spacer (tabs moved out of top bar). */}
      <div style={{ flex: 1 }} />

      {/* Right: actions (compact search moved here on the right side) */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
        {/* Compact search: show icon on narrow layouts, full input otherwise.
            Input area is now flexible and allowed to shrink so the right-side icons never get clipped. */}
        {layoutMode === 'narrow' ? (
          <button
            onClick={() => activateLeftPanel('search')}
            aria-label="Open search"
            className="p-2 rounded"
            data-no-drag={isTauriEnv ? 'true' : undefined}
            style={{
              color: 'var(--color-text-secondary)',
              background: 'transparent',
              border: 'none',
              height: 32,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              paddingLeft: 6,
              paddingRight: 6,
              flex: '0 0 auto',
            }}
          >
            <Icon name="search" size={14} />
          </button>
        ) : (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              padding: '2px 6px',
              borderRadius: 8,
              background: 'var(--color-panel-header-background)',
              border: '1px solid var(--color-border)',
              minWidth: 120,
              maxWidth: 220,
              height: 28,
              flex: '0 1 auto',
            }}
            data-no-drag={isTauriEnv ? 'true' : undefined}
          >
            <Icon name="search" size={12} />
            <input
              type="search"
              placeholder="Search (Ctrl+Shift+F)"
              onFocus={() => activateLeftPanel('search')}
              className="bg-transparent outline-none"
              style={{
                color: 'var(--color-text-primary)',
                fontSize: 12,
                padding: '2px 4px',
                border: 'none',
                background: 'transparent',
                width: '100%',
                minWidth: 80,
                height: '100%',
              }}
              aria-label="Search workspace"
              data-no-drag="true"
            />
          </div>
        )}

        {/* Assistant quick-action (kept) */}
        <button onClick={() => togglePanel('assistant')} title="Assistant" style={{ color: 'var(--color-text-secondary)', background: 'transparent', border: 'none', flex: '0 0 auto' }} data-no-drag="true">
          <Icon name="assistant" size={16} />
        </button>

        {/* Activity rail dock toggle (panel / edge) */}
        <button
          title={`Dock activity rail: ${activityRailDock === 'panel' ? 'panel (inside)' : 'edge (dock)'} - click to toggle`}
          onClick={() => setActivityRailDock(activityRailDock === 'panel' ? 'edge' : 'panel')}
          className="w-8 h-8 flex items-center justify-center rounded"
          data-no-drag="true"
          style={{ color: 'var(--color-text-secondary)', background: 'transparent', border: 'none', flex: '0 0 auto' }}
        >
          <Icon name={activityRailDock === 'panel' ? 'pin' : 'pin'} size={13} />
        </button>

        {isTauriEnv ? (
          <>
            <button onClick={handleMinimize} className="w-8 h-8 flex items-center justify-center rounded" aria-label="Minimize" data-no-drag="true" style={{ flex: '0 0 auto' }}>
              <Icon name="window-minimize" size={12} />
            </button>
            <button onClick={handleMaximize} className="w-8 h-8 flex items-center justify-center rounded" aria-label={isMaximized ? 'Restore' : 'Maximize'} data-no-drag="true" style={{ flex: '0 0 auto' }}>
              <Icon name={isMaximized ? 'window-restore' : 'window-maximize'} size={12} />
            </button>
            <button onClick={handleClose} className="w-8 h-8 flex items-center justify-center rounded hover:bg-hover-bg" aria-label="Close" data-no-drag="true" style={{ flex: '0 0 auto' }}>
              <Icon name="window-close" size={12} />
            </button>
          </>
        ) : (
          <button onClick={() => togglePanel('settings')} className="w-8 h-8 flex items-center justify-center rounded" aria-label="Settings" data-no-drag="true" style={{ flex: '0 0 auto' }}>
            <Icon name="settings" size={13} />
          </button>
        )}
      </div>
    </header>
  );
}
