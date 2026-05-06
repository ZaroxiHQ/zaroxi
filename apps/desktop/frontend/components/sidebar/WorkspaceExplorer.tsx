/**
 * Sidebar Workspace Explorer
 *
 * Thin adapter that exposes the workspace explorer UI from the feature
 * folder under the sidebar components path. This lets layout code import
 * from "@/components/sidebar/WorkspaceExplorer" while the canonical
 * implementation remains in features/workspace/components.
 *
 * Keeping this adapter avoids duplicating logic and makes it easy to
 * move/rename the feature implementation later without touching many imports.
 */

import { WorkspaceExplorer as WorkspaceExplorerImpl } from '@/features/workspace/components/WorkspaceExplorer';

export default WorkspaceExplorerImpl;
