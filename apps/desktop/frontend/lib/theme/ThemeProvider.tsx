import { ReactNode, useEffect, useState } from 'react';
import { useThemeStore, initializeTheme } from './theme-store';

interface ThemeProviderProps {
  children: ReactNode;
}

/**
 * ThemeProvider — lightweight adapter.
 *
 * Responsibilities:
 * - Ensure runtime theme initialization runs (initializeTheme)
 * - Prevent FOUC by delaying children until the first animation frame
 * - Do NOT duplicate or re-map semantic tokens here: the centralized
 *   theme-store is the single place that writes --color-* CSS variables.
 */
export function ThemeProvider({ children }: ThemeProviderProps) {
  const { themeData } = useThemeStore();
  const [themeReady, setThemeReady] = useState(false);

  useEffect(() => {
    // Initialize theme logic (loads from backend, sets up listeners).
    const cleanup = initializeTheme();

    // Wait a single frame to avoid layout flash; store already writes CSS vars.
    const id = requestAnimationFrame(() => setThemeReady(true));

    return () => {
      cancelAnimationFrame(id);
      cleanup();
    };
  }, []);

  // Prevent rendering of children until we have painted once with theme CSS vars.
  if (!themeReady) return null;

  return <div className="min-h-screen w-full h-full">{children}</div>;
}
