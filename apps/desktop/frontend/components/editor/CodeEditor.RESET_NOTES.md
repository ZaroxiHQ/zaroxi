Removed/disabled layers:
- DOM overlay HTML rendering and per-line overlay components
- Per-line HighlightedLineView and related span-to-HTML rendering
- Background highlight hook / bridge-driven highlight application
- Secondary visible textarea used as a second readable glyph source

Sole readable layer:
- contenteditable div (contentRef) is now the only element that renders text glyphs.

Syntax highlighting:
- Temporarily removed to guarantee a single readable layer and eliminate ghosting.
- Reintroduction plan: compute syntax metadata (tree-sitter) but apply decorations without creating a second glyph image. Options: inline non-glyph-duplicating spans carefully applied to the same contenteditable node, or use CSS decorations/backgrounds that don't render separate glyphs.

Why this ends ghosting:
- Exactly one DOM element paints glyphs; no overlays or second text renderers to compete during composition or scrolling.

Shell (suggested) verification commands:
- git add -A && git commit -m "editor: hard reset to single contenteditable rendering layer to remove ghosting"
- yarn build
- yarn start
