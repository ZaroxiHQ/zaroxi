/**
 * Theme bridge for the CodeMirror experiment.
 *
 * The app has an existing theme system (Rust crate + CSS variables).
 * For the trial we intentionally avoid providing a custom CodeMirror theme
 * that hardcodes colors. Instead return an empty extension so CodeMirror
 * uses default styling and the app's CSS variables control appearance.
 *
 * If you want a stronger visual match later, replace this with a small
 * EditorView.theme extension that references your app CSS variables.
 */

export function themeExtension() {
  return [];
}
