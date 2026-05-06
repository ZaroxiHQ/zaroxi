/**
 * Centralized UI tokens derived from the provided mockup palette.
 * Import this from UI primitives when inline values are needed.
 * Global CSS variables are defined in globals.css and are the primary source
 * for colors in CSS; these tokens are convenient for inline styles or JS usage.
 */

const UI_TOKENS = {
  /* Surfaces */
  appBackground: '#0b1020',
  outerShell: '#0f1428',
  mainPanel: '#12182b',
  secondaryPanel: '#161d33',
  elevatedPanel: '#1b2340',

  /* Borders / highlights */
  border: '#27314f',
  borderHighlight: '#33406a',
  dividerSubtle: 'rgba(39,49,79,0.36)',

  /* Typography */
  textPrimary: '#eef2ff',
  textSecondary: '#b7c0e0',
  textMuted: '#7f89ad',
  textDisabled: '#5c6687',

  /* Accents */
  accent: '#6c63ff',
  accentHover: '#7c72ff',
  accentDeep: '#5246e5',
  accentGlow: 'rgba(108, 99, 255, 0.18)',
  assistantAccent: '#8b7dff',

  /* Code semantic colors */
  codeBlue: '#7aa2ff',
  codeCyan: '#79c0ff',
  codeGreen: '#98c379',
  codeYellow: '#e5c07b',
  codeOrange: '#d19a66',
  codeRed: '#e06c75',

  /* Status */
  success: '#6dd6a6',
  warning: '#f0b86b',

  /* Layout */
  radius: '8px',
  radiusSm: '6px',
  spacing4: '4px',
  spacing8: '8px',
  spacing12: '12px',
  spacing16: '16px',

  /* Shadows */
  shadowSoft: '0 8px 30px rgba(2,6,23,0.6)',
  shadowSubtle: '0 6px 18px rgba(2,6,23,0.45)',
};

export default UI_TOKENS;
