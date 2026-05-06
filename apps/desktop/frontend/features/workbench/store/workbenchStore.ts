import { create } from 'zustand';
import { devtools } from 'zustand/middleware';

export type PanelId = 'explorer' | 'search' | 'git' | 'debug' | 'assistant' | 'settings' | 'extensions';

export interface PanelState {
  // Active left panel ID
  activeLeftPanel: PanelId | null;
  // Active right panel ID (for assistant)
  activeRightPanel: PanelId | null;
  // Whether the left panel is visible
  isLeftPanelVisible: boolean;
  // Whether the right panel is visible
  isRightPanelVisible: boolean;
  // Panel widths
  leftPanelWidth: number;
  rightPanelWidth: number;
  // Top bar visibility (always visible in new design)
  isTopBarVisible: boolean;
}

export interface PanelActions {
  // Activate a panel on the left side
  activateLeftPanel: (panelId: PanelId) => void;
  // Activate a panel on the right side
  activateRightPanel: (panelId: PanelId) => void;
  // Toggle left panel visibility
  toggleLeftPanelVisibility: () => void;
  // Toggle right panel visibility
  toggleRightPanelVisibility: () => void;
  // Close left panel
  closeLeftPanel: () => void;
  // Close right panel
  closeRightPanel: () => void;
  // Set panel widths
  setLeftPanelWidth: (width: number) => void;
  setRightPanelWidth: (width: number) => void;
  // Toggle a specific panel
  togglePanel: (panelId: PanelId) => void;
}

const DEFAULT_PANEL_WIDTH = 240;
const DEFAULT_RIGHT_PANEL_WIDTH = 320;

export const useWorkbenchStore = create<PanelState & PanelActions>()(
  devtools(
    (set, get) => ({
      activeLeftPanel: 'explorer',
      activeRightPanel: null,
      isLeftPanelVisible: true,
      isRightPanelVisible: false,
      leftPanelWidth: DEFAULT_PANEL_WIDTH,
      rightPanelWidth: DEFAULT_RIGHT_PANEL_WIDTH,
      isTopBarVisible: true,
      // Where to render the activity rail:
      // - 'panel' = activity rail lives inside each panel (sticky bottom area)
      // - 'edge' = activity rail sits on the very edge (collapsed dock)
      activityRailDock: 'panel',

      activateLeftPanel: (panelId) => {
        const { activeLeftPanel, isLeftPanelVisible } = get();
        if (activeLeftPanel === panelId && isLeftPanelVisible) {
          // Already active and visible, do nothing
          return;
        }
        set({
          activeLeftPanel: panelId,
          isLeftPanelVisible: true,
        });
      },

      activateRightPanel: (panelId) => {
        const { activeRightPanel, isRightPanelVisible } = get();
        if (activeRightPanel === panelId && isRightPanelVisible) {
          // Already active and visible, close it
          set({
            isRightPanelVisible: false,
            activeRightPanel: null,
          });
        } else {
          // Activate this panel on the right
          set({
            activeRightPanel: panelId,
            isRightPanelVisible: true,
          });
        }
      },

      toggleLeftPanelVisibility: () => {
        set((state) => ({
          isLeftPanelVisible: !state.isLeftPanelVisible,
        }));
      },

      toggleRightPanelVisibility: () => {
        set((state) => ({
          isRightPanelVisible: !state.isRightPanelVisible,
        }));
      },

      closeLeftPanel: () => {
        set({
          isLeftPanelVisible: false,
          activeLeftPanel: null,
        });
      },

      closeRightPanel: () => {
        set({
          isRightPanelVisible: false,
          activeRightPanel: null,
        });
      },

      setLeftPanelWidth: (width) => {
        set({ leftPanelWidth: Math.max(200, Math.min(600, width)) });
      },

      setRightPanelWidth: (width) => {
        set({ rightPanelWidth: Math.max(200, Math.min(600, width)) });
      },

      // Set where the activity rail docks ('panel' | 'edge')
      setActivityRailDock: (dock) => {
        set({ activityRailDock: dock });
      },

      // Toggle between 'panel' and 'edge' docking
      toggleActivityRailDock: () => {
        set((state) => ({ activityRailDock: state.activityRailDock === 'panel' ? 'edge' : 'panel' }));
      },

      togglePanel: (panelId) => {
        const { activeLeftPanel, isLeftPanelVisible, activeRightPanel, isRightPanelVisible } = get();
        
        // Check if it's a right-side panel (assistant)
        if (panelId === 'assistant') {
          if (activeRightPanel === panelId && isRightPanelVisible) {
            // Toggle off
            set({ isRightPanelVisible: false });
          } else {
            // Activate on right side
            set({
              activeRightPanel: panelId,
              isRightPanelVisible: true,
            });
          }
        } else {
          // Left-side panel (including settings, extensions, etc.)
          if (activeLeftPanel === panelId && isLeftPanelVisible) {
            // Toggle off
            set({ isLeftPanelVisible: false });
          } else {
            // Activate on left side
            set({
              activeLeftPanel: panelId,
              isLeftPanelVisible: true,
            });
          }
        }
      },
    }),
    {
      name: 'workbench-store',
    }
  )
);
