import { useWorkspaceStore } from '@/features/workspace/stores/useWorkspaceStore';
import { cn } from '@/lib/utils';
import { useEffect, useState, useMemo } from 'react';
import { WorkspaceService } from '@/features/workspace/services/workspaceService';
import { useLayoutMode } from '@/hooks/useLayoutMode';

interface StatusBarProps {
  className?: string;
}

/**
 * Simple language detection by file extension.
 * Used in the status bar to show the language mode without an extra bridge call.
 */
function detectLanguageExtension(path: string): string | undefined {
  const ext = path.split('.').pop()?.toLowerCase() ?? '';
  const known: Record<string, string> = {
    rs: 'Rust',
    ts: 'TypeScript',
    tsx: 'TypeScript JSX',
    js: 'JavaScript',
    jsx: 'JavaScript JSX',
    mjs: 'JavaScript',
    cjs: 'JavaScript',
    py: 'Python',
    go: 'Go',
    toml: 'TOML',
    yaml: 'YAML',
    yml: 'YAML',
    json: 'JSON',
    jsonc: 'JSON with Comments',
    md: 'Markdown',
    mdx: 'MDX',
    html: 'HTML',
    htm: 'HTML',
    css: 'CSS',
    scss: 'SCSS',
    sass: 'SASS',
    less: 'Less',
    styl: 'Stylus',
    vue: 'Vue',
    svelte: 'Svelte',
    c: 'C',
    h: 'C Header',
    cpp: 'C++',
    java: 'Java',
    py: 'Python',
    sh: 'Shell Script',
  };
  return known[ext];
}

export function StatusBar({ className }: StatusBarProps) {
  const { currentWorkspace, isLoading, explorerUI } = useWorkspaceStore();
  const activeFilePath = explorerUI?.activeFilePath ?? null;
  const [fileMeta, setFileMeta] = useState<{ largeFileMode?: string; contentTruncated?: boolean } | null>(null);

  const layoutMode = useLayoutMode();
  const isNarrow = layoutMode === 'narrow';

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
      className={cn('flex items-center justify-between px-3 text-[12px] leading-none', className)}
      style={{
        height: 28,
        backgroundColor: 'var(--color-status-bar-background, var(--color-panel-background))',
        borderTop: '0.5px solid var(--color-divider-subtle)',
        color: 'var(--color-text-primary)',
      }}
      role="contentinfo"
    >
      <div className="flex items-center space-x-3">
        <span style={{ fontSize: 12, fontWeight: 600 }}>{currentWorkspace ? currentWorkspace.name : 'No workspace'}</span>
        {currentWorkspace && !isNarrow && (
          <span style={{ color: 'var(--color-text-secondary)', fontFamily: 'var(--font-mono)', fontSize: 11 }}>
            {currentWorkspace.rootPath.split('/').pop()}
          </span>
        )}
        {isLoading && (
          <span style={{ color: 'var(--color-accent)', fontWeight: 600, display: 'inline-flex', alignItems: 'center', gap: 6 }}>
            <span style={{ width: 8, height: 8, borderRadius: 8, background: 'var(--color-accent)' }} />
            Loading…
          </span>
        )}
      </div>

      <div className="flex items-center space-x-4">
        {activeFilePath ? (
          <>
            <span style={{ fontSize: 12, fontWeight: 600, maxWidth: 220, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={activeFilePath}>
              {fileName}
            </span>
            {!isNarrow && <span style={{ color: 'var(--color-text-secondary)', fontSize: 11 }}>{languageLabel}</span>}
            {!isNarrow && <span style={{ color: 'var(--color-text-secondary)', fontSize: 11 }}>UTF-8</span>}
            {!isNarrow && <span style={{ color: 'var(--color-text-secondary)', fontSize: 11 }}>LF</span>}
            {(largeFileIndicator || truncationIndicator) && (
              <span style={{ color: 'var(--color-warning)', fontSize: 11, fontWeight: 600 }}>
                {largeFileIndicator ?? ''}{truncationIndicator ?? ''}
              </span>
            )}
          </>
        ) : (
          <span style={{ color: 'var(--color-text-secondary)', fontSize: 11 }}>No file</span>
        )}
      </div>
    </footer>
  );
}
