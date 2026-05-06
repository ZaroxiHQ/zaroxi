import { Icon } from '@/components/ui/Icon';
import { cn } from '@/lib/utils';
import { useWorkbenchStore } from '../store/workbenchStore';
import { getAvailableActivities } from '../config/activityRegistry';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/Tooltip';
import { LAYOUT } from '../config/layoutConstants';

/**
 * ActivityRail - compact activity rail intended to live inside a side panel.
 *
 * Key behaviors to match the spec:
 * - Slim vertical rail (~48px) that sits inside the side panel edge.
 * - Main action icons grouped in the lower area of the rail (sticky).
 * - When the side panel is collapsed the rail remains visible (collapsed state).
 * - Accepts `orientation` prop (vertical | horizontal) for flexibility (horizontal kept for backward compatibility).
 *
 * Usage:
 * - Render inside PanelHost to keep activity icons scoped to the panel.
 * - For the collapsed dock, render <ActivityRail orientation="vertical" />.
 */
interface ActivityRailProps {
  className?: string;
  orientation?: 'vertical' | 'horizontal';
}

export function ActivityRail({ className, orientation = 'vertical' }: ActivityRailProps) {
  const {
    activeLeftPanel,
    activeRightPanel,
    isLeftPanelVisible,
    isRightPanelVisible,
    togglePanel,
  } = useWorkbenchStore();

  const activities = getAvailableActivities();

  // Primary actions are those not explicitly marked position === 'bottom' in the registry.
  // Utility actions (position === 'bottom') are considered lower-priority helpers.
  const primaryActivities = activities.filter(a => a.position !== 'bottom');
  const utilityActivities = activities.filter(a => a.position === 'bottom');

  // Vertical rail (the new preferred layout) - icons clustered near the bottom
  if (orientation === 'vertical') {
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
          {/* Calm top area for brand or small spacer */}
          <div style={{ height: 36, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--color-text-secondary)' }}>
            <Icon name="sparkles" size={14} />
          </div>

          {/* Flexible spacer so icons sit at the bottom */}
          <div style={{ flex: 1 }} />

          {/* Primary actions - stacked compact */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8, alignItems: 'center' }}>
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
                        boxShadow: isActive ? '0 8px 20px var(--color-accent-glow)' : 'none',
                        transition: 'all 140ms ease',
                        cursor: 'pointer',
                        padding: 6,
                      }}
                      data-no-drag="true"
                    >
                      <Icon name={activity.icon as any} size={16} />
                    </button>
                  </TooltipTrigger>
                  <TooltipContent side="right" className="border bg-panel shadow-lg" />
                </Tooltip>
              );
            })}
          </div>

          {/* Small separator */}
          <div style={{ height: 8 }} />

          {/* Utility icons and avatar */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8, alignItems: 'center' }}>
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
                        width: 38,
                        height: 38,
                        display: 'inline-flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        borderRadius: 9,
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
                  <TooltipContent side="right" className="border bg-panel shadow-lg" />
                </Tooltip>
              );
            })}

            {/* Workspace badge / avatar */}
            <div style={{ width: 36, height: 36, borderRadius: 10, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--color-text-secondary)' }}>
              <Icon name="star" size={14} />
            </div>
          </div>
        </aside>
      </TooltipProvider>
    );
  }

  // Horizontal fallback (keeps backward compatibility)
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
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, minWidth: 180 }}>
          <div style={{ width: 36, height: 36, borderRadius: 10, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--color-text-secondary)' }}>
            <Icon name="sparkles" size={16} />
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', lineHeight: 1 }}>
            <div style={{ fontSize: 13, fontWeight: 600, color: 'var(--color-text-primary)' }}>Zaroxi Studio</div>
          </div>
        </div>

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

          <div style={{ width: 36, height: 36, borderRadius: 10, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--color-text-secondary)' }}>
            <Icon name="star" size={14} />
          </div>
        </div>
      </footer>
    </TooltipProvider>
  );
}
