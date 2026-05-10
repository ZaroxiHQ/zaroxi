/**
 * Theme helper for the CodeMirror experiment.
 *
 * Instead of statically importing CodeMirror theme helpers (which can break
 * Vite analysis when optional deps are missing) this module provides a small
 * runtime CSS injector that uses your app's CSS variables. It ensures the
 * gutter, active-line and basic token classes are visible and controlled by
 * theme variables.
 *
 * The injector is intentionally conservative and uses CSS variables that your
 * theme system already provides (e.g. --editor-background, --editor-foreground).
 *
 * Usage:
 *   import { injectCmTheme } from './theme';
 *   injectCmTheme();
 */

export function injectCmTheme() {
  // Only inject once
  if ((window as any).__cm_theme_injected__) return;
  (window as any).__cm_theme_injected__ = true;

  const css = `
/* Editor basics */
.cm-editor { background: var(--editor-background, #0b1220); color: var(--editor-foreground, #dbe7ff); height: 100%; }

/* Content caret and selection */
.cm-content { caret-color: var(--editor-caret, #ffffff); }
.cm-selectionBackground { background: rgba(255,255,255,0.06); }

/* Active line */
.cm-activeLine { background-color: rgba(255,255,255,0.02); }

/* Gutters */
.cm-gutters {
  background: var(--editor-gutter, #071021);
  color: var(--editor-gutter-foreground, #7f8aa3);
  border-right: 0;
  padding: 0 6px;
  box-sizing: border-box;
}

/* Line number spacing */
.cm-lineNumbers .cm-gutterElement {
  padding-right: 6px;
}

/* Active line gutter (the gutter number for the active line) */
.cm-activeLineGutter { color: var(--editor-active-gutter-foreground, #cfe3ff); }

/* Fold marker (uses CodeMirror default classes) */
.cm-foldGutterElement {
  color: var(--editor-fold-foreground, #7f8aa3);
}

/* Token classes emitted by our Tree-sitter bridge */
.cm-ts-keyword { color: var(--syntax-keyword, #ffb86b); }
.cm-ts-string { color: var(--syntax-string, #a7f3d0); }
.cm-ts-comment { color: var(--syntax-comment, #7f8aa3); font-style: italic; }
.cm-ts-number { color: var(--syntax-number, #f0c674); }
.cm-ts-function { color: var(--syntax-function, #7aa2f7); }
.cm-ts-variable { color: var(--syntax-variable, #dbe7ff); }
.cm-ts-type { color: var(--syntax-type, #9ccfd8); }
.cm-ts-constant { color: var(--syntax-constant, #f78c6c); }
`;

  const style = document.createElement('style');
  style.setAttribute('data-codemirror-theme-injected', '1');
  style.innerHTML = css;
  document.head.appendChild(style);
}
