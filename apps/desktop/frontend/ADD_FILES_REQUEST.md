I cannot access your repository files directly. Please paste the full contents (exact, whole file) of the files below into the chat so I can produce precise SEARCH/REPLACE edits that update them.

Files I need you to add now (exact path + full file contents):
- apps/desktop/frontend/components/sidebar/WorkspaceExplorer.tsx
- apps/desktop/frontend/components/assistant/AssistantPanel.tsx
- apps/desktop/frontend/src/App.tsx
- apps/desktop/frontend/tailwind.config.cjs

Optional but helpful (paste if available):
- apps/desktop/frontend/components/ui/Button.tsx
- apps/desktop/frontend/layouts/shell/TopBar.css
- any editor-specific CSS under apps/desktop/frontend/components/editor/*.css

How to paste:
- For each file, paste the path followed by the full file contents in a single message.
- Ensure the pasted content is exactly the file content (no truncation).
- If a file does not exist, say so explicitly.

What I'll do after you add them:
1. Produce focused SEARCH/REPLACE blocks (one small logical change per block) to implement the mockup-driven shell/layout refactor while preserving behavior.
2. Create or update shell/layout components (Shell, TopBar, ActivityRail, PanelHost), Editor wiring, WorkspaceExplorer, AssistantPanel and connect them to the runtime ThemeProvider tokens you already exposed.
3. Keep the Rust crate as the authoritative theme source; frontend will read runtime CSS variables.

Suggested commands to run locally after applying edits (run from repo root):
git add -A
git commit -m "WIP: apply UI shell/layout refactor changes"

When you've pasted the files above I will continue and produce exact SEARCH/REPLACE blocks to update them.
