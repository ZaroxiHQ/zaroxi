// Theme types matching Rust structures
export type ZaroxiTheme = 'dark' | 'light' | 'system';

export interface ThemeSettings {
  theme_mode: ZaroxiTheme;
}

export interface SemanticColors {
  // Background surfaces
  app_background: string;
  shell_background: string;
  panel_background: string;
  elevated_panel_background: string;
  editor_background: string;
  input_background: string;
  status_bar_background: string;
  title_bar_background: string;
  activity_rail_background: string;
  sidebar_background: string;
  tab_background: string;
  tab_active_background: string;
  assistant_panel_background: string;
  
  // Text colors
  text_primary: string;
  text_secondary: string;
  text_muted: string;
  text_faint: string;
  text_on_accent: string;
  text_on_surface: string;
  text_disabled: string;
  text_link: string;
  
  // UI elements
  border: string;
  border_subtle: string;
  divider: string;
  divider_subtle: string;
  panel_header_background: string;
  nested_surface_background: string;
  app_chrome_background: string;
  tab_strip_background: string;
  accent: string;
  accent_hover: string;
  accent_soft: string;
  accent_soft_background: string;
  
  // States
  hover_background: string;
  active_background: string;
  selected_background: string;
  selected_text_background: string;
  selected_editor_background: string;
  
  // Status colors
  success: string;
  warning: string;
  error: string;
  info: string;
  
  // Focus
  focus_ring: string;
  
  // Editor specific
  editor_gutter_background: string;
  editor_line_highlight: string;
  editor_cursor: string;
  editor_selection: string;
  editor_find_highlight: string;
  
  // Syntax colors
  syntax_keyword: string;
  syntax_function: string;
  syntax_string: string;
  syntax_comment: string;
  syntax_type: string;
  syntax_variable: string;
  syntax_constant: string;
  syntax_lifetime: string;
}

export interface ThemeData {
  mode: ZaroxiTheme;
  isDark: boolean;
  colors: SemanticColors;
}
