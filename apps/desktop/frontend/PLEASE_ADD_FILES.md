Please add the following missing files (full contents) so I can continue the UI refactor.

Why I need them
- I must operate on the exact current source for these files to produce safe SEARCH/REPLACE edits that preserve behavior.
- The ThemeProvider and runtime tokens are already added; I now need the layout and panel files to refactor the shell and connect the editor/sidebar/assistant.
- I will not modify any files you have not pasted here.

Files I need you to paste (exact path + full file contents)
1) apps/desktop/frontend/components/sidebar/WorkspaceExplorer.tsx
2) apps/desktop/frontend/components/assistant/AssistantPanel.tsx
3) apps/desktop/frontend/src/App.tsx
4) apps/desktop/frontend/tailwind.config.cjs

Optional but helpful (paste if available)
- apps/desktop/frontend/components/ui/Button.tsx
- apps/desktop/frontend/layouts/shell/TopBar.css
- any editor-specific CSS under apps/desktop/frontend/components/editor/*.css

Minimal next steps I'll take after you paste them
1) Create focused SEARCH/REPLACE edits for Shell, TopBar, ActivityRail, PanelHost to match the mockup structure and use runtime CSS variables.
2) Refactor WorkspaceExplorer, Editor, AssistantPanel to consume tokens and layout primitives.
3) Wire src/App.tsx to render the new Shell and preserve current stores/services.
4) If needed, adjust tailwind.config.cjs to align spacing, radii and font tokens.

Short implementation notes (why this matters)
- I will keep the Rust crate as the authoritative theme source; frontend components will read CSS variables provided by ThemeProvider.
- All edits will be small, incremental SEARCH/REPLACE blocks so you can review and apply them safely.
- I will not add or change any other files unless you paste them.

Suggested local commands you can run after applying edits I provide:
```bash
git status
git add -A
git commit -m "ui: apply shell/layout refactor changes"
```

Paste the requested files (exact, full contents) in your next message and I will produce precise SEARCH/REPLACE edits.
