# Editor fix notes

Summary
- Root cause: frontend components were using two different notions of "active file / document" and two separate frontend caches. Editor view was reading the "workspace explorer active file" while tabs store owned the active tab/document id. Additionally, the frontend openFile path cache (fileCache) and the documentCache used for editor edits were not synchronized, causing stale/no content and cross-tab buffer corruption. The overlay was almost correct (pointer-events-none) but to be defensive we explicitly set pointerEvents and z-index to avoid any accidental input interception.

What I changed so far
1. apps/desktop/frontend/features/editor/containers/EditorContainer.tsx
   - Derive the active file path from the active tab when the tab is a file tab. Previously the editor relied solely on workspace.explorerUI.activeFilePath which could be different from the active tab and cause content swapping and stale closures.
2. apps/desktop/frontend/features/workspace/services/workspaceService.ts
   - Consolidated frontend caching to a single `documentCache`. `openFile` now reads/writes `documentCache` so the editor's updateCachedContent and markDocumentDirty functions operate on the same in-memory entries used by openFile. Removed inconsistent use of `fileCache`.
   - `saveFile` now invalidates `documentCache` entries.
3. apps/desktop/frontend/components/editor/CodeEditor.tsx
   - Make the highlighting overlay explicitly non-interactive (pointerEvents: 'none', zIndex: 0) and give the textarea a stable React key (the document id/path) so the DOM input element can't be accidentally reused across different documents, preventing selection/caret/content stomping.
   - These changes preserve the overlay render-only behavior while making the textarea always own input/caret/paste.
4. apps/desktop/frontend/features/tabs/store.ts
   - Synchronize tab activation/creation/close to workspace explorer activeFilePath so there is a single authoritative "active document" on the frontend. This avoids divergence between the tabs UI and the editor's data source.

Why this set of changes
- The most frequent root cause for swapped buffers and stray edits is divergent active-file state (tabs vs explorer) plus multiple independent frontend caches. By making tabs the source of truth for which document is active, and by consolidating the frontend cache, we ensure:
  - Opening or activating a tab always makes the editor render the intended document.
  - The workspace explorer UI follows the active tab (so other code that still reads explorerUI.activeFilePath will see the same document).
  - Updates and saves operate on the same cache entries and therefore cannot get lost or applied to the wrong document.

Next steps (recommended/optional)
- Audit any remaining code paths that call WorkspaceService.openFile/openDocument or read explorerUI.activeFilePath directly; migrate them to use the tabs store where an editor/tab context exists.
- Add unit/integration tests for tab switching, rapid open/close, and assistant/apply behavior to catch regressions.
- Consider canonicalizing file paths consistently at the frontend boundary (e.g., resolve symlinks / normalize separators) to avoid mismatches between strings used as keys and BufferManager canonicalization on the Rust side.

Files changed in this patch
- apps/desktop/frontend/components/editor/CodeEditor.tsx
- apps/desktop/frontend/features/editor/containers/EditorContainer.tsx
- apps/desktop/frontend/features/workspace/services/workspaceService.ts
- apps/desktop/frontend/features/tabs/store.ts
- EDITOR_FIX_NOTES.md

Invariants now enforced
- Each open tab corresponds to a stable document id (tab.id is used when kind === 'file').
- The editor always receives the active tab's document id as its filePath prop.
- The frontend uses one document cache for editor-loaded files and editor-driven updates.
- The highlight overlay is visual-only (pointer-events: none) and cannot claim focus or selection.
- Textarea DOM is keyed by document id to avoid accidental DOM reuse.

How to validate (quick)
- Start frontend dev server and open multiple files; switch tabs while editing to ensure each tab retains its own content.
- Verify syntax highlighting still renders and that typing, paste, and save behave correctly.

Suggested command
```bash
pnpm --filter apps/desktop/frontend dev
```
