import { bridge } from '@/lib/bridge';

// Frontend-side document cache that mirrors the Rust cache.
// Keyed by canonical file path, stores the full document content and metadata.
// This allows tab switching to be instant without any IPC call.
//
// NOTE: consolidate to a single canonical cache (documentCache) so that
// openFile, updateCachedContent, markDocumentDirty, and save flow all
// operate on the same entries.
const documentCache = new Map<string, {
  content: string;
  language?: string;
  lineCount?: number;
  charCount?: number;
  largeFileMode?: string;
  contentTruncated?: boolean;
  isDirty: boolean;
}>();

// Domain types (mirror Rust DTOs)
export interface OpenWorkspaceRequest {
  path: string;
}

export interface OpenWorkspaceResponse {
  workspaceId: string;
  rootPath: string;
  fileCount: number;
}

export interface ListDirectoryRequest {
  path: string;
}

export interface DirectoryEntryDto {
  path: string;
  name: string;
  isDir: boolean;
  fileType?: string;
  size?: number;
  modified?: string;
}

export interface OpenFileRequest {
  path: string;
}

export interface OpenFileResponse {
  content: string;
  language?: string;
  lineCount?: number;
  charCount?: number;
  largeFileMode?: string;
  contentTruncated?: boolean;
}

export interface SaveFileRequest {
  path: string;
  content: string;
}

export interface OpenDialogResponse {
  selectedPath?: string;
}

// Explorer-specific types
export interface ExplorerTreeNode {
  id: string;
  path: string;
  name: string;
  isDir: boolean;
  fileType?: string;
  size?: number;
  modified?: string;
  children?: ExplorerTreeNode[];
  parentPath?: string;
}

export interface WorkspaceTreeRequest {
  workspaceId: string;
  rootPath: string;
}

export interface WorkspaceTreeResponse {
  workspaceId: string;
  rootPath: string;
  tree: ExplorerTreeNode[];
}

// Workspace events
export interface WorkspaceEvent {
  type: 'workspace_opened' | 'workspace_closed' | 'directory_changed';
  payload: unknown;
}

// New types for the editor document system
export interface OpenDocumentResponse {
  documentId: string;
  path: string;
  lineCount: number;
  charCount: number;
  largeFileMode: string;
  isReadOnly: boolean;
  content?: string;
  /** Indicates whether the returned content was truncated (file was large). */
  contentTruncated?: boolean;
}

export interface VisibleLinesRequest {
  documentId: string;
  startLine: number;
  count: number;
}

export interface VisibleLinesResponse {
  lines: LineDto[];
  totalLines: number;
}

export interface LineDto {
  index: number;
  text: string;
}

export interface EditRequest {
  documentId: string;
  startByte: number;
  oldEndByte: number;
  newText: string;
}

// Helper to detect Tauri environment
function isTauriEnvironment(): boolean {
  if (typeof window === 'undefined') return false;
  
  // Check for Tauri globals
  if (window.__TAURI__ !== undefined) return true;
  if ((window as any).__TAURI_INTERNALS__ !== undefined) return true;
  
  // Check user agent
  if (navigator.userAgent.includes('Tauri')) return true;
  
  // Try to detect by checking for Tauri-specific APIs
  try {
    // @ts-ignore
    if (window.__TAURI_IPC__ !== undefined) return true;
  } catch {}
  
  // Additional check for Tauri 2.0
  try {
    // @ts-ignore
    if (window.__TAURI__?.core !== undefined) return true;
  } catch {}
  
  return false;
}

/**
 * WorkspaceService - feature-specific business operations
 * 
 * This layer:
 * - Orchestrates multiple bridge calls
 * - Handles business logic that spans multiple operations
 * - Provides a clean API for containers/stores
 */
export class WorkspaceService {
  // Command operations
  static async openWorkspace(request: OpenWorkspaceRequest): Promise<OpenWorkspaceResponse> {
    // Check if we're in Tauri environment - use multiple detection methods
    const isTauri = 
      typeof window !== 'undefined' && 
      (window.__TAURI__ !== undefined || 
       (window as any).__TAURI_INTERNALS__ !== undefined ||
       navigator.userAgent.includes('Tauri'));
    
    if (!isTauri) {
      // Return mock workspace data for development
      return {
        workspaceId: 'mock-workspace-id-' + Date.now(),
        rootPath: request.path,
        fileCount: 42,
      };
    }
    
    try {
      const result = await bridge.invoke<any>('open_workspace', { request });
      
      // Handle both camelCase and snake_case
      const workspaceId = result.workspaceId || result.workspace_id;
      const rootPath = result.rootPath || result.root_path;
      const fileCount = result.fileCount || result.file_count;
      
      if (!workspaceId || !rootPath) {
        throw new Error('Invalid workspace response');
      }
      
      return {
        workspaceId,
        rootPath,
        fileCount: fileCount || 0,
      };
    } catch (error) {
      throw error;
    }
  }

  static async listDirectory(request: ListDirectoryRequest): Promise<DirectoryEntryDto[]> {
    const tauri = isTauriEnvironment();
    if (!tauri) {
      // Return mock children for development
      return [
        {
          path: `${request.path}/child1.rs`,
          name: 'child1.rs',
          isDir: false,
          fileType: 'rs',
          size: 123,
          modified: new Date().toISOString(),
        },
        {
          path: `${request.path}/subdir`,
          name: 'subdir',
          isDir: true,
          fileType: undefined,
          size: undefined,
          modified: new Date().toISOString(),
        },
      ];
    }
    return await bridge.invoke<DirectoryEntryDto[]>('list_directory', { request });
  }

  static async openFile(request: OpenFileRequest): Promise<OpenFileResponse> {
    // Return cached version if available (frontend document cache)
    const cached = documentCache.get(request.path);
    if (cached) {
      return {
        content: cached.content,
        language: cached.language,
        lineCount: cached.lineCount,
        charCount: cached.charCount,
        largeFileMode: cached.largeFileMode,
        contentTruncated: cached.contentTruncated,
      };
    }

    const docResponse = await this.openDocument(request.path);
    const content = docResponse.content ?? '';
    const response: OpenFileResponse = {
      content,
      // Use language detected by the backend when available. Previously this
      // was left undefined which caused the editor to treat files as plaintext.
      language: docResponse.language,
      lineCount: docResponse.lineCount,
      charCount: docResponse.charCount,
      largeFileMode: docResponse.largeFileMode,
      contentTruncated: docResponse.contentTruncated,
    };

    // Store in frontend document cache for instant subsequent access
    documentCache.set(request.path, {
      content: response.content,
      language: response.language,
      lineCount: response.lineCount,
      charCount: response.charCount,
      largeFileMode: response.largeFileMode,
      contentTruncated: response.contentTruncated,
      isDirty: false,
    });

    return response;
  }

  static async saveFile(request: SaveFileRequest): Promise<void> {
    try {
      // Call the workspace-scoped save command (renamed on the backend to avoid conflicts).
      await bridge.invoke<void>('save_file_workspace', { request });
      // Invalidate frontend document cache for this path after a successful save
      documentCache.delete(request.path);
    } catch (error) {
      throw error;
    }
  }

  static async openFileDialog(): Promise<OpenDialogResponse> {
    const isTauri = isTauriEnvironment();
    
    if (!isTauri) {
      // For development, return a mock path
      const mockPath = '/Users/developer/projects/test-workspace';
      return { selectedPath: mockPath };
    }
    
    try {
      const result = await bridge.invoke<any>('open_file_dialog');
      
      // Handle both camelCase and snake_case
      const selectedPath = result.selectedPath || result.selected_path;
      
      return { selectedPath };
    } catch (error) {
      throw error;
    }
  }

  // New document-based API
  static async openDocument(path: string): Promise<OpenDocumentResponse> {
    const isTauri = isTauriEnvironment();
    
    if (!isTauri) {
      // Return mock document data for development
      return {
        documentId: 'mock-doc-' + Date.now(),
        path,
        lineCount: 100,
        charCount: 5000,
        largeFileMode: 'Normal',
        isReadOnly: false,
        content: '// Mock file content for development\n'.repeat(100),
      };
    }
    
    try {
      const result = await bridge.invoke<OpenDocumentResponse>('open_document', { path });
      return result;
    } catch (error) {
      throw error;
    }
  }

  static async getVisibleLines(request: VisibleLinesRequest): Promise<VisibleLinesResponse> {
    const isTauri = isTauriEnvironment();
    
    if (!isTauri) {
      // Return mock lines for development
      const lines: LineDto[] = [];
      for (let i = request.startLine; i < request.startLine + request.count && i < 100; i++) {
        lines.push({
          index: i,
          text: `Line ${i + 1} - mock content for development`,
        });
      }
      return {
        lines,
        totalLines: 100,
      };
    }
    
    try {
      const result = await bridge.invoke<VisibleLinesResponse>('get_visible_lines', {
        request,
      });
      return result;
    } catch (error) {
      throw error;
    }
  }

  static async applyEdit(request: EditRequest): Promise<void> {
    const isTauri = isTauriEnvironment();
    
    if (!isTauri) {
      // No-op for development
      return;
    }
    
    try {
      await bridge.invoke<void>('apply_edit', { request });
    } catch (error) {
      throw error;
    }
  }

  static async saveDocument(documentId: string): Promise<void> {
    const isTauri = isTauriEnvironment();
    
    if (!isTauri) {
      return;
    }
    
    try {
      await bridge.invoke<void>('save_document', { documentId });
    } catch (error) {
      throw error;
    }
  }

  static async getLineCount(documentId: string): Promise<number> {
    const isTauri = isTauriEnvironment();
    
    if (!isTauri) {
      return 100;
    }
    
    try {
      return await bridge.invoke<number>('get_line_count', { documentId });
    } catch (error) {
      throw error;
    }
  }

  // Explorer-specific operations
  static async getWorkspaceTree(request: WorkspaceTreeRequest): Promise<WorkspaceTreeResponse> {
    // Check if we're in Tauri environment - use multiple detection methods
    const isTauri = 
      typeof window !== 'undefined' && 
      (window.__TAURI__ !== undefined || 
       (window as any).__TAURI_INTERNALS__ !== undefined ||
       navigator.userAgent.includes('Tauri'));
    
    if (!isTauri) {
      // Return mock tree data for development
      const mockTree: ExplorerTreeNode[] = [
        {
          id: `${request.rootPath}/file1.rs`,
          path: `${request.rootPath}/file1.rs`,
          name: 'file1.rs',
          isDir: false,
          fileType: 'rs',
          size: 1234,
          modified: new Date().toISOString(),
          children: undefined,
          parentPath: request.rootPath,
        },
        {
          id: `${request.rootPath}/src`,
          path: `${request.rootPath}/src`,
          name: 'src',
          isDir: true,
          fileType: undefined,
          size: undefined,
          modified: new Date().toISOString(),
          children: [],
          parentPath: request.rootPath,
        },
        {
          id: `${request.rootPath}/Cargo.toml`,
          path: `${request.rootPath}/Cargo.toml`,
          name: 'Cargo.toml',
          isDir: false,
          fileType: 'toml',
          size: 567,
          modified: new Date().toISOString(),
          children: undefined,
          parentPath: request.rootPath,
        },
      ];
      return {
        workspaceId: request.workspaceId,
        rootPath: request.rootPath,
        tree: mockTree,
      };
    }
    
    try {
      // The Rust command expects a WorkspaceTreeRequest with camelCase fields
      // due to #[serde(rename_all = "camelCase")]
      // We pass it as a single request object
      const result = await bridge.invoke<WorkspaceTreeResponse>('get_workspace_tree', {
        request: {
          workspaceId: request.workspaceId,
          rootPath: request.rootPath
        }
      });
      return result;
    } catch (error: any) {
      // Extract error message from BridgeError
      const errorMessage = error?.message || error?.toString() || 'Unknown error building workspace tree';
      
      // Try to get more details
      let detailedMessage = 'Unknown error loading workspace tree';
      if (typeof error === 'string') {
        detailedMessage = error;
      } else if (error?.message) {
        detailedMessage = error.message;
      } else if (error?.toString) {
        detailedMessage = error.toString();
      }
      
      throw new Error(`Failed to load workspace tree: ${detailedMessage}`);
    }
  }

  static async loadDirectoryChildren(path: string): Promise<DirectoryEntryDto[]> {
    return await this.listDirectory({ path });
  }

  // Event subscriptions
  static onWorkspaceOpened(handler: (workspaceId: string, rootPath: string) => void) {
    return bridge.listen<{ workspaceId: string; rootPath: string }>('workspace:opened', (event) => {
      handler(event.workspaceId, event.rootPath);
    });
  }

  static onDirectoryChanged(handler: (path: string) => void) {
    return bridge.listen<{ path: string }>('workspace:directory_changed', (event) => {
      handler(event.path);
    });
  }

  // Business operations (combine multiple commands)
  static async openWorkspaceAndLoadTree(
    path: string
  ): Promise<{ workspace: OpenWorkspaceResponse; tree: WorkspaceTreeResponse }> {
    const workspace = await this.openWorkspace({ path });
    const tree = await this.getWorkspaceTree({
      workspaceId: workspace.workspaceId,
      rootPath: workspace.rootPath
    });
    
    return { workspace, tree };
  }

  static async openFileInEditor(path: string): Promise<OpenFileResponse> {
    return await this.openFile({ path });
  }

  /**
   * Get a cached document from the frontend cache without any IPC call.
   * Returns null if the document is not in the cache.
   */
  static getCachedDocument(path: string): OpenFileResponse | null {
    const cached = documentCache.get(path);
    if (!cached) return null;
    return {
      content: cached.content,
      language: cached.language,
      lineCount: cached.lineCount,
      charCount: cached.charCount,
      largeFileMode: cached.largeFileMode,
      contentTruncated: cached.contentTruncated,
    };
  }

  /**
   * Mark a document as dirty in the frontend cache.
   */
  static markDocumentDirty(path: string): void {
    const cached = documentCache.get(path);
    if (cached) {
      cached.isDirty = true;
    }
  }

  /**
   * Mark a document as clean in the frontend cache.
   */
  static markDocumentClean(path: string): void {
    const cached = documentCache.get(path);
    if (cached) {
      cached.isDirty = false;
    }
  }

  /**
   * Update the content of a cached document (after an edit).
   */
  static updateCachedContent(path: string, content: string): void {
    const cached = documentCache.get(path);
    if (cached) {
      cached.content = content;
      cached.isDirty = true;
    }
  }
}
