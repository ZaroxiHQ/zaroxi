import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { ZaroxiTheme, SemanticColors, ThemeData } from './types';

// Check if we're running in Tauri
const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;

// Helper to convert Rust enum to TypeScript string
function toRustTheme(mode: ZaroxiTheme): 'Dark' | 'Light' | 'System' {
  switch (mode) {
    case 'dark': return 'Dark';
    case 'light': return 'Light';
    case 'system': return 'System';
  }
}

function fromRustTheme(mode: 'Dark' | 'Light' | 'System'): ZaroxiTheme {
  switch (mode) {
    case 'Dark': return 'dark';
    case 'Light': return 'light';
    case 'System': return 'system';
  }
}

// Default semantic colors for fallback (dark theme)
function getDefaultColors(isDark: boolean): SemanticColors {
  if (isDark) {
    return {
      app_background: '#1B1D22',
      shell_background: '#1E2025',
      panel_background: '#252931',
      elevated_panel_background: '#2A2E37',
      editor_background: '#1E1F24',
      input_background: '#2A2E37',
      status_bar_background: '#23262D',
      title_bar_background: '#23262D',
      activity_rail_background: '#20232A',
      sidebar_background: '#252931',
      tab_background: '#252830',
      tab_active_background: '#1E1F24',
      assistant_panel_background: '#262A32',
      text_primary: '#E6EAF2',
      text_secondary: '#C8CDD6',
      text_muted: '#AAB2BF',
      text_faint: '#7E8794',
      text_on_accent: '#FFFFFF',
      text_on_surface: '#E6EAF2',
      text_disabled: '#5A6270',
      text_link: '#5B8CFF',
      border: '#343944',
      border_subtle: 'rgba(52,57,68,0.5)',
      divider: '#343944',
      accent: '#5B8CFF',
      accent_hover: '#6B9CFF',
      accent_soft: 'rgba(91,140,255,0.15)',
      accent_soft_background: 'rgba(91,140,255,0.08)',
      hover_background: 'rgba(255,255,255,0.06)',
      active_background: 'rgba(255,255,255,0.10)',
      selected_background: 'rgba(91,140,255,0.18)',
      selected_text_background: 'rgba(91,140,255,0.22)',
      selected_editor_background: 'rgba(91,140,255,0.18)',
      success: '#4CAF50',
      warning: '#FF9800',
      error: '#F44336',
      info: '#5B8CFF',
      focus_ring: 'rgba(91,140,255,0.30)',
      editor_gutter_background: '#1E1F24',
      editor_line_highlight: 'rgba(255,255,255,0.03)',
      editor_cursor: '#E6EAF2',
      editor_selection: 'rgba(91,140,255,0.22)',
      editor_find_highlight: 'rgba(255,96,0,0.25)',

      // Syntax colors (dark defaults mirror crates/zaroxi-theme)
      syntax_keyword: '#FF6B6B',
      syntax_function: '#4CAF50',
      syntax_method: '#4CAF50',
      syntax_string: '#FFB74D',
      syntax_comment: '#7E8794',
      syntax_type: '#5B8CFF',
      syntax_variable: '#E6EAF2',
      syntax_constant: '#FFB74D',
      syntax_number: '#B39DDB',
      syntax_operator: '#C792EA',
      syntax_punctuation: '#AAB2BF',
      syntax_attribute: '#F78C6C',
      syntax_tag: '#F07178',
      syntax_namespace: '#82AAFF',
      syntax_macro: '#C792EA',
      syntax_property: '#FFCB6B',
      syntax_parameter: '#F78C6C',
      syntax_builtin: '#FF5370',
      syntax_escape: '#89DDFF',
      syntax_embedded: '#C3E88D',
      syntax_regex: '#89DDFF',
      syntax_markup_heading: '#FF5370',
      syntax_markup_list: '#89DDFF',
      syntax_markup_quote: '#7E8794',
      syntax_markup_link: '#82AAFF',
      syntax_markup_code: '#C3E88D',
      syntax_markup_bold: '#E6EAF2',
      syntax_markup_italic: '#E6EAF2',
      syntax_markup_strikethrough: '#7E8794',
      syntax_lifetime: '#89DDFF',
    };
  } else {
    return {
      app_background: '#F4F3EF',
      shell_background: '#F0EFEA',
      panel_background: '#F0EEE8',
      elevated_panel_background: '#F8F6F2',
      editor_background: '#FBFAF7',
      input_background: '#FFFFFF',
      status_bar_background: '#ECE9E3',
      title_bar_background: '#ECE9E3',
      activity_rail_background: '#E7E4DD',
      sidebar_background: '#F0EEE8',
      tab_background: '#F1EEE8',
      tab_active_background: '#FBFAF7',
      assistant_panel_background: '#F2F0EA',
      text_primary: '#22262B',
      text_secondary: '#3D434A',
      text_muted: '#616975',
      text_faint: '#8A919D',
      text_on_accent: '#FFFFFF',
      text_on_surface: '#22262B',
      text_disabled: '#B0B6C0',
      text_link: '#426EDB',
      border: '#D7D1C7',
      border_subtle: 'rgba(215,209,199,0.5)',
      divider: '#D7D1C7',
      accent: '#426EDB',
      accent_hover: '#3A62C8',
      accent_soft: 'rgba(66,110,219,0.10)',
      accent_soft_background: 'rgba(66,110,219,0.05)',
      hover_background: 'rgba(0,0,0,0.04)',
      active_background: 'rgba(0,0,0,0.08)',
      selected_background: 'rgba(66,110,219,0.08)',
      selected_text_background: 'rgba(66,110,219,0.14)',
      selected_editor_background: 'rgba(66,110,219,0.08)',
      success: '#2E7D32',
      warning: '#E65100',
      error: '#C62828',
      info: '#426EDB',
      focus_ring: 'rgba(66,110,219,0.25)',
      editor_gutter_background: '#FBFAF7',
      editor_line_highlight: 'rgba(66,110,219,0.03)',
      editor_cursor: '#22262B',
      editor_selection: 'rgba(66,110,219,0.14)',
      editor_find_highlight: 'rgba(230,102,0,0.18)',

      // Syntax colors (light defaults mirror crates/zaroxi-theme)
      syntax_keyword: '#D32F2F',
      syntax_function: '#2E7D32',
      syntax_method: '#2E7D32',
      syntax_string: '#E65100',
      syntax_comment: '#8A919D',
      syntax_type: '#426EDB',
      syntax_variable: '#22262B',
      syntax_constant: '#E65100',
      syntax_number: '#7B1FA2',
      syntax_operator: '#6A1B9A',
      syntax_punctuation: '#616975',
      syntax_attribute: '#E65100',
      syntax_tag: '#C62828',
      syntax_namespace: '#1565C0',
      syntax_macro: '#6A1B9A',
      syntax_property: '#F57F17',
      syntax_parameter: '#E65100',
      syntax_builtin: '#C62828',
      syntax_escape: '#00838F',
      syntax_embedded: '#2E7D32',
      syntax_regex: '#00838F',
      syntax_markup_heading: '#C62828',
      syntax_markup_list: '#00838F',
      syntax_markup_quote: '#8A919D',
      syntax_markup_link: '#1565C0',
      syntax_markup_code: '#2E7D32',
      syntax_markup_bold: '#22262B',
      syntax_markup_italic: '#22262B',
      syntax_markup_strikethrough: '#8A919D',
      syntax_lifetime: '#00838F',
    };
  }
}

interface ThemeStore {
  themeMode: ZaroxiTheme;
  isDark: boolean;
  isSystem: boolean;
  isLoading: boolean;
  themeData: ThemeData | null;
  
  // Actions
  setThemeMode: (mode: ZaroxiTheme) => Promise<void>;
  loadThemeSettings: () => Promise<void>;
  applyTheme: (data: { mode: ZaroxiTheme; isDark: boolean; colors?: SemanticColors }) => void;
}

export const useThemeStore = create<ThemeStore>()(
  persist(
    (set, get) => ({
      themeMode: 'system',
      isDark: true,
      isSystem: true,
      isLoading: false,
      themeData: null,
      
      setThemeMode: async (mode) => {
        set({ isLoading: true });
        try {
          if (isTauri) {
            // In Tauri environment, use the backend
            const { invoke } = await import('@tauri-apps/api/core');
            const rustTheme = toRustTheme(mode);
            await invoke('set_theme', { theme: rustTheme });
          } else {
            // In browser, just update locally
            // Simulate async operation
            await new Promise(resolve => setTimeout(resolve, 50));
          }
          
          // Determine if dark based on mode
          const isSystem = mode === 'system';
          const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
          const isDark = mode === 'dark' || (isSystem && systemPrefersDark);
          
          const colors = getDefaultColors(isDark);
          const themeData: ThemeData = { mode, isDark, colors };
          
          set({ 
            themeMode: mode, 
            isDark,
            isSystem,
            isLoading: false,
            themeData,
          });
          
          // Update CSS variables
          updateCssVariables(isDark);
        } catch (error) {
          console.error('Failed to set theme:', error);
          set({ isLoading: false });
        }
      },
      
      loadThemeSettings: async () => {
        set({ isLoading: true });
        // Add a timeout to ensure loading doesn't hang indefinitely
        const timeoutId = setTimeout(() => {
          set({ isLoading: false });
        }, 2000);
        
        try {
          if (isTauri) {
            // In Tauri environment, load from backend
            const { invoke } = await import('@tauri-apps/api/core');
            // The Rust command returns a ZaroxiTheme enum which will be serialized as a string
            const currentTheme: string = await invoke('get_current_theme');
            const rustTheme = currentTheme as 'Dark' | 'Light' | 'System';
            const tsTheme = fromRustTheme(rustTheme);
            
            const isSystem = tsTheme === 'system';
            const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
            const isDark = tsTheme === 'dark' || (isSystem && systemPrefersDark);
            
            const colors = getDefaultColors(isDark);
            const themeData: ThemeData = { mode: tsTheme, isDark, colors };
            
            clearTimeout(timeoutId);
            set({ 
              themeMode: tsTheme,
              isDark,
              isSystem,
              isLoading: false,
              themeData,
            });
            
            updateCssVariables(isDark);
          } else {
            // In browser, load from localStorage (handled by persist middleware)
            // Also check system preference
            const savedState = get();
            const isSystem = savedState.themeMode === 'system';
            const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
            const isDark = savedState.themeMode === 'dark' || (isSystem && systemPrefersDark);
            
            const colors = getDefaultColors(isDark);
            const themeData: ThemeData = { mode: savedState.themeMode, isDark, colors };
            
            clearTimeout(timeoutId);
            set({ 
              themeMode: savedState.themeMode,
              isDark,
              isSystem,
              isLoading: false,
              themeData,
            });
            
            updateCssVariables(isDark);
          }
        } catch (error) {
          console.error('Failed to load theme settings:', error);
          clearTimeout(timeoutId);
          // On error, use system preference
          const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
          const isDark = systemPrefersDark;
          const colors = getDefaultColors(isDark);
          const themeData: ThemeData = { mode: 'system', isDark, colors };
          set({ 
            themeMode: 'system',
            isDark,
            isSystem: true,
            isLoading: false,
            themeData,
          });
          updateCssVariables(isDark);
        }
      },
      
      applyTheme: (data) => {
        const isSystem = data.mode === 'system';
        const colors = data.colors || getDefaultColors(data.isDark);
        const themeData: ThemeData = { mode: data.mode, isDark: data.isDark, colors };
        set({
          themeMode: data.mode,
          isDark: data.isDark,
          isSystem,
          themeData,
        });
        updateCssVariables(data.isDark);
      },
    }),
    {
      name: 'zaroxi-theme-storage',
      partialize: (state) => ({
        themeMode: state.themeMode,
        isDark: state.isDark,
      }),
    }
  )
);

// Listen to theme changes from backend (Tauri only)
async function setupThemeListener() {
  if (!isTauri) return () => {};
  
  try {
    const { listen } = await import('@tauri-apps/api/event');
    return listen<{ mode: string; isDark: boolean; colors?: Record<string, string> }>('theme:changed', (event) => {
      // The mode comes as a string from Rust serialization
      const rustMode = event.payload.mode as 'Dark' | 'Light' | 'System';
      const tsMode = fromRustTheme(rustMode);
      
      // If the backend sent full palette data, use it
      if (event.payload.colors) {
        const colors = event.payload.colors as unknown as SemanticColors;
        useThemeStore.getState().applyTheme({ 
          mode: tsMode, 
          isDark: event.payload.isDark,
          colors,
        });
      } else {
        // Fallback to just mode and isDark
        useThemeStore.getState().applyTheme({ 
          mode: tsMode, 
          isDark: event.payload.isDark 
        });
      }
    });
  } catch (error) {
    console.error('Failed to setup theme listener:', error);
    return () => {};
  }
}

// Apply semantic color tokens as CSS custom properties on the document root.
function applyThemeColors(colors: SemanticColors) {
  const root = document.documentElement;
  
  // Background surfaces
  root.style.setProperty('--color-app-background', colors.app_background);
  root.style.setProperty('--color-shell-background', colors.shell_background);
  root.style.setProperty('--color-panel-background', colors.panel_background);
  root.style.setProperty('--color-elevated-panel-background', colors.elevated_panel_background);
  root.style.setProperty('--color-editor-background', colors.editor_background);
  root.style.setProperty('--color-input-background', colors.input_background);
  root.style.setProperty('--color-status-bar-background', colors.status_bar_background);
  root.style.setProperty('--color-title-bar-background', colors.title_bar_background);
  root.style.setProperty('--color-activity-rail-background', colors.activity_rail_background);
  root.style.setProperty('--color-sidebar-background', colors.sidebar_background);
  root.style.setProperty('--color-tab-background', colors.tab_background);
  root.style.setProperty('--color-tab-active-background', colors.tab_active_background);
  root.style.setProperty('--color-assistant-panel-background', colors.assistant_panel_background);
  
  // Text colors
  root.style.setProperty('--color-text-primary', colors.text_primary);
  root.style.setProperty('--color-text-secondary', colors.text_secondary);
  root.style.setProperty('--color-text-muted', colors.text_muted);
  root.style.setProperty('--color-text-faint', colors.text_faint);
  root.style.setProperty('--color-text-on-accent', colors.text_on_accent);
  root.style.setProperty('--color-text-on-surface', colors.text_on_surface);
  root.style.setProperty('--color-text-disabled', colors.text_disabled);
  root.style.setProperty('--color-text-link', colors.text_link);
  
  // UI elements
  root.style.setProperty('--color-border', colors.border);
  root.style.setProperty('--color-border-subtle', colors.border_subtle);
  root.style.setProperty('--color-divider', colors.divider);
  root.style.setProperty('--color-accent', colors.accent);
  root.style.setProperty('--color-accent-hover', colors.accent_hover);
  root.style.setProperty('--color-accent-soft', colors.accent_soft);
  root.style.setProperty('--color-accent-soft-background', colors.accent_soft_background);
  
  // States
  root.style.setProperty('--color-hover-background', colors.hover_background);
  root.style.setProperty('--color-active-background', colors.active_background);
  root.style.setProperty('--color-selected-background', colors.selected_background);
  root.style.setProperty('--color-selected-text-background', colors.selected_text_background);
  root.style.setProperty('--color-selected-editor-background', colors.selected_editor_background);
  
  // Status colors
  root.style.setProperty('--color-success', colors.success);
  root.style.setProperty('--color-warning', colors.warning);
  root.style.setProperty('--color-error', colors.error);
  root.style.setProperty('--color-info', colors.info);
  
  // Focus
  root.style.setProperty('--color-focus-ring', colors.focus_ring);
  
  // Editor specific
  root.style.setProperty('--color-editor-gutter-background', colors.editor_gutter_background);
  root.style.setProperty('--color-editor-line-highlight', colors.editor_line_highlight);
  root.style.setProperty('--color-editor-cursor', colors.editor_cursor);
  root.style.setProperty('--color-editor-selection', colors.editor_selection);
  root.style.setProperty('--color-editor-find-highlight', colors.editor_find_highlight);

  // Editor font family and size — prefer JetBrains Mono Nerd Font if available.
  root.style.setProperty('--editor-font-family', '"JetBrains Mono Nerd Font", "JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, Monaco, "Roboto Mono", "Segoe UI Mono", "Courier New", monospace');
  root.style.setProperty('--editor-font-size', '15px');
  
  // Syntax colors (mirror the full set emitted by the Rust theme crate).
  // THEME-FIRST: ensure every semantic role that the backend provides is present
  // as a CSS custom property so HighlightStyle can consume them without hardcoded colors.
  root.style.setProperty('--color-syntax-keyword', colors.syntax_keyword);
  root.style.setProperty('--color-syntax-function', colors.syntax_function);
  root.style.setProperty('--color-syntax-method', (colors as any).syntax_method ?? colors.syntax_function);
  root.style.setProperty('--color-syntax-string', colors.syntax_string);
  root.style.setProperty('--color-syntax-comment', colors.syntax_comment);
  root.style.setProperty('--color-syntax-type', colors.syntax_type);
  root.style.setProperty('--color-syntax-variable', colors.syntax_variable);
  root.style.setProperty('--color-syntax-constant', colors.syntax_constant);
  root.style.setProperty('--color-syntax-number', (colors as any).syntax_number ?? colors.syntax_constant);
  root.style.setProperty('--color-syntax-operator', (colors as any).syntax_operator ?? colors.syntax_punctuation ?? colors.syntax_variable);
  root.style.setProperty('--color-syntax-punctuation', (colors as any).syntax_punctuation ?? colors.syntax_variable);
  root.style.setProperty('--color-syntax-attribute', (colors as any).syntax_attribute ?? colors.syntax_variable);
  root.style.setProperty('--color-syntax-tag', (colors as any).syntax_tag ?? colors.syntax_variable);
  root.style.setProperty('--color-syntax-namespace', (colors as any).syntax_namespace ?? colors.syntax_type);
  root.style.setProperty('--color-syntax-macro', (colors as any).syntax_macro ?? colors.syntax_operator);
  root.style.setProperty('--color-syntax-property', (colors as any).syntax_property ?? colors.syntax_variable);
  root.style.setProperty('--color-syntax-parameter', (colors as any).syntax_parameter ?? colors.syntax_variable);
  root.style.setProperty('--color-syntax-builtin', (colors as any).syntax_builtin ?? colors.syntax_variable);
  root.style.setProperty('--color-syntax-escape', (colors as any).syntax_escape ?? colors.syntax_string);
  root.style.setProperty('--color-syntax-embedded', (colors as any).syntax_embedded ?? colors.syntax_string);
  root.style.setProperty('--color-syntax-regex', (colors as any).syntax_regex ?? colors.syntax_escape);
  root.style.setProperty('--color-syntax-markup-heading', (colors as any).syntax_markup_heading ?? colors.syntax_tag);
  root.style.setProperty('--color-syntax-markup-list', (colors as any).syntax_markup_list ?? colors.syntax_regex);
  root.style.setProperty('--color-syntax-markup-quote', (colors as any).syntax_markup_quote ?? colors.syntax_comment);
  root.style.setProperty('--color-syntax-markup-link', (colors as any).syntax_markup_link ?? colors.syntax_namespace);
  root.style.setProperty('--color-syntax-markup-code', (colors as any).syntax_markup_code ?? colors.syntax_embedded);
  root.style.setProperty('--color-syntax-markup-bold', (colors as any).syntax_markup_bold ?? colors.text_on_surface);
  root.style.setProperty('--color-syntax-markup-italic', (colors as any).syntax_markup_italic ?? colors.text_on_surface);
  root.style.setProperty('--color-syntax-markup-strikethrough', (colors as any).syntax_markup_strikethrough ?? colors.text_faint);
  root.style.setProperty('--color-syntax-lifetime', (colors as any).syntax_lifetime ?? (colors as any).syntax_escape ?? colors.syntax_variable);
}

// Update CSS custom properties based on theme
let currentTheme: 'light' | 'dark' | null = null;

function updateCssVariables(isDark: boolean) {
  const root = document.documentElement;
  
  // Prevent unnecessary updates
  const newTheme = isDark ? 'dark' : 'light';
  if (currentTheme === newTheme) {
    return;
  }
  
  currentTheme = newTheme;
  console.log(`Setting theme to ${newTheme}`);
  
  if (isDark) {
    root.classList.add('dark');
    root.classList.remove('light');
  } else {
    root.classList.add('light');
    root.classList.remove('dark');
  }
  
  // Set data attribute for CSS selectors
  root.setAttribute('data-theme', newTheme);
  
  // Apply the actual color values as CSS custom properties
  const colors = getDefaultColors(isDark);
  applyThemeColors(colors);
}

// Apply theme immediately when this module loads (before any React code runs)
function applyThemeImmediately() {
  try {
    const saved = localStorage.getItem('zaroxi-theme-storage');
    let isDark = false;
    let themeMode: 'dark' | 'light' | 'system' = 'system';
    
    if (saved) {
      const parsed = JSON.parse(saved);
      if (parsed.state && parsed.state.themeMode) {
        themeMode = parsed.state.themeMode;
        const isSystem = themeMode === 'system';
        const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        isDark = themeMode === 'dark' || (isSystem && systemPrefersDark);
      } else {
        // If no saved theme, use system preference
        const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        isDark = systemPrefersDark;
        themeMode = 'system';
      }
    } else {
      // If no saved data, use system preference
      const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      isDark = systemPrefersDark;
      themeMode = 'system';
    }
    
    // Update CSS variables immediately
    const root = document.documentElement;
    if (isDark) {
      root.classList.add('dark');
      root.classList.remove('light');
    } else {
      root.classList.add('light');
      root.classList.remove('dark');
    }
    root.setAttribute('data-theme', isDark ? 'dark' : 'light');
    
    const colors = getDefaultColors(isDark);
    const themeData: ThemeData = { mode: themeMode, isDark, colors };
    
    // Apply the actual color values as CSS custom properties immediately
    applyThemeColors(colors);
    
    // Also update the store state
    useThemeStore.setState({
      themeMode,
      isDark,
      isSystem: themeMode === 'system',
      isLoading: false, // Don't load from backend if we already have the theme
      themeData,
    });
    
    // Store the applied theme to prevent unnecessary backend loading
    (window as any).__zaroxi_theme_applied = true;
  } catch (error) {
    console.error('Failed to apply theme immediately:', error);
    // Default to dark theme on error
    document.documentElement.classList.add('dark');
    document.documentElement.setAttribute('data-theme', 'dark');
    const colors = getDefaultColors(true);
    const themeData: ThemeData = { mode: 'dark', isDark: true, colors };
    // Apply the actual color values as CSS custom properties
    applyThemeColors(colors);
    useThemeStore.setState({
      themeMode: 'dark',
      isDark: true,
      isSystem: false,
      isLoading: false,
      themeData,
    });
  }
}

// Call this immediately when the module is imported
applyThemeImmediately();

// Initialize theme on app start
export function initializeTheme() {
  const store = useThemeStore.getState();
  
  // Only load from backend if we haven't already applied the theme
  // This prevents double theme application and flash
  if (!(window as any).__zaroxi_theme_applied) {
    store.loadThemeSettings().catch(error => {
      console.error('Failed to load theme settings:', error);
      useThemeStore.setState({ isLoading: false });
    });
  }
  
  // Listen to system theme changes
  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
  const handleSystemThemeChange = (e: MediaQueryListEvent) => {
    const { themeMode } = useThemeStore.getState();
    if (themeMode === 'system') {
      const isDark = e.matches;
      const colors = getDefaultColors(isDark);
      updateCssVariables(isDark);
      useThemeStore.getState().applyTheme({ 
        mode: 'system', 
        isDark,
        colors,
      });
    }
  };
  
  mediaQuery.addEventListener('change', handleSystemThemeChange);
  
  // Setup backend listener (Tauri only)
  let cleanupListener: (() => void) | undefined;
  setupThemeListener().then(unlisten => {
    if (unlisten) {
      cleanupListener = () => {
        try {
          unlisten.then(f => f());
        } catch (error) {
          console.error('Error cleaning up listener:', error);
        }
      };
    }
  });
  
  return () => {
    mediaQuery.removeEventListener('change', handleSystemThemeChange);
    if (cleanupListener) {
      cleanupListener();
    }
  };
}
