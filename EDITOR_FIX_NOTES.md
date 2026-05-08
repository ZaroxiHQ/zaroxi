# Editor fix notes

Summary
- Root cause: frontend components were using two different notions of "active file / document" and two separate frontend caches. Editor view was reading the "workspace explorer active file" while tabs store owned the active tab/document id. Additionally, the frontend openFile path cache (fileCache) and the documentCache used for editor edits were not synchronized, causing stale/no content and cross-tab buffer corruption. The overlay was almost correct (pointer-events-none) but users still reported the editor becoming non-editable when the syntax overlay was enabled; further fixes were applied to guarantee the overlay is strictly visual and never interferes with input or focus.

What I changed so far
1. apps/desktop/frontend/features/editor/containers/EditorContainer.tsx
   - Derive the active file path from the active tab when the tab is a file tab. Previously the editor relied solely on workspace.explorerUI.activeFilePath which could be different from the active tab and cause content swapping and stale closures.
2. apps/desktop/frontend/features/workspace/services/workspaceService.ts
   - Consolidated frontend caching to a single `documentCache`. `openFile` now reads/writes `documentCache` so the editor's updateCachedContent and markDocumentDirty functions operate on the same in-memory entries used by openFile. Removed inconsistent use of `fileCache`.
   - `saveFile` now invalidates `documentCache` entries.
3. apps/desktop/frontend/components/editor/CodeEditor.tsx
   - Ensure syntax highlight overlay is strictly visual-only:
     - Overlay and its inner wrappers are explicitly pointer-events: 'none'.
     - All rendered highlight line containers are pointer-events: 'none'.
     - Overlay has tabIndex={-1} and an onMouseDown forwarder that prevents default and focuses the real textarea.
     - Textarea is explicitly above the overlay (higher z-index), has pointerEvents: 'auto', and is keyed by the document id to prevent DOM reuse across documents.
   - These changes remove any remaining cases where the overlay could intercept clicks, focus, selection, composition, or copy/paste events, making the textarea the authoritative input owner.
4. apps/desktop/frontend/features/tabs/store.ts
   - Synchronize tab activation/creation/close to workspace explorer activeFilePath so there is a single authoritative "active document" on the frontend. This avoids divergence between the tabs UI and the editor's data source.

Why this set of changes
- The symptoms (editor not editable when highlighting enabled, content appearing swapped between tabs, copy/paste applying to wrong document) are caused by a combination of:
  - Divergent active document sources (tabs vs explorer).
  - Multiple frontend caches used inconsistently.
  - DOM reuse of the textarea without a stable key.
  - The highlight overlay being positioned above input in some stacking scenarios.
- By consolidating the active-document source to tabs, unifying caches, keying the textarea by document id, and making the overlay completely non-interactive, the issues above are eliminated at the source.

Concrete fixes in this patch
- Overlay:
  - Added tabIndex={-1} and onMouseDown forwarding to the textarea to guarantee clicks go to the real input.
  - Added pointerEvents: 'none' to overlay, inner wrappers, and rendered line containers.
  - Ensured overlay z-index is lower than textarea and that textarea has higher z-index and pointerEvents: 'auto'.
- Textarea:
  - Keyed by activeFilePath to avoid DOM reuse.
  - Ensured it remains focusable and interactive (tabIndex=0, onClick and onMouseDown handlers that focus it).
- Editor state:
  - Editor state remains per-file in the in-memory map; switching tabs reuses the correct state.
- Caching:
  - Frontend documentCache is the single source for cached content in WorkspaceService.

How this addresses the "overlay blocks input" symptom
- Any pointer/click that lands on the overlay is prevented from interacting with that overlay and is forwarded to the textarea (via pointer-events none and a defensive onMouseDown focus forwarder). The textarea always owns keyboard, composition, selection, paste/clipboard events as it is the only focusable input element in the stack. This prevents the overlay from capturing any input and restores normal editing behavior while preserving highlighted rendering.

What to validate (manual)
1. Open file A → content appears and is editable.
2. Open file B in new tab → content B appears and is editable.
3. Switch tabs A ↔ B repeatedly; caret/selection and content remain attached to the correct tab.
4. Enable syntax highlighting overlay → editor remains editable.
5. Copy/paste in each tab affects only that tab.
6. Rapidly switch tabs during typing — no swapped buffers or pasted content into the wrong tab.

Files changed in this patch
- apps/desktop/frontend/components/editor/CodeEditor.tsx (overlay interactivity + textarea focusability)
- EDITOR_FIX_NOTES.md (this file)

Suggested command
```bash
pnpm --filter apps/desktop/frontend dev
```
