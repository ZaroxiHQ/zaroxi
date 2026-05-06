Files I still need you to add (exact path + full file contents)

Why
- I will refactor the shell/layout and wire the editor, sidebar and assistant panels into it.
- To produce safe, minimal SEARCH/REPLACE edits I must work against the exact current source of the files listed below.
- I will not modify any other files unless you add them.

Please paste the full contents of these files (one message is fine):

Required (please add):
- apps/desktop/frontend/layouts/shell/Shell.tsx
- apps/desktop/frontend/layouts/shell/TopBar.tsx
- apps/desktop/frontend/layouts/shell/ActivityRail.tsx
- apps/desktop/frontend/layouts/shell/PanelHost.tsx
- apps/desktop/frontend/components/editor/Editor.tsx
- apps/desktop/frontend/components/sidebar/WorkspaceExplorer.tsx
- apps/desktop/frontend/components/assistant/AssistantPanel.tsx
- apps/desktop/frontend/src/App.tsx (or the top-level composition that renders the Shell)
- apps/desktop/frontend/tailwind.config.cjs (or tailwind.config.js)

Helpful but optional:
- apps/desktop/frontend/components/ui/Button.tsx (shared button primitive)
- apps/desktop/frontend/layouts/shell/TopBar.css (or other component css)
- apps/desktop/frontend/components/editor/*.css

Short plan once you add the files
1. Create a single Shell layout (rounded outer container, drop shadow, token-driven surfaces).
2. Implement TopBar, ActivityRail, PanelHost and StatusBar wiring to use CSS variables set by the crate ThemeProvider.
3. Refactor Editor, WorkspaceExplorer, AssistantPanel to consume tokens and layout primitives.
4. Keep all theme palette authoritative in the Rust -> ThemeProvider path; frontend will only read runtime CSS variables.
5. Deliver focused SEARCH/REPLACE edits (one small logical change per block) so you can review and apply incrementally.

If you want to prepare locally after applying edits I provide, run:
```bash
git status
git add -A
```
