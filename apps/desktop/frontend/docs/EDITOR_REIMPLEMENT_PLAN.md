# Editor Highlighting Reimplementation Plan & Summary

This file documents the rationale, architecture, and the list of code changes applied to
reimplement the editor highlighting flow so it adheres to the "single source of truth"
and "input ownership" rules.

Why the old architecture failed
- Highlights were computed from a backend cache that was often out-of-sync with the
  frontend ephemeral edits. The frontend accepted local edits but did not update the
  server-side buffer for highlight computation, producing stale spans.
- Highlighting was effectively a separate virtual editor overlay that tried to own
  visual state. This produced a second "virtual" text model and caused partial/stuck
  rendering when caches diverged.
- Multiple caches existed (frontend documentCache, backend BufferManager). They were
  not always synchronized, and highlights were sometimes computed against the wrong
  cache/version.
- Race conditions: opening a file, reading a cached value, and async highlight fetches
  created interleavings that allowed partial or wrong content to be rendered.

New editor / highlighting architecture
- Single Source of Truth:
  - The textarea input and the frontend document model (per-tab EditorState) are the
    authoritative source for the editor text.
  - Highlighting is always requested for the exact current text string from that source.
- Input Ownership:
  - There is one real textarea that owns typing, IME, selection, clipboard, and caret.
  - The highlight rendering is strictly passive (pointer-events: none) and never handles input.
- Highlighting Ownership:
  - Highlights are derived data computed from the supplied text.
  - The frontend sends the exact current text to the backend highlight command `highlight_text`.
  - The backend hashes the supplied text and uses the existing cache pipeline keyed by that hash,
    ensuring highlights are never reused across different texts.
- Document / Tab Binding:
  - Each tab has a stable document id (path). Editor state map keeps per-document buffer.
  - Opening a file loads the full text into the editor state. Switching tab swaps active document id.
- Cache Rules:
  - The backend cache is used only for performance: cached highlight spans are keyed to the
    provided text's version/hash.
  - The frontend never accepts highlights computed for a different document/text.

What I changed (concise)
- Frontend:
  - apps/desktop/frontend/components/editor/CodeEditor.tsx
    - Replaced direct highlight-by-document-id approach with a "highlight_text" flow that sends
      the exact in-memory text to the backend (debounced). Responses ignored if out-of-order.
    - Ensures overlay rendering uses highlights computed from the current text, eliminating stale spans.
    - Switched to use bridge.invoke wrapper for normalized error handling.
- Backend (Tauri):
  - apps/desktop/src-tauri/src/commands/editor.rs
    - Added HighlightTextRequest DTO and `highlight_text` command.
    - `highlight_text` computes a hash-version for the supplied text and calls into the existing
      cache compute helper so previously implemented parsing/query logic is reused safely.
    - `highlight_text` maps spans to character offsets using the supplied text (no dependency on backend buffer).
- Documentation:
  - apps/desktop/frontend/docs/EDITOR_REIMPLEMENT_PLAN.md (this file) documents rationale and changed files.

How this prevents regressions
- Highlights are always produced for the active editor text; there is no chance the backend will return
  highlights computed for a different cached document.
- Debounced requests + request-id guarding avoid out-of-order application of stale highlight responses.
- The textarea remains the only input surface; highlights are passive visualization only.
- The backend cache remains useful for performance (re-using highlight computations for identical text),
  but because the key includes a hash of the supplied text, cached highlights are never reused across different texts.

Files changed
- apps/desktop/frontend/components/editor/CodeEditor.tsx
  - Import bridge, add text-based highlighting hook, ensure requests are debounced and guarded.
- apps/desktop/src-tauri/src/commands/editor.rs
  - Add HighlightTextRequest and highlight_text command that computes highlights for provided text.
- apps/desktop/frontend/docs/EDITOR_REIMPLEMENT_PLAN.md (this file) - documentation.

Manual validation checklist (run locally in dev):
1. Open file A → full content renders immediately.
2. Enable syntax highlighting → editor remains editable.
3. Type into the editor → content updates correctly and highlights follow.
4. Line numbers match the actual text (no phantom lines).
5. Switch between tabs → each tab shows its correct, full content.
6. Copy/paste edits affect only the active document.
7. Scrolling stays aligned between text, gutter, and highlight overlay.

Suggested command to run the desktop frontend dev server:
```bash
pnpm --filter apps/desktop/frontend dev
```

If you want, I will:
- Add additional defensive checks (e.g. message size limits for highlight_text).
- Add an optional client-side fallback highlighter for non‑tauri/dev mode.
- Wire incremental edit application to backend `apply_edit` for collaborative or server-synced modes.
