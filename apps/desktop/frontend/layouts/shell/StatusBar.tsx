import { useWorkspaceStore } from '@/features/workspace/stores/useWorkspaceStore';
import { cn } from '@/lib/utils';
import { useEffect, useState, useMemo } from 'react';
import { WorkspaceService } from '@/features/workspace/services/workspaceService';
import { useLayoutMode } from '@/hooks/useLayoutMode';

interface StatusBarProps {
  className?: string;
}

/**
 * Lightweight file extension -> language label detection.
 */
function detectLanguageExtension(path: string): string | undefined {
  const ext = path.split('.').pop()?.toLowerCase() ?? '';
  const known: Record<string, string> = {
    rs: 'Rust',
    ts: 'TypeScript',
    tsx: 'TSX',
    js: 'JavaScript',
    jsx: 'JSX',
    go: 'Go',
    toml: 'TOML',
    yaml: 'YAML',
    yml: 'YAML',
    json: 'JSON',
    md: 'Markdown',
    html: 'HTML',
    css: 'CSS',
    cpp: 'C++',
    c: 'C',
    java: 'Java',
    py: 'Python',
    sh: 'Shell',
  };
  return known[ext];
}

/**
 * StatusBar — non‑wrapping, prioritized layout.
 *
 * Improvements:
 * - Prevents multi-line wrapping by using no-wrap and truncation.
 * - Left (primary) and right (metadata) groups both truncate safely.
 * - Low‑priority metadata hides on narrow layouts.
 */
export function StatusBar({ className }: StatusBarProps) {
  const { currentWorkspace, isLoading, explorerUI } = useWorkspaceStore();
  const activeFilePath = explorerUI?.activeFilePath ?? null;
  const [fileMeta, setFileMeta] = useState<{ largeFileMode?: string; contentTruncated?: boolean } | null>(null);

  const layoutMode = useLayoutMode();
  const isNarrow = layoutMode === 'narrow';
  const isMedium = layoutMode === 'medium';

  useEffect(() => {
    let cancelled = false;
    if (activeFilePath) {
      WorkspaceService.openFile({ path: activeFilePath }).then((resp) => {
        if (!cancelled) {
          setFileMeta({
            largeFileMode: resp.largeFileMode ?? 'Normal',
            contentTruncated: resp.contentTruncated ?? false,
          });
        }
      }).catch(() => {
        if (!cancelled) setFileMeta(null);
      });
    } else {
      setFileMeta(null);
    }
    return () => { cancelled = true; };
  }, [activeFilePath]);

  const fileName = useMemo(() => (activeFilePath ? activeFilePath.split(/[\\/]/).pop() ?? '—' : null), [activeFilePath]);
  const languageLabel = useMemo(() => (activeFilePath ? detectLanguageExtension(activeFilePath) ?? 'Plain Text' : null), [activeFilePath]);

  const largeFileIndicator = fileMeta && fileMeta.largeFileMode && fileMeta.largeFileMode !== 'Normal'
    ? ` ${fileMeta.largeFileMode === 'VeryLarge' ? 'Very Large' : 'Large'} File`
    : null;

  const truncationIndicator = fileMeta && fileMeta.contentTruncated ? ' (truncated)' : null;

  return (
    <footer
      className={cn('flex items-center px-3 text-[12px] leading-none', className)}
      style={{
        height: 28,
        backgroundColor: 'var(--color-status-bar-background)',
        borderTop: '0.5px solid var(--color-divider-subtle)',
        color: 'var(--color-text-primary)',
        boxSizing: 'border-box',
        paddingLeft: 12,
        paddingRight: 12,
        gap: 8,
        whiteSpace: 'nowrap',
        overflow: 'hidden',
      }}
      role="contentinfo"
    >
      {/* LEFT: Workspace info (primary) */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, minWidth: 0, flex: '1 1 0', overflow: 'hidden' }}>
        <div style={{ minWidth: 0, overflow: 'hidden' }}>
          <div style={{ fontSize: 12, fontWeight: 600, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', maxWidth: 320 }}>
            {currentWorkspace ? currentWorkspace.name : 'No workspace'}
          </div>
          <div style={{ fontSize: 11, color: 'var(--color-text-secondary)', fontFamily: 'var(--font-mono)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', maxWidth: 320 }}>
            {currentWorkspace ? currentWorkspace.rootPath.split('/').pop() : ''}
          </div>
        </div>

        {isLoading && (
          <div style={{ display: 'inline-flex', alignItems: 'center', gap: 8, color: 'var(--color-accent)' }}>
            <span style={{ width: 8, height: 8, borderRadius: 8, background: 'var(--color-accent)' }} />
            <span style={{ fontSize: 12, fontWeight: 600 }}>Loading…</span>
          </div>
        )}
      </div>

      {/* RIGHT: File metadata (priority-based) */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, minWidth: 0, justifyContent: 'flex-end', overflow: 'visible', flex: '0 0 auto' }}>
        {/* Filename (always shown when available) */}
        <div style={{ minWidth: 0, maxWidth: 420, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={activeFilePath ?? undefined}>
          <span style={{ fontSize: 12, fontWeight: 600, display: 'inline-block', maxWidth: '100%', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {activeFilePath ? fileName : 'No file'}
          </span>
        </div>

        {/* Core metadata - visible on medium+ */}
        {!isNarrow && (
          <div style={{ display: 'flex', gap: 8, alignItems: 'center', color: 'var(--color-text-secondary)', flexShrink: 1, minWidth: 0, overflow: 'hidden' }}>
            <span style={{ fontSize: 11, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis' }}>{languageLabel ?? 'Plain Text'}</span>
            {isMedium && <span style={{ fontSize: 11 }}>UTF-8</span>}
            {isMedium && <span style={{ fontSize: 11 }}>LF</span>}
          </div>
        )}

        {/* Low-priority indicators - only on wide */}
        {!isNarrow && !isMedium && (largeFileIndicator || truncationIndicator) && (
          <span style={{ color: 'var(--color-warning)', fontSize: 11, fontWeight: 600, whiteSpace: 'nowrap' }}>
            {largeFileIndicator ?? ''}{truncationIndicator ?? ''}
          </span>
        )}
      </div>
    </footer>
  );
}
