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
 * TopBar - refined to match Zaroxi Studio mockup:
 * - Thin elegant bar
 * - Brand + optional MenuBar on the left
 * - Tab strip centered and visually connected to editor
 * - Compact action icons on the right (including Tauri window controls)
 */
export function TopBar({ className }: TopBarProps) {
  const layoutMode = useLayoutMode();
  const { togglePanel } = useWorkbenchStore();
  const [isMaximized, setIsMaximized] = useState(false);
  const [isTauriEnv, setIsTauriEnv] = useState(false);

  // macOS gets native menu bar, other platforms get the React MenuBar
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

  const handleMinimize = async () => {
    if (isTauriEnv) await windowControlActions.minimize();
  };

  const handleMaximize = async () => {
    if (isTauriEnv) {
      await windowControlActions.maximize();
      setTimeout(async () => {
        try {
          const currentWindow = await getWindowInstance();
          if (currentWindow) setIsMaximized(await currentWindow.isMaximized());
        } catch {}
      }, 80);
    }
  };

  const handleClose = async () => {
    if (isTauriEnv) await windowControlActions.close();
  };

  return (
    <header
      className={cn(
        'flex items-center px-4',
        'select-none',
        className
      )}
      style={{
        height: 44,
        backgroundColor: 'var(--color-title-bar-background, var(--outer-shell))',
        borderBottom: '1px solid var(--color-divider-subtle)',
        alignItems: 'center',
      }}
      {...(isTauriEnv ? { 'data-tauri-drag-region': 'true' } : {})}
    >
      {/* Left: brand + menu */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, minWidth: 220 }}>
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
            boxShadow: '0 2px 8px rgba(0,0,0,0.35)',
          }}
          aria-hidden
        >
          <Icon name="star" size={14} />
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', lineHeight: 1 }}>
          <div style={{ fontWeight: 600, fontSize: 14, color: 'var(--color-text-primary)' }}>Zaroxi Studio</div>
          <div style={{ fontSize: 11, color: 'var(--color-text-secondary)' }}>Workspace</div>
        </div>

        {!isMac && <MenuBar />}
      </div>

      {/* Center: tab strip (visually connected to editor) */}
      <div style={{ flex: 1, display: 'flex', justifyContent: 'center', paddingLeft: 12, paddingRight: 12 }}>
        <div
          style={{
            width: '100%',
            maxWidth: 980,
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            padding: '4px 8px',
            backgroundColor: 'var(--color-tab-strip-background, rgba(0,0,0,0))',
            border: '1px solid transparent',
            borderRadius: 8,
            boxShadow: 'none',
          }}
          data-no-drag={isTauriEnv ? 'true' : undefined}
        >
          <TabStrip />
        </div>
      </div>

      {/* Right: compact action icons / window controls */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 160 }}>
        <button
          onClick={() => togglePanel('search')}
          className="p-1 radius-sm"
          title="Search"
          style={{ color: 'var(--color-text-secondary)', background: 'transparent', border: 'none' }}
          data-no-drag="true"
        >
          <Icon name="search" size={16} />
        </button>

        <button
          onClick={() => togglePanel('git')}
          className="p-1 radius-sm"
          title="Source Control"
          style={{ color: 'var(--color-text-secondary)', background: 'transparent', border: 'none' }}
          data-no-drag="true"
        >
          <Icon name="git-branch" size={16} />
        </button>

        <button
          onClick={() => togglePanel('assistant')}
          className="p-1 radius-sm"
          title="Assistant"
          style={{ color: 'var(--color-text-secondary)', background: 'transparent', border: 'none' }}
          data-no-drag="true"
        >
          <Icon name="assistant" size={16} />
        </button>

        {isTauriEnv ? (
          <>
            <button
              onClick={handleMinimize}
              className="w-8 h-8 flex items-center justify-center rounded hover:bg-hover-bg transition-colors"
              aria-label="Minimize"
              data-no-drag="true"
            >
              <Icon name="window-minimize" size={12} />
            </button>
            <button
              onClick={handleMaximize}
              className="w-8 h-8 flex items-center justify-center rounded hover:bg-hover-bg transition-colors"
              aria-label={isMaximized ? 'Restore' : 'Maximize'}
              data-no-drag="true"
            >
              <Icon name={isMaximized ? 'window-restore' : 'window-maximize'} size={12} />
            </button>
            <button
              onClick={handleClose}
              className="w-8 h-8 flex items-center justify-center rounded hover:bg-destructive/10 hover:text-destructive transition-colors"
              aria-label="Close"
              data-no-drag="true"
            >
              <Icon name="window-close" size={12} />
            </button>
          </>
        ) : (
          <button
            onClick={() => togglePanel('settings')}
            className="w-8 h-8 flex items-center justify-center rounded hover:bg-hover-bg transition-colors"
            aria-label="Settings"
            data-no-drag="true"
          >
            <Icon name="settings" size={13} />
          </button>
        )}
      </div>
    </header>
  );
}
