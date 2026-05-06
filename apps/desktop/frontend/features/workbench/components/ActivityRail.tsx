import { Icon } from '@/components/ui/Icon';
import { cn } from '@/lib/utils';
import { useWorkbenchStore } from '../store/workbenchStore';
import { getAvailableActivities } from '../config/activityRegistry';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/Tooltip';
import { LAYOUT } from '../config/layoutConstants';

/**
 * ActivityRail - slim, integrated rail styled to match the mockup:
 * - Uses runtime tokens for colors
 * - Subtle rounded buttons with soft accent glow for active state
 * - Top and bottom sections, centered icons, compact density
 */
interface ActivityRailProps {
  className?: string;
}

export function ActivityRail({ className }: ActivityRailProps) {
  const {
    activeLeftPanel,
    activeRightPanel,
    isLeftPanelVisible,
    isRightPanelVisible,
    togglePanel,
  } = useWorkbenchStore();

  const activities = getAvailableActivities();
  const topActivities = activities.filter((a) => a.position !== 'bottom');
  const bottomActivities = activities.filter((a) => a.position === 'bottom');

  return (
    <TooltipProvider delayDuration={300}>
      <aside
        className={cn('flex flex-col items-center py-4', className)}
        style={{
          width: LAYOUT.activityRailWidth,
          backgroundColor: 'var(--color-activity-rail-background)',
          borderRight: '1px solid var(--color-divider-subtle)',
          gap: 8,
        }}
        aria-label="Activity"
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10, alignItems: 'center' }}>
          {topActivities.map((activity) => {
            const isActive =
              activity.side === 'left'
                ? activeLeftPanel === activity.id && isLeftPanelVisible
                : activeRightPanel === activity.id && isRightPanelVisible;

            return (
              <Tooltip key={activity.id}>
                <TooltipTrigger asChild>
                  <button
                    onClick={() => togglePanel(activity.id)}
                    aria-label={activity.label}
                    style={{
                      width: 40,
                      height: 40,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      borderRadius: 8,
                      border: '1px solid transparent',
                      background: isActive ? 'var(--color-accent)' : 'transparent',
                      color: isActive ? 'var(--color-text-on-accent)' : 'var(--color-text-secondary)',
                      boxShadow: isActive ? '0 8px 28px var(--color-accent-soft)' : 'none',
                      transition: 'all 140ms ease',
                      cursor: 'pointer',
                    }}
                  >
                    <Icon name={activity.icon} size={16} />
                    {activity.badge !== undefined && activity.badge > 0 && (
                      <span
                        style={{
                          position: 'absolute',
                          top: -6,
                          right: -6,
                          width: 18,
                          height: 18,
                          borderRadius: 18,
                          backgroundColor: 'var(--color-error)',
                          color: 'white',
                          fontSize: 10,
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'center',
                          border: '1px solid var(--color-activity-rail-background)',
                        }}
                      >
                        {activity.badge > 9 ? '9+' : activity.badge}
                      </span>
                    )}
                  </button>
                </TooltipTrigger>
                <TooltipContent side="right" className="border bg-panel shadow-lg" />
              </Tooltip>
            );
          })}
        </div>

        <div style={{ flex: 1 }} />

        <div style={{ display: 'flex', flexDirection: 'column', gap: 10, alignItems: 'center' }}>
          {bottomActivities.map((activity) => {
            const isActive =
              activity.side === 'left'
                ? activeLeftPanel === activity.id && isLeftPanelVisible
                : activeRightPanel === activity.id && isRightPanelVisible;

            return (
              <Tooltip key={activity.id}>
                <TooltipTrigger asChild>
                  <button
                    onClick={() => togglePanel(activity.id)}
                    aria-label={activity.label}
                    style={{
                      width: 40,
                      height: 40,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      borderRadius: 8,
                      border: '1px solid transparent',
                      background: isActive ? 'var(--color-accent)' : 'transparent',
                      color: isActive ? 'var(--color-text-on-accent)' : 'var(--color-text-secondary)',
                      boxShadow: isActive ? '0 8px 28px var(--color-accent-soft)' : 'none',
                      transition: 'all 140ms ease',
                      cursor: 'pointer',
                    }}
                  >
                    <Icon name={activity.icon} size={16} />
                  </button>
                </TooltipTrigger>
                <TooltipContent side="right" className="border bg-panel shadow-lg" />
              </Tooltip>
            );
          })}
        </div>
      </aside>
    </TooltipProvider>
  );
}
