import { Icon } from '@/components/ui/Icon';
import { cn } from '@/lib/utils';
import { useWorkbenchStore } from '../store/workbenchStore';
import { useEffect, useState, useRef } from 'react';
import { isTauri, getWindowInstance, windowControlActions } from '@/lib/platform/windowControls';
import { useLayoutMode } from '@/hooks/useLayoutMode';
import { MenuBar } from './MenuBar';
import { LAYOUT } from '../config/layoutConstants';

/**
 * TopBar — responsive 3‑zone layout (left | center | right).
 *
 * Refinements:
 * - Strict 3-zone grid so center can compress (min-width:0).
 * - Consistent control height rhythm and vertical centering.
 * - Menu collapses to hamburger when tiled / narrow.
 * - Brand text hides on half/narrow widths; logo remains.
 * - Search adapts by layoutMode / half-screen detection.
 * - All interactive controls have data-no-drag="true" so drag region is preserved.
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
  const [popupPos, setPopupPos] = useState<{ left: number; top: number }>({ left: 8, top: LAYOUT.topBarHeight + 6 });

  // Heuristic whether window is half/tiled (used to decide hamburger + brand)
  const [isHalfScreen, setIsHalfScreen] = useState<boolean>(() => {
    if (typeof window === 'undefined') return false;
    const w = window.innerWidth || 0;
    return w <= Math.max(700, Math.round((window.screen?.availWidth || w) / 2) + 12);
  });

  useEffect(() => {
    const onResize = () => {
      const w = window.innerWidth || 0;
      setIsHalfScreen(w <= Math.max(700, Math.round((window.screen?.availWidth || w) / 2) + 12));
    };
    window.addEventListener('resize', onResize);
    window.addEventListener('orientationchange', onResize);
    onResize();
    return () => {
      window.removeEventListener('resize', onResize);
      window.removeEventListener('orientationchange', onResize);
    };
  }, []);

  // compute popup position next to the menu button
  useEffect(() => {
    if (!menuOpen || !menuBtnRef.current) return;
    const r = menuBtnRef.current.getBoundingClientRect();
    setPopupPos({ left: Math.max(8, r.left), top: Math.max(8, r.bottom + 6) });
  }, [menuOpen]);

  useEffect(() => {
    let mounted = true;
    const init = async () => {
      const tauriCheck = await isTauri();
      if (!mounted) return;
      setIsTauriEnv(tauriCheck);
      if (tauriCheck) {
        try {
          const w = await getWindowInstance();
          if (!w) return;
          setIsMaximized(await w.isMaximized());
          const unlisten = await w.onResized(async () => setIsMaximized(await w.isMaximized()));
          return () => { if (unlisten) unlisten(); };
        } catch (err) {
          console.error('TopBar: window API error', err);
        }
      }
    };
    init();
    return () => { mounted = false; };
  }, []);

  useEffect(() => {
    function onDocClick(e: MouseEvent) {
      const t = e.target as Node;
      if (menuOpen && popupRef.current && !popupRef.current.contains(t) && menuBtnRef.current && !menuBtnRef.current.contains(t)) {
        setMenuOpen(false);
      }
    }
    window.addEventListener('mousedown', onDocClick);
    return () => window.removeEventListener('mousedown', onDocClick);
  }, [menuOpen]);

  const controlSize = 36; // consistent height for interactive controls
  const iconSize = 14;

  const showHamburger = isHalfScreen || layoutMode === 'narrow';
  const showBrandText = !isHalfScreen && layoutMode !== 'narrow';

  const handleMinimize = async () => { if (isTauriEnv) await windowControlActions.minimize(); };
  const handleMaximize = async () => { if (isTauriEnv) await windowControlActions.maximize(); };
  const handleClose = async () => { if (isTauriEnv) await windowControlActions.close(); };

  return (
    <header
      className={cn('select-none', className)}
      style={{
        display: 'grid',
        gridTemplateColumns: 'auto 1fr auto',
        alignItems: 'center',
        gap: 8,
        padding: '6px 10px',
        height: LAYOUT.topBarHeight,
        background: 'var(--color-title-bar-background)',
        borderBottom: '1px solid var(--color-divider-subtle)',
        boxSizing: 'border-box',
        alignContent: 'center',
        minWidth: 0,
      }}
      {...(isTauriEnv ? { 'data-tauri-drag-region': 'true' } : {})}
    >
      {/* LEFT ZONE */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, minWidth: 0 }}>
        {/* logo mark (always visible) */}
        <div style={{ width: 32, height: 32, display: 'inline-flex', alignItems: 'center', justifyContent: 'center', borderRadius: 8, border: '1px solid var(--color-border)', background: 'transparent', flex: '0 0 auto' }} aria-hidden>
          <Icon name="star" size={14} className="text-accent" />
        </div>

        {/* brand text (responsive) */}
        {showBrandText && (
          <div
            title="Zaroxi Studio"
            style={{
              fontWeight: 600,
              fontSize: 14,
              color: 'var(--color-text-primary)',
              whiteSpace: 'nowrap',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              maxWidth: layoutMode === 'wide' ? 280 : 140,
              flex: '0 1 auto',
            }}
          >
            Zaroxi Studio
          </div>
        )}

        {/* inline menu on wide, otherwise hamburger */}
        <div style={{ marginLeft: showBrandText ? 6 : 2 }}>
          {!showHamburger ? (
            <div style={{ minWidth: 0 }}>
              <MenuBar />
            </div>
          ) : (
            <div style={{ position: 'relative' }}>
              <button
                ref={menuBtnRef}
                onClick={() => setMenuOpen(s => !s)}
                aria-expanded={menuOpen}
                aria-label="Menu"
                data-no-drag="true"
                style={{
                  width: controlSize,
                  height: controlSize,
                  display: 'inline-flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  borderRadius: 8,
                  background: 'transparent',
                  border: '1px solid transparent',
                  padding: 6,
                  color: 'var(--color-text-primary)',
                }}
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden focusable="false">
                  <path d="M3 6h18" />
                  <path d="M3 12h18" />
                  <path d="M3 18h18" />
                </svg>
              </button>

              {menuOpen && (
                <div
                  ref={popupRef}
                  data-no-drag="true"
                  style={{
                    position: 'fixed',
                    top: popupPos.top,
                    left: popupPos.left,
                    zIndex: 80,
                    minWidth: 220,
                    maxWidth: 480,
                    background: 'var(--color-panel-background)',
                    border: '1px solid var(--color-border)',
                    borderRadius: 8,
                    boxShadow: 'var(--shadow-subtle)',
                    overflow: 'hidden',
                  }}
                >
                  <div style={{ padding: 8 }}>
                    <MenuBar compact />
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* CENTER ZONE (flexible spacer so TabStrip below can stay centered) */}
      <div style={{ minWidth: 0, display: 'flex', alignItems: 'center', justifyContent: 'center' }} data-no-drag="true" />

      {/* RIGHT ZONE */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, justifyContent: 'flex-end', minWidth: 0 }}>
        {/* Search: adapt by layout mode and half-screen */}
        {layoutMode === 'narrow' ? (
          <button
            onClick={() => activateLeftPanel('search')}
            aria-label="Open search"
            data-no-drag="true"
            style={{ width: controlSize, height: controlSize, display: 'inline-flex', alignItems: 'center', justifyContent: 'center', borderRadius: 8, background: 'transparent', border: 'none', color: 'var(--color-text-secondary)' }}
          >
            <Icon name="search" size={iconSize} />
          </button>
        ) : (
          <div
            data-no-drag="true"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              padding: '4px 8px',
              borderRadius: 8,
              background: 'var(--color-panel-header-background, var(--color-panel-background))',
              border: '1px solid var(--color-border)',
              minWidth: isHalfScreen ? 120 : 160,
              maxWidth: layoutMode === 'wide' && !isHalfScreen ? 'clamp(160px, 28vw, 420px)' : isHalfScreen ? 220 : 300,
              flex: '0 1 auto',
              boxSizing: 'border-box',
            }}
          >
            <Icon name="search" size={11} />
            <input
              type="search"
              placeholder={layoutMode === 'wide' && !isHalfScreen ? 'Search (Ctrl+Shift+F)' : 'Search'}
              onFocus={() => activateLeftPanel('search')}
              className="bg-transparent outline-none"
              style={{
                color: 'var(--color-text-primary)',
                fontSize: 11,
                border: 'none',
                width: '100%',
                minWidth: 40,
                height: 24,
                flex: '1 1 auto',
                boxSizing: 'border-box',
                padding: '0 6px',
              }}
              aria-label="Search workspace"
              data-no-drag="true"
            />
          </div>
        )}

        {/* Important actions — stable order, don't disappear (lower priority items can be moved inside a menu later) */}
        <button onClick={() => togglePanel('assistant')} title="Assistant" aria-label="Assistant" data-no-drag="true" style={{ width: controlSize, height: controlSize, display: 'inline-flex', alignItems: 'center', justifyContent: 'center', border: 'none', background: 'transparent', color: 'var(--color-text-secondary)' }}>
          <Icon name="assistant" size={16} />
        </button>

        <button
          title={`Dock activity rail: ${activityRailDock === 'panel' ? 'panel' : 'edge'}`}
          onClick={() => setActivityRailDock(activityRailDock === 'panel' ? 'edge' : 'panel')}
          data-no-drag="true"
          style={{ width: controlSize, height: controlSize, display: 'inline-flex', alignItems: 'center', justifyContent: 'center', border: 'none', background: 'transparent', color: 'var(--color-text-secondary)' }}
        >
          <Icon name="pin" size={13} />
        </button>

        {isTauriEnv ? (
          <>
            <button onClick={handleMinimize} aria-label="Minimize" data-no-drag="true" style={{ width: controlSize, height: controlSize, display: 'inline-flex', alignItems: 'center', justifyContent: 'center', border: 'none', background: 'transparent' }}>
              <Icon name="window-minimize" size={12} />
            </button>
            <button onClick={handleMaximize} aria-label={isMaximized ? 'Restore' : 'Maximize'} data-no-drag="true" style={{ width: controlSize, height: controlSize, display: 'inline-flex', alignItems: 'center', justifyContent: 'center', border: 'none', background: 'transparent' }}>
              <Icon name={isMaximized ? 'window-restore' : 'window-maximize'} size={12} />
            </button>
            <button onClick={handleClose} aria-label="Close" data-no-drag="true" style={{ width: controlSize, height: controlSize, display: 'inline-flex', alignItems: 'center', justifyContent: 'center', borderRadius: 8, border: 'none', background: 'transparent' }}>
              <Icon name="window-close" size={12} />
            </button>
          </>
        ) : (
          <button onClick={() => togglePanel('settings')} aria-label="Settings" data-no-drag="true" style={{ width: controlSize, height: controlSize, display: 'inline-flex', alignItems: 'center', justifyContent: 'center', border: 'none', background: 'transparent' }}>
            <Icon name="settings" size={13} />
          </button>
        )}
      </div>
    </header>
  );
}
