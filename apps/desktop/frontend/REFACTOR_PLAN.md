Refactor plan — Theme-first UI alignment (compact)

Goal
- Ensure the frontend consumes the crates/zaroxi-theme as single source of truth.
- Tighten surfaces, deepen contrast, make activity rail unique (icons grouped bottom),
  and align spacing / elevations to the provided mockup.

Summary of actions
1) Theme integration (done centrally)
   - Frontend already receives SemanticColors via ThemeProvider / theme-store.
   - Add a small compatibility layer (CSS aliases) that maps legacy variable names
     used across the codebase to the canonical crate-provided --color-* tokens.
   - Remove duplicated hardcoded palette usage in components and make components
     read only from the canonical CSS variables.

2) Token mapping (crates -> frontend)
   - Crate semantic names (SemanticColors) map to CSS properties:
     app_background          -> --color-app-background
     shell_background        -> --color-outer-shell
     panel_background        -> --color-panel-main
     elevated_panel_background -> --color-panel-elevated
     editor_background       -> --color-editor-background
     text_primary            -> --color-text-primary
     text_secondary          -> --color-text-secondary
     border                  -> --color-border
     accent                  -> --color-accent
     accent_hover            -> --color-accent-hover
     accent_soft             -> --color-accent-soft
   - Components must use --color-* variables only. Legacy aliases (e.g. --panel-main)
     are provided for compatibility while the refactor runs.

3) Layout problems found vs mockup (short)
   - Activity rail used top stacked icons (VS Code pattern). The mockup groups main actions bottom.
   - Shell surfaces were using mixed token names and some washed-out colors / fallbacks.
   - Tabs and top bar had generic spacing, not the compact premium rhythm the mockup requires.
   - Editor surface and gutter lacked the deeper contrast and subtle elevation.
   - Right assistant panel layout was generic; needs stronger hierarchy and better spacing.

4) Components to change (priority order)
   - Theme wiring / globals (alias layer)
   - Shell / AppShell / TopBar (top tabs / brand)
   - ActivityRail (move main icons bottom, calmer top)
   - TabStrip / TabItem (tighter, indigo subtle active)
   - Sidebar (Explorer) — spacing + active row tint
   - Editor surface and gutter (deeper canvas)
   - PanelHost (panel sizing / handles)
   - Assistant panel (composition, input composer)
   - StatusBar & Terminal (compact, thin)

5) Implementation plan (small focused edits)
   - Add REFACTOR_PLAN.md (this file).
   - Add CSS aliases to globals.css mapping legacy tokens to --color-*.
   - Update presentational components to consume --color-* variables and refine
     a few style objects to use enriched shadow / radii and tighter spacing.
   - Replace ActivityRail render so the main actions are located at the bottom.
   - Adjust TopBar header styling and tab area to use the accent and subtle inner highlight.
   - Keep edits minimal and incremental so you can review & test.

Files changed in this commit
- apps/desktop/frontend/REFACTOR_PLAN.md (this file)
- apps/desktop/frontend/app/globals.css (add legacy aliases -> canonical tokens)
- apps/desktop/frontend/layouts/shell/Shell.tsx (use canonical tokens & refined radius/shadow)
- apps/desktop/frontend/features/workbench/components/TopBar.tsx (improved title area & tab container styling)
- apps/desktop/frontend/features/workbench/components/ActivityRail.tsx (move main icons to bottom)
- apps/desktop/frontend/components/ui/Icon.tsx (use canonical text token + subtle hover)

Why this preserves architecture
- The authoritative palette is still the crate (zaroxi-theme) -> theme-store -> ThemeProvider.
- Frontend reads only CSS custom properties (--color-*). No duplicate palette logic.
- Legacy variable aliases ease migration without changing every component in one go.

How to validate locally after applying patches
- Start the app (tauri dev or vite) and inspect the shell surface:
  - App background should be deep navy (#0b1020)
  - Shell outer edge should use --color-outer-shell (#0f1428)
  - Activity rail icons should be grouped at the bottom
  - Top bar tabs should be compact with indigo subtle active bar
- Suggested commands:
  - git status
  - git add -A
  - git commit -m "ui: integrate crate theme tokens + activity rail refactor"
