Refactor steps (concise)
1) Add theme/tokens.ts (done) and centralize colors as CSS variables in globals.css.
2) Update UI primitives (Icon, Text) to consume tokens for color/inline styles.
3) Replace globals.css with palette-driven variables and small utility classes.
4) Next: add Shell, TopBar, ActivityRail, PanelHost, Editor, WorkspaceExplorer, AssistantPanel files (please add their current contents so I can modify them).
5) After you add the shell/layout files I'll produce small SEARCH/REPLACE edits per file to implement the full mockup.

Suggested local commands:
git add -A
git commit -m "ui: introduce theme tokens and update globals & primitives"
