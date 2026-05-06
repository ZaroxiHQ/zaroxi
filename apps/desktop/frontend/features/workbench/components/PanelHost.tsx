import { Suspense, useRef, useEffect, useState, useCallback } from 'react';
import { useWorkbenchStore } from '../store/workbenchStore';
import { getActivityItem } from '../config/activityRegistry';
import { ActivityRail } from '@/features/workbench/components/ActivityRail';
import { cn } from '@/lib/utils';
import { LAYOUT } from '../config/layoutConstants';
import { useLayoutMode } from '@/hooks/useLayoutMode';

/**
 * PanelHost - composes left/right panels with refined surface treatment.
 * Uses runtime color tokens and consistent spacing to match the mockup.
 */
interface PanelHostProps {
  className?: string;
  side?: 'left' | 'right';
}

export function PanelHost({ className, side = 'left' }: PanelHostProps) {
  const {
    activeLeftPanel,
    activeRightPanel,
    isLeftPanelVisible,
    isRightPanelVisible,
    leftPanelWidth,
    rightPanelWidth,
    setLeftPanelWidth,
    setRightPanelWidth,
  } = useWorkbenchStore();

  const activePanel = side === 'left' ? activeLeftPanel : activeRightPanel;
  const isVisible = side === 'left' ? isLeftPanelVisible : isRightPanelVisible;
  const panelWidth = side === 'left' ? leftPanelWidth : rightPanelWidth;

  const layoutMode = useLayoutMode();
  const [isResizing, setIsResizing] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const startXRef = useRef(0);
  const startWidthRef = useRef(0);

  const isNarrow = layoutMode === 'narrow';
  const minPanelWidth = side === 'left'
    ? (isNarrow ? 220 : 280)
    : (isNarrow ? LAYOUT.panelRight.minNarrowWidth : LAYOUT.panelRight.minWidth);
  const maxPanelWidth = isNarrow
    ? (side === 'left' ? LAYOUT.panelLeft.maxNarrowWidth : LAYOUT.panelRight.maxNarrowWidth)
    : (side === 'left' ? LAYOUT.panelLeft.maxWidth : LAYOUT.panelRight.maxWidth);
  const factor = side === 'left' ? 0.30 : 0.35;

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsResizing(true);
    startXRef.current = e.clientX;
    startWidthRef.current = panelWidth;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      moveEvent.preventDefault();
      const delta = side === 'left'
        ? moveEvent.clientX - startXRef.current
        : startXRef.current - moveEvent.clientX;

      const newWidth = Math.max(minPanelWidth, Math.min(maxPanelWidth, startWidthRef.current + delta));
      if (side === 'left') {
        setLeftPanelWidth(newWidth);
      } else {
        setRightPanelWidth(newWidth);
      }
    };

    const handleMouseUp = () => {
      setIsResizing(false);
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }, [panelWidth, side, setLeftPanelWidth, setRightPanelWidth, minPanelWidth, maxPanelWidth]);

  useEffect(() => {
    if (!isResizing) return;
    const handleMouseUp = () => {
      setIsResizing(false);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
    document.addEventListener('mouseup', handleMouseUp);
    return () => document.removeEventListener('mouseup', handleMouseUp);
  }, [isResizing]);

  useEffect(() => {
    if (side !== 'right') return;
    const style = document.createElement('style');
    style.textContent = `
      .panel-host-right * {
        max-width: 100% !important;
        min-width: 0 !important;
        box-sizing: border-box !important;
        word-break: break-word !important;
        overflow-wrap: break-word !important;
        white-space: normal !important;
      }
    `;
    document.head.appendChild(style);
    return () => document.head.removeChild(style);
  }, [side]);

  if (!activePanel) return null;

  // When the panel is collapsed we still render a slim, sticky rail so the icons
  // remain visible and usable (consistent compact IDE behavior).
  if (!isVisible) {
    return (
      <div
        className={cn('h-full flex items-end', className)}
        style={{
          flex: '0 0 auto',
          width: LAYOUT.activityRailWidth,
          minWidth: LAYOUT.activityRailWidth,
          maxWidth: LAYOUT.activityRailWidth,
          backgroundColor: 'transparent',
        }}
        role="complementary"
        aria-label={`${side} collapsed panel rail`}
      >
        <div style={{ position: 'sticky', bottom: 12, width: '100%', display: 'flex', justifyContent: 'center', paddingLeft: 6, paddingRight: 6 }}>
          <ActivityRail orientation="vertical" />
        </div>
      </div>
    );
  }

  const activityItem = getActivityItem(activePanel);
  if (!activityItem) {
    console.warn(`No activity item found for panel ID: ${activePanel}`);
    return null;
  }

  const PanelComponent = activityItem.panelComponent;

  return (
    <>
      <div
        ref={panelRef}
        className={cn('h-full overflow-hidden flex flex-col relative min-h-0', side === 'right' ? 'panel-host-right' : '', className)}
        style={{
          flex: '0 1 auto',
          width: 'auto',
          flexBasis: panelWidth,
          minWidth: `${minPanelWidth}px`,
          maxWidth: `min(${maxPanelWidth}px, ${(factor * 100).toFixed(0)}vw)`,
          order: side === 'right' ? 2 : 0,
          backgroundColor: 'var(--color-panel-background)',
          borderRight: side === 'left' ? '1px solid var(--color-divider-subtle)' : 'none',
          borderLeft: side === 'right' ? '1px solid var(--color-divider-subtle)' : 'none',
        }}
      >
        {/* Inner sticky activity rail inside the panel (keeps icons scoped to the panel)
            NOTE: place the rail *inside* the panel and anchor it to the inner edge so it
            visually lives within the panel surface. For left panels we anchor to the right
            (inner edge), for right panels we anchor to the left (inner edge). This makes
            icons appear inside the panel and remain sticky when scrolling.
        */}
        <div
          style={{
            position: 'absolute',
            top: 48,
            bottom: 12,
            // Anchor to the panel inner edge: right for left panels, left for right panels
            right: side === 'left' ? 8 : undefined,
            left: side === 'right' ? 8 : undefined,
            width: LAYOUT.activityRailWidth,
            display: 'flex',
            alignItems: 'flex-end',
            padding: 4,
            zIndex: 20,
            pointerEvents: 'auto',
          }}
          aria-hidden
        >
          <ActivityRail orientation="vertical" compact={true} />
        </div>

        {/* Resize handle */}
        <div
          className={cn('absolute top-0 bottom-0 z-50 resize-handle')}
          style={{
            width: 6,
            right: side === 'left' ? 0 : undefined,
            left: side === 'right' ? 0 : undefined,
            transform: side === 'left' ? 'translateX(3px)' : 'translateX(-3px)',
            cursor: 'col-resize',
            background: 'transparent',
            transition: 'background 120ms ease',
          }}
          onMouseDown={handleMouseDown}
          aria-hidden
        />

        {/* Panel header */}
        <div
          className="px-4 py-2 flex items-center justify-between"
          style={{
            height: 36,
            borderBottom: '1px solid var(--color-divider-subtle)',
            backgroundColor: 'var(--color-activity-rail-background)',
            display: 'flex',
            alignItems: 'center',
            gap: 12,
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--color-text-primary)' }}>{activityItem.label}</span>
            {activityItem.badge !== undefined && activityItem.badge > 0 && (
              <span style={{ padding: '2px 6px', borderRadius: 8, backgroundColor: 'var(--color-accent)', color: 'var(--color-text-on-accent)', fontSize: 11, fontWeight: 600 }}>
                {activityItem.badge}
              </span>
            )}
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            {activityItem.shortcut ? (
              <span style={{ fontSize: 12, color: 'var(--color-text-secondary)', fontFamily: 'var(--font-mono)' }}>{activityItem.shortcut}</span>
            ) : null}
          </div>
        </div>

        {/* Panel body */}
        <div style={{ flex: 1, overflowY: 'auto', overflowX: 'hidden', backgroundColor: 'var(--color-panel-background)' }}>
          <Suspense fallback={
            <div style={{ padding: 12 }}>
              <div style={{ display: 'grid', gap: 8 }}>
                <div style={{ height: 12, background: 'var(--color-divider)', borderRadius: 6 }} />
                <div style={{ height: 12, background: 'var(--color-divider)', borderRadius: 6, width: '60%' }} />
                <div style={{ height: 12, background: 'var(--color-divider)', borderRadius: 6, width: '80%' }} />
              </div>
            </div>
          }>
            <div style={{ minHeight: 0 }}>
              <PanelComponent />
            </div>
          </Suspense>
        </div>
      </div>

      {isResizing && <div className="fixed inset-0 z-40" style={{ cursor: 'col-resize', userSelect: 'none' }} />}
    </>
  );
}
