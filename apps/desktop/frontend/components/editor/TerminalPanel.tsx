import React from 'react';
import { cn } from '@/lib/utils';
import { Icon } from '@/components/ui/Icon';
import { FONT_TOKENS } from '@/lib/theme/font-tokens';

/**
 * TerminalPanel
 *
 * Compact terminal attached to the editor column.
 * - Reads colors and spacing from --color-* CSS variables applied by ThemeProvider.
 * - Uses small, tight spacing to match the Zaroxi Studio rhythm.
 * - Exported as a named component (imported by AppShell).
 */

export function TerminalPanel() {
  return (
    <div
      className={cn('flex flex-col w-full h-full')}
      style={{
        backgroundColor: 'var(--color-panel-secondary)',
        color: 'var(--color-text-primary)',
        fontSize: 13,
        minHeight: 0,
      }}
      role="region"
      aria-label="Terminal"
    >
      {/* Terminal header (tab row) */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 12,
          padding: '8px 12px',
          borderBottom: '1px solid var(--color-divider-subtle)',
          backgroundColor: 'var(--color-panel-main)',
          minHeight: 36,
        }}
      >
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          {['Terminal', 'Problems', 'Output', 'Debug Console'].map((t, i) => {
            const active = i === 0;
            return (
              <button
                key={t}
                aria-pressed={active}
                className={cn('flex items-center gap-2', active ? 'cursor-default' : 'cursor-pointer')}
                style={{
                  background: active
                    ? 'linear-gradient(180deg, rgba(108,99,255,0.06), rgba(82,70,229,0.03))'
                    : 'transparent',
                  color: active ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
                  border: active ? '1px solid rgba(108,99,255,0.12)' : '1px solid transparent',
                  borderRadius: 8,
                  fontSize: 12,
                  padding: '6px 10px',
                  display: 'inline-flex',
                  alignItems: 'center',
                }}
                data-no-drag="true"
              >
                <span style={{ fontFamily: FONT_TOKENS.mono, fontSize: 11 }}>{t}</span>
                {active && <span style={{ width: 8, height: 8, borderRadius: 8, background: 'var(--color-accent)' }} />}
              </button>
            );
          })}
        </div>

        <div style={{ flex: 1 }} />

        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <button
            title="Clear terminal"
            className="p-1 rounded hover:bg-hover"
            style={{ color: 'var(--color-text-secondary)', background: 'transparent', border: 'none' }}
            aria-label="Clear terminal"
          >
            <Icon name="trash" size={14} />
          </button>

          <button
            title="Toggle soft wrap"
            className="p-1 rounded hover:bg-hover"
            style={{ color: 'var(--color-text-secondary)', background: 'transparent', border: 'none' }}
            aria-label="Toggle soft wrap"
          >
            <Icon name="wrap" size={14} />
          </button>
        </div>
      </div>

      {/* Terminal body */}
      <div style={{ flex: 1, overflow: 'auto', padding: 12, fontFamily: FONT_TOKENS.mono }}>
        <pre
          style={{
            margin: 0,
            color: 'var(--color-text-primary)',
            lineHeight: 1.45,
            fontSize: 12,
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-word',
          }}
        >
{`$ echo "Welcome to Zaroxi Studio Terminal"
$ ls -la
-rw-r--r--  1 user staff  1024 May  5 12:00 main.rs
drwxr-xr-x  6 user staff   192 May  5 12:00 src
$ npm run dev
> zaroxi-frontend@ dev
> vite

  vite v3.0.0 dev server running at:

  > Local: http://localhost:3000/
  > Network: use --host to expose
`}
        </pre>
      </div>

      {/* Composer / input row */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: 10,
          borderTop: '1px solid var(--color-divider-subtle)',
          backgroundColor: 'var(--color-panel-secondary)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <select
            aria-label="Terminal profile"
            defaultValue="bash"
            style={{
              background: 'transparent',
              color: 'var(--color-text-secondary)',
              border: '1px solid var(--color-border)',
              padding: '6px 8px',
              borderRadius: 8,
              fontSize: 12,
            }}
          >
            <option value="bash">bash</option>
            <option value="zsh">zsh</option>
            <option value="powershell">powershell</option>
            <option value="local">local</option>
          </select>
        </div>

        <input
          placeholder="Run command..."
          aria-label="Terminal command"
          style={{
            flex: 1,
            padding: '8px 12px',
            borderRadius: 8,
            border: '1px solid var(--color-border)',
            background: 'transparent',
            color: 'var(--color-text-primary)',
            fontFamily: FONT_TOKENS.mono,
            fontSize: 13,
          }}
        />

        <button
          className="px-3 py-2 rounded"
          style={{
            padding: '8px 12px',
            borderRadius: 8,
            border: 'none',
            background: 'linear-gradient(180deg, var(--color-accent), var(--color-accent-hover))',
            color: 'var(--color-text-on-accent)',
            fontWeight: 600,
            cursor: 'pointer',
          }}
          aria-label="Run command"
        >
          Run
        </button>
      </div>
    </div>
  );
}
