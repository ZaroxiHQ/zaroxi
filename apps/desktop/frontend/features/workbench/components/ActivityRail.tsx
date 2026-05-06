import { Icon } from '@/components/ui/Icon';
import { cn } from '@/lib/utils';
import { useWorkbenchStore } from '../store/workbenchStore';
import { getAvailableActivities } from '../config/activityRegistry';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/Tooltip';
import { LAYOUT } from '../config/layoutConstants';

/**
 * ActivityRail - tailored to the mockup:
 * - Calm top region (logo / subtle separator)
 * - Primary action icons grouped at the bottom
 * - Compact spacing and premium active treatment
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

  // Put all primary activities into the bottom group to match the mockup.
  const activities = getAvailableActivities();
  const primaryActivities = activities.filter(a => a.side === 'left' && a.position !== 'bottom');
  const utilityActivities = activities.filter(a => a.position === 'bottom');

  return (
    <TooltipProvider delayDuration={300}>
      <aside
        className={cn('flex flex-col items-stretch py-3', className)}
        style={{
          width: LAYOUT.activityRailWidth,
          backgroundColor: 'var(--color-activity-rail-background)',
          borderRight: '1px solid var(--color-divider-subtle)',
          paddingLeft: 6,
          paddingRight: 6,
        }}
        aria-label="Activity"
      >
        {/* Top: calm zone (logo / small spacer) */}
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: 48 }}>
          <div style={{
            width: 34,
            height: 34,
            borderRadius: 8,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            border: '1px solid transparent',
            color: 'var(--color-text-secondary)'
          }}>
            <Icon name="sparkles" size={14} />
          </div>
        </div>

        {/* Flexible spacer - pushes primary actions to bottom */}
        <div style={{ flex: 1 }} />

        {/* Bottom grouped primary actions - compact and premium */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8, alignItems: 'center', paddingBottom: 10 }}>
          {primaryActivities.map((activity) => {
            const isActive = activity.side === 'left'
              ? activeLeftPanel === activity.id && isLeftPanelVisible
              : activeRightPanel === activity.id && isRightPanelVisible;

            return (
              <Tooltip key={activity.id}>
                <TooltipTrigger asChild>
                  <button
                    onClick={() => togglePanel(activity.id)}
                    aria-label={activity.label}
                    style={{
                      width: 42,
                      height: 42,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      borderRadius: 8,
                      border: isActive ? `1px solid var(--color-border)` : '1px solid transparent',
                      background: isActive ? 'linear-gradient(180deg, rgba(108,99,255,0.08), rgba(82,70,229,0.04))' : 'transparent',
                      color: isActive ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
                      boxShadow: isActive ? '0 10px 24px var(--color-accent-glow)' : 'none',
                      transition: 'all 160ms ease',
                      cursor: 'pointer',
                    }}
                  >
                    <Icon name={activity.icon as any} size={16} />
                  </button>
                </TooltipTrigger>
                <TooltipContent side="right" className="border bg-panel shadow-lg" />
              </Tooltip>
            );
          })}

          {/* Small separator between primary actions and utilities */}
          <div style={{ height: 8 }} />

          {/* Utility / bottom actions (settings, account, etc.) */}
          {utilityActivities.map((activity) => {
            const isActive = activity.side === 'left'
              ? activeLeftPanel === activity.id && isLeftPanelVisible
              : activeRightPanel === activity.id && isRightPanelVisible;

            return (
              <Tooltip key={activity.id}>
                <TooltipTrigger asChild>
                  <button
                    onClick={() => togglePanel(activity.id)}
                    aria-label={activity.label}
                    style={{
                      width: 42,
                      height: 42,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      borderRadius: 8,
                      border: isActive ? `1px solid var(--color-border)` : '1px solid transparent',
                      background: isActive ? 'var(--color-accent)' : 'transparent',
                      color: isActive ? 'var(--color-text-on-accent)' : 'var(--color-text-secondary)',
                      boxShadow: isActive ? '0 8px 20px var(--color-accent-glow)' : 'none',
                      transition: 'all 160ms ease',
                      cursor: 'pointer',
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
      </aside>
    </TooltipProvider>
  );
}
