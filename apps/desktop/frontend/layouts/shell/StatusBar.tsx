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
        boxSizing: 'border-box',
        paddingLeft: 12,
        paddingRight: 12,
        // Allow wrapping and breathing room so content never gets clipped on narrow windows.
        flexWrap: 'wrap',
        gap: 8,
      }}
      role="contentinfo"
    >
      {/* Left group: workspace info (always present, ellipsised on small widths) */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, minWidth: 0, overflow: 'hidden', flex: '1 1 0' }}>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: 12, fontWeight: 600, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', maxWidth: 240 }}>
            {currentWorkspace ? currentWorkspace.name : 'No workspace'}
          </div>
          <div style={{ fontSize: 11, color: 'var(--color-text-secondary)', fontFamily: 'var(--font-mono)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', maxWidth: 240 }}>
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

      {/* Right group: file + metadata (always present, responsive truncation) */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, minWidth: 0, flex: '1 1 0', justifyContent: 'flex-end' }}>
        <div style={{ minWidth: 0, maxWidth: 360, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={activeFilePath ?? undefined}>
          <span style={{ fontSize: 12, fontWeight: 600, display: 'inline-block', maxWidth: '100%', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {activeFilePath ? fileName : 'No file'}
          </span>
        </div>

        <div style={{ display: 'flex', gap: 8, alignItems: 'center', color: 'var(--color-text-secondary)', flexShrink: 1 }}>
          <span style={{ fontSize: 11, minWidth: 0 }}>{languageLabel ?? 'Plain Text'}</span>
          <span style={{ fontSize: 11 }}>UTF-8</span>
          <span style={{ fontSize: 11 }}>LF</span>
        </div>

        {(largeFileIndicator || truncationIndicator) && (
          <span style={{ color: 'var(--color-warning)', fontSize: 11, fontWeight: 600, whiteSpace: 'nowrap' }}>
            {largeFileIndicator ?? ''}{truncationIndicator ?? ''}
          </span>
        )}
      </div>
    </footer>
  );
}
