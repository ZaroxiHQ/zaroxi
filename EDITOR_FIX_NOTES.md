# Editor fix notes

Summary
- Root cause: frontend components were using two different notions of "active file / document" and two separate frontend caches. Editor view was reading the "workspace explorer active file" while tabs store owned the active tab/document id. Additionally, the frontend openFile path cache (fileCache) and the documentCache used for editor edits were not synchronized, causing stale/no content and cross-tab buffer corruption. The overlay was almost correct (pointer-events-none) but to be defensive we explicitly set pointerEvents and z-index to avoid any accidental input interception.

What I changed
1. apps/desktop/frontend/features/editor/containers/EditorContainer.tsx
   - Derive the active file path from the active tab when the tab is a file tab. Previously the editor relied solely on workspace.explorerUI.activeFilePath which could be different from the active tab and cause content swapping and stale closures.
2. apps/desktop/frontend/features/workspace/services/workspaceService.ts
   - Consolidated frontend caching to a single `documentCache`. `openFile` now reads/writes `documentCache` so the editor's updateCachedContent and markDocumentDirty functions operate on the same in-memory entries used by openFile. Removed inconsistent use of `fileCache`.
   - `saveFile` now invalidates `documentCache` entries.
3. apps/desktop/frontend/components/editor/CodeEditor.tsx
   - Make the highlighting overlay explicitly non-interactive (pointerEvents: 'none', zIndex: 0) and give the textarea a stable React key (the document id/path) so the DOM input element can't be accidentally reused across different documents, preventing selection/caret/content stomping.
   - These changes preserve the overlay render-only behavior while making the textarea always own input/caret/paste.

Root cause details (concise)
- The editor UI used two separate sources of truth for "which file is active": tabs store (activeTabId + tab.id) vs workspace store (explorerUI.activeFilePath). When these diverged, the editor loaded a different file than the tab visual state — leading to swapped contents and edits being applied to the wrong document.
- The frontend had two caches (fileCache vs documentCache). openFile populated fileCache but editor update functions updated documentCache; so updates were often no-ops, causing dirty state not to be tracked and content to revert or be read from stale sources.
- React could reuse the same textarea DOM node across file switches (no key) which, combined with the above, let clipboard/input actions be applied to the wrong buffer.

Invariants now enforced
- Each open tab corresponds to a stable document id (tab.id is used when kind === 'file').
- The editor always receives the active tab's document id as its filePath prop.
- The frontend uses one document cache for editor-loaded files and editor-driven updates.
- The highlight overlay is visual-only (pointer-events: none) and cannot claim focus or selection.
- Textarea DOM is keyed by document id to avoid accidental DOM reuse.

How to validate (quick)
- Start frontend dev server and open multiple files; switch tabs while editing to ensure each tab retains its own content.
- Verify syntax highlighting still renders and that typing, paste, and save behave correctly.

Suggested commands
```bash
# Start frontend dev server for manual validation
pnpm --filter apps/desktop/frontend dev
```
