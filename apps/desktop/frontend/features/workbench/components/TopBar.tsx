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
 * TopBar — responsive 3‑zone layout (left | center | right).
 *
 * Strategy:
 * - Use CSS grid with three columns:
 *   left: minmax(120px, 320px)
 *   center: 1fr (min-width: 0 so it can shrink)
 *   right: minmax(120px, 420px)
 * - Center contains TabStrip; it uses overflow-x auto so tabs never break layout.
 * - Right zone holds search and compact actions; on narrow layouts the search collapses to icon.
 * - Left zone contains brand + optional menu; keeps min/max width to avoid pushing other zones.
 */
export function TopBar({ className }: TopBarProps) {
  const layoutMode = useLayoutMode();
  const { togglePanel, activityRailDock, setActivityRailDock, activateLeftPanel } = useWorkbenchStore();
  const [isMaximized, setIsMaximized] = useState(false);
  const [isTauriEnv, setIsTauriEnv] = useState(false);

  const isMac = typeof navigator !== 'undefined' && navigator.platform?.toLowerCase().includes('mac');

  useEffect(() => {
    let mounted = true;
    const checkTauri = async () => {
      const tauriCheck = await isTauri();
      if (!mounted) return;
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
    return () => { mounted = false; };
  }, []);

  const handleMinimize = async () => { if (isTauriEnv) await windowControlActions.minimize(); };
  const handleMaximize = async () => { if (isTauriEnv) await windowControlActions.maximize(); };
  const handleClose = async () => { if (isTauriEnv) await windowControlActions.close(); };

  return (
    <header
      className={cn('select-none', className)}
      style={{
        display: 'grid',
        gridTemplateColumns: 'minmax(140px, 320px) 1fr minmax(140px, 420px)',
        alignItems: 'center',
        gap: 12,
        padding: '6px 12px',
        height: 44,
        background: 'var(--color-title-bar-background)',
        borderBottom: '1px solid var(--color-divider-subtle)',
        boxShadow: 'var(--shadow-subtle)',
        boxSizing: 'border-box',
      }}
      {...(isTauriEnv ? { 'data-tauri-drag-region': 'true' } : {})}
    >
      {/* LEFT ZONE — brand + optional menu */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, minWidth: 0 }}>
        <div
          style={{
            width: 32,
            height: 32,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: 8,
            border: '1px solid var(--color-border)',
            background: 'transparent',
            flex: '0 0 auto',
          }}
          aria-hidden
        >
          <Icon name="star" size={14} />
        </div>

        <div style={{ minWidth: 0, overflow: 'hidden' }}>
          <div
            style={{
              fontWeight: 600,
              fontSize: 14,
              color: 'var(--color-text-primary)',
              whiteSpace: 'nowrap',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              maxWidth: 220,
            }}
            title="Zaroxi Studio"
          >
            Zaroxi Studio
          </div>
        </div>

        {/* Menu hidden on narrow */}
        {!isMac && layoutMode !== 'narrow' && <div style={{ marginLeft: 8 }}><MenuBar /></div>}
      </div>

      {/* CENTER ZONE — Tab strip (shrinkable, scrollable) */}
      <div style={{ minWidth: 0, overflow: 'hidden', display: 'flex', alignItems: 'center' }} data-no-drag="true">
        <div style={{ flex: 1, minWidth: 0, overflow: 'hidden' }}>
          <TabStrip />
        </div>
      </div>

      {/* RIGHT ZONE — search / actions / window controls */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, justifyContent: 'flex-end', minWidth: 0 }}>
        {layoutMode === 'narrow' ? (
          <button
            onClick={() => activateLeftPanel('search')}
            aria-label="Open search"
            data-no-drag={isTauriEnv ? 'true' : undefined}
            className="p-2 rounded"
            style={{ background: 'transparent', border: 'none', color: 'var(--color-text-secondary)' }}
          >
            <Icon name="search" size={14} />
          </button>
        ) : (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              padding: '4px 8px',
              borderRadius: 8,
              background: 'var(--color-panel-header-background, var(--color-panel-background))',
              border: '1px solid var(--color-border)',
              minWidth: 160,
              maxWidth: 360,
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
                fontSize: 13,
                border: 'none',
                width: '100%',
                minWidth: 120,
                height: 28,
              }}
              aria-label="Search workspace"
              data-no-drag="true"
            />
          </div>
        )}

        <button onClick={() => togglePanel('assistant')} title="Assistant" aria-label="Assistant" data-no-drag="true" style={{ background: 'transparent', border: 'none', color: 'var(--color-text-secondary)' }}>
          <Icon name="assistant" size={16} />
        </button>

        <button
          title={`Dock activity rail: ${activityRailDock === 'panel' ? 'panel' : 'edge'}`}
          onClick={() => setActivityRailDock(activityRailDock === 'panel' ? 'edge' : 'panel')}
          className="rounded p-1"
          data-no-drag="true"
          style={{ background: 'transparent', border: 'none', color: 'var(--color-text-secondary)' }}
        >
          <Icon name="pin" size={13} />
        </button>

        {isTauriEnv ? (
          <>
            <button onClick={handleMinimize} aria-label="Minimize" data-no-drag="true" className="rounded p-1" style={{ background: 'transparent', border: 'none' }}>
              <Icon name="window-minimize" size={12} />
            </button>
            <button onClick={handleMaximize} aria-label={isMaximized ? 'Restore' : 'Maximize'} data-no-drag="true" className="rounded p-1" style={{ background: 'transparent', border: 'none' }}>
              <Icon name={isMaximized ? 'window-restore' : 'window-maximize'} size={12} />
            </button>
            <button onClick={handleClose} aria-label="Close" data-no-drag="true" className="rounded p-1 hover:bg-hover-bg" style={{ background: 'transparent', border: 'none' }}>
              <Icon name="window-close" size={12} />
            </button>
          </>
        ) : (
          <button onClick={() => togglePanel('settings')} aria-label="Settings" data-no-drag="true" className="rounded p-1" style={{ background: 'transparent', border: 'none' }}>
            <Icon name="settings" size={13} />
          </button>
        )}
      </div>
    </header>
  );
}
