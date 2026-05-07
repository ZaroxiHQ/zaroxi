/*!
 Visual design constants for the gutter.

 Important: colors are derived from the theme CSS variables so the Rust crate
 remains the single source of truth. Components should not hardcode colors.
*/
export const GUTTER_STYLE = {
  /** Background color of the gutter (subtle distinction from code area) */
  BACKGROUND: 'var(--color-editor-gutter-background)',
  /** Color for line numbers (low contrast) */
  LINE_NUMBER_COLOR: 'var(--color-text-faint)',
  /** Color for the current line number (subtle emphasis) */
  CURRENT_LINE_COLOR: 'var(--color-text-primary)',
  /** Separator between gutter and code */
  SEPARATOR_COLOR: 'var(--color-divider-subtle)',
  /** Separator width */
  SEPARATOR_WIDTH: 1,
  /** Font family for line numbers */
  FONT_FAMILY: 'inherit',
  /** Font size for line numbers - use a reasonable size that matches the editor */
  FONT_SIZE: '12px',
  /** Line height for line numbers (should match editor line height) */
  LINE_HEIGHT: 22,
} as const;
