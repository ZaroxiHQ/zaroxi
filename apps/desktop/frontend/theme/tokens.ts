/**
 * theme/tokens.ts — JS-friendly token accessors.
 *
 * Do NOT hardcode palette hex values here. Instead expose CSS variable
 * references so components using inline styles bind to the canonical
 * theme CSS custom properties (which are set at runtime from the Rust crate).
 *
 * Example usage in React:
 *  style={{ background: UI_TOKENS.appBackground }}
 */
const UI_TOKENS = {
  appBackground: 'var(--color-app-background)',
  outerShell: 'var(--color-shell-background)',
  mainPanel: 'var(--color-panel-background)',
  secondaryPanel: 'var(--color-panel-secondary)',
  elevatedPanel: 'var(--color-elevated-panel-background)',

  border: 'var(--color-border)',
  dividerSubtle: 'var(--color-divider-subtle)',

  textPrimary: 'var(--color-text-primary)',
  textSecondary: 'var(--color-text-secondary)',
  textMuted: 'var(--color-text-muted)',

  accent: 'var(--color-accent)',
  accentHover: 'var(--color-accent-hover)',
  accentSoft: 'var(--color-accent-soft)',

  editorGutter: 'var(--color-editor-gutter-background)',
  editorLineHighlight: 'var(--color-editor-line-highlight)',

  shadowSoft: 'var(--shadow-soft, 0 8px 30px rgba(2,6,23,0.6))',
};

export default UI_TOKENS;
