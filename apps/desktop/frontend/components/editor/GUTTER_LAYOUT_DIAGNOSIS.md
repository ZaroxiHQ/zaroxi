Root cause diagnosis and plan to fix gutter/content overlap
----------------------------------------------------------

1) Exact reason the text is overlapping the gutter
- The previous implementation rendered a single absolute gutter column (position: absolute; left: 0)
  and then placed the content inside a sibling container that relied on a margin-left to avoid
  overlap. However the visible line content itself used absolute-positioned line elements that
  were not structured as two-column rows. That produced cases where the computed margin/offset
  was not applied consistently to each absolutely-positioned line (or the gutter width value
  was undefined/zero during initial layout), so text could render underneath the gutter.

2) Whether gutter and code are rendered in the same flow without a stable column layout
- Yes. The gutter was effectively overlayed on top of the same flow (absolute positioned overlay)
  while content lines were placed with absolute top offsets but without explicit per-line gutter cells.
  This lacked a stable row-based two-column structure.

3) Whether line wrappers/content offsets are missing or wrong
- Line wrappers existed but were absolute positioned and not composed of gutter + content cells.
  As a result offsets were applied as a wrapper margin rather than as a guaranteed content column,
  yielding race conditions and overlapping hitboxes.

4) Whether gutter width is fixed/measured incorrectly
- The gutter width could be computed correctly, but it was applied at a container level (margin/padding)
  and that relied on the container being measured and available at the right time. If the container is
  not yet measured, marginLeft becomes 0 and text will overlap. Also absolute children sometimes ignored
  the container margin when layout context changed.

5) What exact two-column line layout will be implemented
- Replace the overlay gutter + margin-left pattern with a structural per-line layout:
  - Each rendered line will be a row container absolutely positioned (top = index * lineHeight).
  - Inside each row container use CSS flex (display:flex; align-items:center; height:lineHeight).
  - The row will contain two cells:
    - Gutter cell: fixed width (gutterWidth px), flex: 0 0 gutterWidth, right-aligned numbers, no overlap with content.
    - Content cell: flex: 1 1 auto, contains the line text/spans, starts immediately after the gutter cell.
  - Selection and caret overlays are offset by gutterWidth when showGutter is true.
  - This ensures the gutter and content are separate DOM elements in each row (no margin-only layout hacks).

Why this stops overlap without hacks
- Structural separation (two cells per row) guarantees the content area begins after the gutter regardless of timing
  or measurement ordering. Absolute top positioning keeps virtualization performance (single container with many
  absolutely positioned rows) while each row enforces column separation. No margin-left patching or global overlays
  needed; padding/margins are not relied upon for column separation.

Next: apply the single coherent commit that replaces the previous gutter overlay block in CustomSurface.tsx
with a proper per-line row layout as described above.
