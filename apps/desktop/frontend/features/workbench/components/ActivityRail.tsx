import { Icon } from '@/components/ui/Icon';
import { cn } from '@/lib/utils';
import { useWorkbenchStore } from '../store/workbenchStore';
import { getAvailableActivities } from '../config/activityRegistry';
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
  side?: 'left' | 'right';
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

  const activities = getAvailableActivities().filter(a => a.side === side);

  // Split semantic groups: primary (top/main) and utility (bottom helpers)
  const primaryActivities = activities.filter((a) => a.position !== 'bottom');
  const utilityActivities = activities.filter((a) => a.position === 'bottom');

  // Sizes adapt when compact is true (used inside panel)
  const primarySize = compact ? 36 : 44;
  const utilitySize = compact ? 30 : 38;
  const primaryRadius = compact ? 8 : 10;
  const utilityRadius = compact ? 8 : 9;

  // Helper to check active state
  const isActive = (activityId: string, activitySide: 'left' | 'right') => {
    return activitySide === 'left'
      ? activeLeftPanel === activityId && isLeftPanelVisible
      : activeRightPanel === activityId && isRightPanelVisible;
  };

  // HORIZONTAL bottom-oriented bar (compact) — refined, theme-aware, icons-only
  if (orientation === 'bottom' || orientation === 'horizontal') {
    return (
      <TooltipProvider delayDuration={200}>
        <div
          className={cn('w-full flex items-center justify-center', className)}
          style={{
            height: compact ? 44 : 56,
            display: 'flex',
            gap: 8,
            padding: '6px 4px',
            boxSizing: 'border-box',
            background: 'transparent',
            alignItems: 'center',
            justifyContent: 'center',
          }}
          role="toolbar"
          aria-label="Panel activity rail"
        >
          <div style={{ display: 'flex', gap: 8, alignItems: 'center', justifyContent: 'center' }}>
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
                        background: active ? 'var(--color-selected-background)' : 'transparent',
                        border: active ? `1px solid var(--color-border)` : '1px solid transparent',
                        color: active ? 'var(--color-accent)' : 'var(--color-text-secondary)',
                        boxShadow: active ? '0 10px 28px var(--color-accent-glow)' : 'none',
                        cursor: 'pointer',
                        transition: 'all 140ms ease',
                        padding: 6,
                      }}
                    >
                      <Icon name={activity.icon as any} size={16} />
                    </button>
                  </TooltipTrigger>
                  <TooltipContent side="top" className="border bg-panel shadow-lg" />
                </Tooltip>
              );
            })}
          </div>

          {/* compact utilities aligned after a small spacer */}
          {utilityActivities.length > 0 && <div style={{ width: 8 }} />}

          <div style={{ display: 'flex', gap: 6, alignItems: 'center', justifyContent: 'center' }}>
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
                        padding: 6,
                      }}
                    >
                      <Icon name={activity.icon as any} size={14} />
                    </button>
                  </TooltipTrigger>
                  <TooltipContent side="top" className="border bg-panel shadow-lg" />
                </Tooltip>
              );
            })}
          </div>
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
