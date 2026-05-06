import { ReactNode, useEffect } from 'react';
import { ErrorBoundary } from './ErrorBoundary';
import { TauriProvider } from './TauriProvider';
import { KeyboardShortcutsProvider } from '@/lib/keyboard/KeyboardShortcutsProvider';
import { FontLoader } from './FontLoader';
import { ThemeProvider } from '@/lib/theme/ThemeProvider';
import { useWorkbenchStore } from '@/features/workbench/store/workbenchStore';

// Check if we're running in Tauri
const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;

interface AppProviderProps {
  children: ReactNode;
}

function AppProviderContent({ children }: AppProviderProps) {
  const { activateLeftPanel } = useWorkbenchStore();

  useEffect(() => {
    // Listen for open-settings event from menu (Tauri only)
    let unlistenSettings: Promise<() => void> | undefined;
    let unlistenWorkspace: Promise<() => void> | undefined;
    
    if (isTauri) {
      import('@tauri-apps/api/event').then(({ listen }) => {
        unlistenSettings = listen('open-settings', () => {
          activateLeftPanel('settings');
        });
        
        unlistenWorkspace = listen('menu:open-workspace', () => {
          // Trigger the open workspace dialog
          import('@tauri-apps/api/core').then(({ invoke }) => {
            invoke('open_file_dialog').catch(console.error);
          });
        });
      });
    }

    // Also listen for custom event from toolbar (works in both environments)
    const handleOpenSettings = () => activateLeftPanel('settings');
    window.addEventListener('open-settings', handleOpenSettings as EventListener);

    return () => {
      if (unlistenSettings) {
        unlistenSettings.then(f => f()).catch(console.error);
      }
      if (unlistenWorkspace) {
        unlistenWorkspace.then(f => f()).catch(console.error);
      }
      window.removeEventListener('open-settings', handleOpenSettings as EventListener);
    };
  }, [activateLeftPanel]);

  return (
    <div className="font-sans antialiased text-primary h-screen flex flex-col">
      <div className="flex-1 overflow-hidden">
        {children}
      </div>
    </div>
  );
}

export function AppProvider({ children }: AppProviderProps) {
  return (
    <ErrorBoundary>
      <TauriProvider>
        <KeyboardShortcutsProvider>
          <ThemeProvider>
            <FontLoader />
            <AppProviderContent children={children} />
          </ThemeProvider>
        </KeyboardShortcutsProvider>
      </TauriProvider>
    </ErrorBoundary>
  );
}
