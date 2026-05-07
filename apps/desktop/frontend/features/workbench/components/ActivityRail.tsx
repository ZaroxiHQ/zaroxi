import { Icon } from '@/components/ui/Icon';
import { cn } from '@/lib/utils';
import { useWorkbenchStore } from '../store/workbenchStore';
import { useWorkspaceStore } from '@/features/workspace/stores/useWorkspaceStore';
import { getAvailableActivities } from '../config/activityRegistry';
import { getLanguageIcon } from '@/features/tabs/languageIcons';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/Tooltip';
import { LAYOUT } from '../config/layoutConstants';
import { useLayoutMode } from '@/hooks/useLayoutMode';

/**
 * ActivityRail - minimal, panel-scoped activity rail.
 *
 * Changes made:
 * - Removed decorative "sparkles" and workspace badge to keep the rail clean.
 * - Added `side` prop so the rail only renders activities relevant to the panel side.
 * - Supports `orientation: 'vertical' | 'horizontal' | 'bottom'`.
 *   - 'bottom' = horizontal compact bar intended to sit at the bottom of a panel.
 * - Compact sizing for in-panel usage.
 *
 * Visual intent: compact, consistent, only actionable icons that relate to the panel.
 */
interface ActivityRailProps {
  className?: string;
  side?: 'left' | 'right' | 'both';
  orientation?: 'vertical' | 'horizontal' | 'bottom';
  compact?: boolean;
}

export function ActivityRail({
  className,
  side = 'left',
  orientation = 'vertical',
  compact = false,
}: ActivityRailProps) {
  // Responsive layout mode (wide | medium | narrow) drives compact behaviour
  const layoutMode = useLayoutMode();

  const {
    activeLeftPanel,
    activeRightPanel,
    isLeftPanelVisible,
    isRightPanelVisible,
    togglePanel,
  } = useWorkbenchStore();

  // Determine activities to render. Normally we scope by side, but for bottom rails
  // include the AI assistant as a global quick-action so it is always accessible.
  let activities = getAvailableActivities().filter(a => (side === 'both' ? true : a.side === side)).filter(a => a.id !== 'search');

  if (orientation === 'bottom' && side !== 'both') {
    const assistantItem = getAvailableActivities().find(a => a.id === 'assistant');
    if (assistantItem && !activities.find(a => a.id === 'assistant')) {
      activities = [...activities, assistantItem];
    }
  }

  // Active file from workspace (for file pill in panel footer)
  const { explorerUI } = useWorkspaceStore();
  const activeFilePath = explorerUI?.activeFilePath ?? null;
  const activeFileName = activeFilePath ? activeFilePath.split(/[\\/]/).pop()! : null;
  const activeFileIcon = activeFileName ? getLanguageIcon(activeFileName) : null;

  // Split semantic groups: primary (top/main) and utility (bottom helpers)
  const primaryActivities = activities.filter((a) => a.position !== 'bottom');
  const utilityActivities = activities.filter((a) => a.position === 'bottom');

  // Responsive compacting:
  // - Use explicit `compact` OR layoutMode narrow OR panel width under threshold.
  // - Read the actual panel width from the workbench store so the rail reacts
  //   immediately when the panel is resized (not only when the window crosses breakpoints).
  const { leftPanelWidth, rightPanelWidth } = useWorkbenchStore();
  const hostWidth = side === 'both'
    ? (typeof window !== 'undefined' ? window.innerWidth : undefined)
    : side === 'left'
      ? leftPanelWidth
      : rightPanelWidth;

  // Threshold at which the rail should switch to compact controls.
  const PANEL_COMPACT_THRESHOLD = 260;

  const effectiveCompact = compact || layoutMode === 'narrow' || (typeof hostWidth === 'number' && hostWidth > 0 && hostWidth < PANEL_COMPACT_THRESHOLD);

  // Sizes adapt when compact is true (used inside panel or narrow layouts)
  const primarySize = effectiveCompact ? 28 : 40;
  const utilitySize = effectiveCompact ? 22 : 34;
  const primaryRadius = effectiveCompact ? 7 : 9;
  const utilityRadius = effectiveCompact ? 6 : 8;

  // Helper to check active state
  const isActive = (activityId: string, activitySide: 'left' | 'right') => {
    return activitySide === 'left'
      ? activeLeftPanel === activityId && isLeftPanelVisible
      : activeRightPanel === activityId && isRightPanelVisible;
  };

  // HORIZONTAL bottom-oriented bar (compact) — icons-only, equal spacing, active blends with panel
  if (orientation === 'bottom' || orientation === 'horizontal') {
    // combine primary + utility into a single ordered list for uniform spacing
    const allActivities = [...primaryActivities, ...utilityActivities];

    // Detect whether any activity in this rail is currently active (open).
    const panelHasActive = allActivities.some((a) => isActive(a.id, a.side));

    // thin top strip height that will visually blend with the panel when active
    const topStripHeight = effectiveCompact ? 4 : 6;

    return (
      <TooltipProvider delayDuration={180}>
        <div
          className={cn('w-full flex items-center', className)}
          style={{
            position: 'relative', // needed for the absolute top strip
            height: compact ? 36 + topStripHeight : 44 + topStripHeight,
            display: 'flex',
            // use space-evenly so icons are distributed consistently across the panel width
            justifyContent: 'space-evenly',
            padding: compact ? `${2 + topStripHeight}px 6px 2px` : `${4 + topStripHeight}px 8px 4px`,
            boxSizing: 'border-box',
            // PanelHost supplies the rail surface; this container stays transparent so it stretches edge-to-edge.
            background: 'transparent',
            alignItems: 'center',
            width: '100%',
          }}
          role="toolbar"
          aria-label="Panel activity rail"
        >
          {/* Top strip: when any activity in this rail is active, blend strip with the panel color.
              This creates an immediate visual connection between the open panel and its rail. */}
          <div
            aria-hidden
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              right: 0,
              height: topStripHeight,
              background: panelHasActive ? 'var(--color-panel-background)' : 'transparent',
              pointerEvents: 'none',
            }}
          />

          {allActivities.map((activity) => {
            const active = isActive(activity.id, activity.side);
            const isUtility = activity.position === 'bottom';
            const size = isUtility ? utilitySize : primarySize;
            const radius = isUtility ? utilityRadius : primaryRadius;
            const iconSize = isUtility ? 14 : 16;

            return (
              <Tooltip key={activity.id}>
                <TooltipTrigger asChild>
                  <button
                    onClick={() => togglePanel(activity.id)}
                    aria-label={activity.label}
                    data-no-drag="true"
                    style={{
                      width: size,
                      height: size,
                      borderRadius: radius,
                      display: 'inline-flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      // Active button blends with panel background; inactive buttons sit on the rail surface.
                      background: active ? 'var(--color-panel-background)' : 'var(--color-activity-rail-background)',
                      border: 'none',
                      color: active ? 'var(--color-text-on-surface)' : 'var(--color-text-secondary)',
                      boxShadow: 'none',
                      cursor: 'pointer',
                      transition: 'color 120ms ease, background 120ms ease',
                      padding: 4,
                      ariaPressed: active ? 'true' : 'false',
                    }}
                  >
                    <Icon name={activity.icon as any} size={iconSize} />
                  </button>
                </TooltipTrigger>
                <TooltipContent side="top" className="border bg-panel" />
              </Tooltip>
            );
          })}
        </div>
      </TooltipProvider>
    );
  }

  // VERTICAL (inside collapsed dock) - stacked compact icons (keeps behaviour but clean)
  return (
    <TooltipProvider delayDuration={220}>
      <aside
        className={cn('flex flex-col items-center', className)}
        style={{
          width: LAYOUT.activityRailWidth,
          minWidth: LAYOUT.activityRailWidth,
          backgroundColor: 'transparent',
          paddingTop: 8,
          paddingBottom: 8,
          gap: 8,
          boxSizing: 'border-box',
          pointerEvents: 'auto',
        }}
        aria-label="Panel Activity Rail"
      >
        <div style={{ flex: 1 }} />

        <div style={{ display: 'flex', flexDirection: 'column', gap: 8, alignItems: 'center' }}>
          {primaryActivities.map((activity) => {
            const active = isActive(activity.id, activity.side);
            return (
              <Tooltip key={activity.id}>
                <TooltipTrigger asChild>
                  <button
                    onClick={() => togglePanel(activity.id)}
                    aria-label={activity.label}
                    data-no-drag="true"
                    style={{
                      width: primarySize,
                      height: primarySize,
                      borderRadius: primaryRadius,
                      display: 'inline-flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      // Inactive buttons filled; active button uses the panel surface color
                      // so the user can immediately associate the open panel with this icon.
                      background: active ? 'var(--color-panel-background)' : 'var(--color-activity-rail-background)',
                      border: 'none',
                      color: active ? 'var(--color-text-on-surface)' : 'var(--color-text-secondary)',
                      boxShadow: 'none',
                      cursor: 'pointer',
                      transition: 'color 140ms ease',
                    }}
                  >
                    <Icon name={activity.icon as any} size={16} />
                  </button>
                </TooltipTrigger>
                <TooltipContent side="right" className="border bg-panel" />
              </Tooltip>
            );
          })}
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 8, alignItems: 'center' }}>
          {utilityActivities.map((activity) => {
            const active = isActive(activity.id, activity.side);
            return (
              <Tooltip key={activity.id}>
                <TooltipTrigger asChild>
                  <button
                    onClick={() => togglePanel(activity.id)}
                    aria-label={activity.label}
                    data-no-drag="true"
                    style={{
                      width: utilitySize,
                      height: utilitySize,
                      borderRadius: utilityRadius,
                      display: 'inline-flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      background: active ? 'var(--color-accent)' : 'transparent',
                      color: active ? 'var(--color-text-on-accent)' : 'var(--color-text-secondary)',
                      border: active ? '1px solid var(--color-border)' : '1px solid transparent',
                      boxShadow: active ? '0 6px 18px var(--color-accent-glow)' : 'none',
                      cursor: 'pointer',
                      transition: 'all 140ms ease',
                    }}
                  >
                    <Icon name={activity.icon as any} size={14} />
                  </button>
                </TooltipTrigger>
                <TooltipContent side="right" className="border bg-panel" />
              </Tooltip>
            );
          })}
        </div>
      </aside>
    </TooltipProvider>
  );
}
