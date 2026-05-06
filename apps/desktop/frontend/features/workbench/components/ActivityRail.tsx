import { Icon } from '@/components/ui/Icon';
import { cn } from '@/lib/utils';
import { useWorkbenchStore } from '../store/workbenchStore';
import { useWorkspaceStore } from '@/features/workspace/stores/useWorkspaceStore';
import { getAvailableActivities } from '../config/activityRegistry';
import { getLanguageIcon } from '@/features/tabs/languageIcons';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/Tooltip';
import { LAYOUT } from '../config/layoutConstants';

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
  const {
    activeLeftPanel,
    activeRightPanel,
    isLeftPanelVisible,
    isRightPanelVisible,
    togglePanel,
  } = useWorkbenchStore();

  const activities = getAvailableActivities().filter(a => (side === 'both' ? true : a.side === side));

  // Active file from workspace (for file pill in panel footer)
  const { explorerUI } = useWorkspaceStore();
  const activeFilePath = explorerUI?.activeFilePath ?? null;
  const activeFileName = activeFilePath ? activeFilePath.split(/[\\/]/).pop()! : null;
  const activeFileIcon = activeFileName ? getLanguageIcon(activeFileName) : null;

  // Split semantic groups: primary (top/main) and utility (bottom helpers)
  const primaryActivities = activities.filter((a) => a.position !== 'bottom');
  const utilityActivities = activities.filter((a) => a.position === 'bottom');

  // Sizes adapt when compact is true (used inside panel)
  // Reduced sizes for a more compact bottom rail as requested.
  const primarySize = compact ? 32 : 40;
  const utilitySize = compact ? 28 : 34;
  const primaryRadius = compact ? 8 : 9;
  const utilityRadius = compact ? 7 : 8;

  // Helper to check active state
  const isActive = (activityId: string, activitySide: 'left' | 'right') => {
    return activitySide === 'left'
      ? activeLeftPanel === activityId && isLeftPanelVisible
      : activeRightPanel === activityId && isRightPanelVisible;
  };

  // HORIZONTAL bottom-oriented bar (compact) — icons-only, minimal styling
  // Compact height and active visual treatment that visually matches the panel surface.
  if (orientation === 'bottom' || orientation === 'horizontal') {
    return (
      <TooltipProvider delayDuration={180}>
        <div
          className={cn('w-full flex items-center justify-center', className)}
          style={{
            height: compact ? 36 : 44,
            display: 'flex',
            gap: 6,
            padding: compact ? '2px 4px' : '4px 6px',
            boxSizing: 'border-box',
            // Keep the rail visually minimal and let active buttons signal the open panel.
            // Use transparent background so the rail visually reads as part of the editor surface,
            // while the active button uses the panel surface color for clear association.
            background: 'transparent',
            alignItems: 'center',
            justifyContent: 'center',
            width: '100%',
          }}
          role="toolbar"
          aria-label="Panel activity rail"
        >
          <div style={{ display: 'flex', gap: 8, alignItems: 'center', justifyContent: 'center', width: '100%' }}>
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
                        background: active ? 'var(--color-panel-background)' : 'transparent',
                        border: active ? `1px solid var(--color-border)` : 'none',
                        color: active ? 'var(--color-text-on-surface)' : 'var(--color-text-secondary)',
                        boxShadow: active ? '0 6px 18px rgba(2,6,23,0.6)' : 'none',
                        cursor: 'pointer',
                        transition: 'all 140ms ease',
                        padding: 4,
                        ariaPressed: active ? 'true' : 'false',
                      }}
                    >
                      <Icon name={activity.icon as any} size={16} />
                    </button>
                  </TooltipTrigger>
                  <TooltipContent side="top" className="border bg-panel shadow-sm" />
                </Tooltip>
              );
            })}
          </div>

          {/* compact utilities are rendered without extra decoration but still show active color */}
          {utilityActivities.length > 0 && (
            <div style={{ display: 'flex', gap: 6, alignItems: 'center', justifyContent: 'flex-end' }}>
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
                          background: active ? 'var(--color-panel-background)' : 'transparent',
                          border: active ? `1px solid var(--color-border)` : 'none',
                          color: active ? 'var(--color-accent)' : 'var(--color-text-secondary)',
                          boxShadow: active ? '0 6px 18px var(--color-accent-glow)' : 'none',
                          cursor: 'pointer',
                          transition: 'all 140ms ease',
                          padding: 4,
                        }}
                      >
                        <Icon name={activity.icon as any} size={14} />
                      </button>
                    </TooltipTrigger>
                    <TooltipContent side="top" className="border bg-panel shadow-sm" />
                  </Tooltip>
                );
              })}
            </div>
          )}
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
        {/* top spacer */}
        <div style={{ height: 8 }} />

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
                      background: active ? 'linear-gradient(180deg, rgba(108,99,255,0.06), rgba(82,70,229,0.03))' : 'transparent',
                      border: active ? '1px solid var(--color-border)' : '1px solid transparent',
                      color: active ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
                      boxShadow: active ? '0 8px 20px var(--color-accent-glow)' : 'none',
                      cursor: 'pointer',
                      transition: 'all 140ms ease',
                    }}
                  >
                    <Icon name={activity.icon as any} size={16} />
                  </button>
                </TooltipTrigger>
                <TooltipContent side="right" className="border bg-panel shadow-lg" />
              </Tooltip>
            );
          })}
        </div>

        <div style={{ height: 8 }} />

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
                <TooltipContent side="right" className="border bg-panel shadow-lg" />
              </Tooltip>
            );
          })}
        </div>

        <div style={{ height: 8 }} />
      </aside>
    </TooltipProvider>
  );
}
