import { create } from 'zustand';
import { devtools } from 'zustand/middleware';
import { useWorkspaceStore } from '@/features/workspace/stores/useWorkspaceStore';

export type TabKind = 'file' | 'welcome';

export interface Tab {
  id: string;
  title: string;
  isDirty: boolean;
  kind: TabKind;
}

/** The reserved id for the built‑in Welcome tab. */
export const WELCOME_TAB_ID = '__welcome__';

interface TabsState {
  tabs: Tab[];
  activeTabId: string | null;
  openFile: (id: string, title: string, kind?: TabKind) => void;
  closeTab: (id: string) => void;
  setActiveTab: (id: string) => void;
  markDirty: (id: string) => void;
  markClean: (id: string) => void;
}

export const useTabsStore = create<TabsState>()(
  devtools(
    (set, get) => ({
      tabs: [{
        id: WELCOME_TAB_ID,
        title: 'Welcome',
        isDirty: false,
        kind: 'welcome',
      }],
      activeTabId: WELCOME_TAB_ID,

      openFile: (id, title, kind = 'file') => {
        const { tabs } = get();
        const existing = tabs.find((t) => t.id === id);
        if (existing) {
          // Activate the existing tab and synchronize workspace active file.
          set({ activeTabId: id });
          if (kind === 'file') {
            useWorkspaceStore.getState().setActiveFilePath(id);
          } else {
            useWorkspaceStore.getState().setActiveFilePath(null);
          }
          return;
        }
        const newTab: Tab = {
          id,
          title,
          isDirty: false, // tabs are created clean by default
          kind,
        };
        set({
          tabs: [...tabs, newTab],
          activeTabId: id,
        });
        // Ensure workspace explorer activeFilePath follows the active tab for file tabs.
        if (kind === 'file') {
          useWorkspaceStore.getState().setActiveFilePath(id);
        } else {
          useWorkspaceStore.getState().setActiveFilePath(null);
        }
      },

      closeTab: (id) => {
        const { tabs, activeTabId } = get();
        const tab = tabs.find((t) => t.id === id);
        if (!tab) return;
        // completely block closing dirty tabs (no prompts, no close)
        if (tab.isDirty) {
          return;
        }
        const idx = tabs.findIndex((t) => t.id === id);
        if (idx === -1) return;

        const newTabs = tabs.filter((t) => t.id !== id);
        let newActive = activeTabId;
        if (activeTabId === id) {
          if (idx < newTabs.length) {
            newActive = newTabs[idx].id;
          } else if (newTabs.length > 0) {
            newActive = newTabs[newTabs.length - 1].id;
          } else {
            newActive = null;
          }
        }
        set({ tabs: newTabs, activeTabId: newActive });

        // Keep workspace explorer activeFilePath in sync with the newly active tab.
        if (newActive) {
          // If the new active tab is a file tab, set the explorer active path; otherwise clear it.
          const newTabObj = newTabs.find((t) => t.id === newActive);
          if (newTabObj?.kind === 'file') {
            useWorkspaceStore.getState().setActiveFilePath(newActive);
          } else {
            useWorkspaceStore.getState().setActiveFilePath(null);
          }
        } else {
          useWorkspaceStore.getState().setActiveFilePath(null);
        }

        // If the last tab was closed, re‑open the Welcome tab automatically.
        if (newTabs.length === 0) {
          get().openFile(WELCOME_TAB_ID, 'Welcome', 'welcome');
        }
      },

      setActiveTab: (id) => {
        const { tabs } = get();
        const tab = tabs.find((t) => t.id === id);
        set({ activeTabId: id });
        if (tab?.kind === 'file') {
          useWorkspaceStore.getState().setActiveFilePath(id);
        } else {
          useWorkspaceStore.getState().setActiveFilePath(null);
        }
      },

      markDirty: (id) => {
        set((state) => ({
          tabs: state.tabs.map((t) =>
            t.id === id ? { ...t, isDirty: true } : t
          ),
        }));
      },

      markClean: (id) => {
        set((state) => ({
          tabs: state.tabs.map((t) =>
            t.id === id ? { ...t, isDirty: false } : t
          ),
        }));
      },
    }),
    { name: 'tabs-store' }
  )
);
