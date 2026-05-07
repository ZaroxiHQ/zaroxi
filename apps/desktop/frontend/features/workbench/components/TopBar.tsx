import { Icon } from '@/components/ui/Icon';
import { cn } from '@/lib/utils';
import { useWorkbenchStore } from '../store/workbenchStore';
import { useEffect, useState, useRef } from 'react';
import { isTauri, getWindowInstance, windowControlActions } from '@/lib/platform/windowControls';
import { useLayoutMode } from '@/hooks/useLayoutMode';
import { MenuBar } from './MenuBar';
import { LAYOUT } from '../config/layoutConstants';

/**
 * TopBar — compact, responsive 3‑zone layout (left | center | right).
 *
 * Goals addressed:
 * - Brand should not truncate at normal desktop widths.
 * - Show a hamburger menu on narrow/medium widths that opens the MenuBar as a popup.
 * - Search input collapses to an icon on narrow layouts and is consistently sized.
 * - Top bar height aligned with layout constants for consistency.
 * - Avoid clipping and ensure children can shrink (min-width: 0 where needed).
 */

interface TopBarProps {
  className?: string;
}

export function TopBar({ className }: TopBarProps) {
  const layoutMode = useLayoutMode();
  const { togglePanel, activityRailDock, setActivityRailDock, activateLeftPanel } = useWorkbenchStore();
  const [isMaximized, setIsMaximized] = useState(false);
  const [isTauriEnv, setIsTauriEnv] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const menuBtnRef = useRef<HTMLButtonElement | null>(null);
  const popupRef = useRef<HTMLDivElement | null>(null);

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

  useEffect(() => {
    // Close popup when clicking outside
    function onDocClick(e: MouseEvent) {
      const target = e.target as Node;
      if (menuOpen && popupRef.current && !popupRef.current.contains(target) && menuBtnRef.current && !menuBtnRef.current.contains(target)) {
        setMenuOpen(false);
      }
    }
    window.addEventListener('mousedown', onDocClick);
    return () => window.removeEventListener('mousedown', onDocClick);
  }, [menuOpen]);

  const handleMinimize = async () => { if (isTauriEnv) await windowControlActions.minimize(); };
  const handleMaximize = async () => { if (isTauriEnv) await windowControlActions.maximize(); };
  const handleClose = async () => { if (isTauriEnv) await windowControlActions.close(); };

  // Determine left column sizing and brand max width based on layout mode
  const brandMaxWidth = layoutMode === 'wide' ? 360 : layoutMode === 'medium' ? 220 : 140;

  return (
    <header
      className={cn('select-none', className)}
      style={{
        display: 'grid',
        gridTemplateColumns: 'minmax(160px, 420px) 1fr minmax(140px, 420px)',
        alignItems: 'center',
        gap: 10,
        padding: '4px 10px',
        height: LAYOUT.topBarHeight,
        background: 'var(--color-title-bar-background)',
        borderBottom: '1px solid var(--color-divider-subtle)',
        boxShadow: 'var(--shadow-subtle)',
        boxSizing: 'border-box',
        whiteSpace: 'nowrap',
        overflow: 'visible', // allow popups to be visible
      }}
      {...(isTauriEnv ? { 'data-tauri-drag-region': 'true' } : {})}
    >
      {/* LEFT ZONE — brand + hamburger/menu */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, minWidth: 0 }}>
        {/* Optional hamburger shown for medium/narrow layouts */}
        {(layoutMode === 'narrow' || layoutMode === 'medium') && (
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <button
              ref={menuBtnRef}
              onClick={() => setMenuOpen((s) => !s)}
              aria-expanded={menuOpen}
              aria-label="Menu"
              data-no-drag="true"
              style={{
                width: 34,
                height: 34,
                display: 'inline-flex',
                alignItems: 'center',
                justifyContent: 'center',
                borderRadius: 8,
                background: 'transparent',
                border: '1px solid transparent',
                color: 'var(--color-text-secondary)',
                flex: '0 0 auto',
                padding: 6,
              }}
            >
              <span style={{ fontSize: 16, lineHeight: 1 }}>☰</span>
            </button>

            {/* Popup menu rendered when hamburger is open */}
            {menuOpen && (
              <div
                ref={popupRef}
                style={{
                  position: 'absolute',
                  top: LAYOUT.topBarHeight + 6,
                  left: 8,
                  zIndex: 80,
                  minWidth: 220,
                  maxWidth: 480,
                  background: 'var(--color-panel-background)',
                  border: '1px solid var(--color-border)',
                  borderRadius: 8,
                  boxShadow: 'var(--shadow-subtle)',
                  overflow: 'hidden',
                }}
                data-no-drag="true"
              >
                <div style={{ padding: 8 }}>
                  <MenuBar />
                </div>
              </div>
            )}
          </div>
        )}

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

        <div style={{ minWidth: 0, overflow: 'hidden', flex: '1 1 auto' }}>
          <div
            style={{
              fontWeight: 600,
              fontSize: 14,
              color: 'var(--color-text-primary)',
              whiteSpace: 'nowrap',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              maxWidth: brandMaxWidth,
            }}
            title="Zaroxi Studio"
          >
            Zaroxi Studio
          </div>
        </div>

        {/* On wide layouts show the full MenuBar inline (desktop) */}
        {!isMac && layoutMode === 'wide' && <div style={{ marginLeft: 8 }}><MenuBar /></div>}
      </div>

      {/* CENTER ZONE — reserved spacer (tabs live in editor area) */}
      <div style={{ minWidth: 0, overflow: 'hidden', display: 'flex', alignItems: 'center', justifyContent: 'center' }} data-no-drag="true">
        {/* intentionally empty */}
      </div>

      {/* RIGHT ZONE — search / actions / window controls */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, justifyContent: 'flex-end', minWidth: 0 }}>
        {/* Search: icon on narrow, compact input on medium, full input on wide */}
        {layoutMode === 'narrow' ? (
          <button
            onClick={() => activateLeftPanel('search')}
            aria-label="Open search"
            data-no-drag={isTauriEnv ? 'true' : undefined}
            className="p-2 rounded"
            style={{ background: 'transparent', border: 'none', color: 'var(--color-text-secondary)', flex: '0 0 auto' }}
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
              minWidth: 120,
              maxWidth: 360,
              flex: '0 1 220px',
              boxSizing: 'border-box',
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
                minWidth: 80,
                height: 28,
                flex: '1 1 auto',
                boxSizing: 'border-box',
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
