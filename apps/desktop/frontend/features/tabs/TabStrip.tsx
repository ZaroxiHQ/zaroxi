import { useTabsStore } from './store';
import { TabItem } from './TabItem';
import { LAYOUT } from '@/features/workbench/config/layoutConstants';

export function TabStrip() {
  const { tabs, activeTabId } = useTabsStore();

  if (tabs.length === 0) {
    return null;
  }

  return (
    <div
      className="flex items-center overflow-x-auto overflow-y-hidden bg-activity-rail text-activity-rail-foreground"
      style={{
        height: LAYOUT.topBarHeight,
        minHeight: LAYOUT.topBarHeight,
        alignItems: 'center',
        scrollbarWidth: 'none',
        msOverflowStyle: 'none',
        borderBottom: '0.5px solid var(--color-divider-subtle)',
        paddingLeft: 6,
        paddingRight: 6,
        minWidth: 0,
      }}
      data-no-drag="true"
    >
      <div style={{ display: 'flex', gap: 6, alignItems: 'center', minWidth: 0 }}>
        {tabs.map((tab) => (
          <TabItem key={tab.id} tab={tab} isActive={tab.id === activeTabId} />
        ))}
        {/* small right‑side spacer to give a bit of room after the last tab */}
        <div className="flex-shrink-0 w-4" />
      </div>
    </div>
  );
}
