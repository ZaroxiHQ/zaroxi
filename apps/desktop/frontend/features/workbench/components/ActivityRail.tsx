import { Icon } from '@/components/ui/Icon';
import { cn } from '@/lib/utils';
import { useWorkbenchStore } from '../store/workbenchStore';
import { getAvailableActivities } from '../config/activityRegistry';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/Tooltip';
import { LAYOUT } from '../config/layoutConstants';

/**
 * ActivityRail (bottom) - horizontal bottom bar variant
 *
 * This component renders the same activity items but in a horizontal bottom
 * bar instead of a vertical left rail. It accepts an optional `className`
 * for layout integration in AppShell.
 *
 * Visual rules:
 * - Slim horizontal bar (~56px) spanning full app width.
 * - Calm brand area at left, primary actions centered, utility actions on right.
 * - Uses theme CSS variables only (no hardcoded hex).
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

  // Primary actions are those not explicitly marked position === 'bottom' in the registry.
  // Utility actions (position === 'bottom') will be aligned to the right side of the bar.
  const primaryActivities = activities.filter(a => a.position !== 'bottom');
  const utilityActivities = activities.filter(a => a.position === 'bottom');

  return (
    <TooltipProvider delayDuration={240}>
      <footer
        className={cn('w-full flex items-center px-4', className)}
        style={{
          height: 56,
          backgroundColor: 'var(--color-activity-rail-background)',
          borderTop: '1px solid var(--color-divider-subtle)',
          gap: 12,
          alignItems: 'center',
        }}
        role="toolbar"
        aria-label="Activity Bar"
      >
        {/* Left: calm brand mark */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, minWidth: 180 }}>
          <div
            style={{
              width: 36,
              height: 36,
              borderRadius: 10,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              background: 'transparent',
              border: '1px solid transparent',
              color: 'var(--color-text-secondary)',
            }}
            aria-hidden
          >
            <Icon name="sparkles" size={16} />
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', lineHeight: 1 }}>
            <div style={{ fontSize: 13, fontWeight: 600, color: 'var(--color-text-primary)' }}>Zaroxi Studio</div>
          </div>
        </div>

        {/* Center: primary actions (centered group) */}
        <div style={{ display: 'flex', flex: 1, justifyContent: 'center', gap: 10 }}>
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
                      width: 44,
                      height: 44,
                      display: 'inline-flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      borderRadius: 10,
                      border: isActive ? `1px solid var(--color-border)` : '1px solid transparent',
                      background: isActive ? 'linear-gradient(180deg, rgba(108,99,255,0.06), rgba(82,70,229,0.03))' : 'transparent',
                      color: isActive ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
                      boxShadow: isActive ? '0 8px 24px var(--color-accent-glow)' : 'none',
                      transition: 'all 140ms ease',
                      cursor: 'pointer',
                      padding: 6,
                    }}
                    data-no-drag="true"
                  >
                    <Icon name={activity.icon as any} size={16} />
                  </button>
                </TooltipTrigger>
                <TooltipContent side="top" className="border bg-panel shadow-lg" />
              </Tooltip>
            );
          })}
        </div>

        {/* Right: utility actions (aligned to the right) */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 180, justifyContent: 'flex-end' }}>
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
                      width: 40,
                      height: 40,
                      display: 'inline-flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      borderRadius: 8,
                      border: isActive ? `1px solid var(--color-border)` : '1px solid transparent',
                      background: isActive ? 'var(--color-accent)' : 'transparent',
                      color: isActive ? 'var(--color-text-on-accent)' : 'var(--color-text-secondary)',
                      boxShadow: isActive ? '0 6px 18px var(--color-accent-glow)' : 'none',
                      transition: 'all 140ms ease',
                      cursor: 'pointer',
                      padding: 6,
                    }}
                    data-no-drag="true"
                  >
                    <Icon name={activity.icon as any} size={14} />
                  </button>
                </TooltipTrigger>
                <TooltipContent side="top" className="border bg-panel shadow-lg" />
              </Tooltip>
            );
          })}

          {/* Optional small avatar / workspace badge */}
          <div style={{ width: 36, height: 36, borderRadius: 10, display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'transparent', border: '1px solid transparent', color: 'var(--color-text-secondary)' }}>
            <Icon name="star" size={14} />
          </div>
        </div>
      </footer>
    </TooltipProvider>
  );
}
