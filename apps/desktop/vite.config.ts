import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  root: path.resolve(__dirname, 'frontend'),
  publicDir: path.resolve(__dirname, 'frontend/public'),
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './frontend'),
      '@lib': path.resolve(__dirname, './frontend/lib'),
      '@features': path.resolve(__dirname, './frontend/features'),
      '@components': path.resolve(__dirname, './frontend/components'),
      '@styles': path.resolve(__dirname, './frontend/styles'),
      // Resolve CodeMirror and web-tree-sitter packages from the desktop node_modules
      // so Vite can find them when frontend/node_modules is not present (Tauri setup).
      '@codemirror/view': path.resolve(__dirname, 'node_modules', '@codemirror', 'view'),
      '@codemirror/state': path.resolve(__dirname, 'node_modules', '@codemirror', 'state'),
      '@codemirror/commands': path.resolve(__dirname, 'node_modules', '@codemirror', 'commands'),
      '@codemirror/history': path.resolve(__dirname, 'node_modules', '@codemirror', 'history'),
      '@codemirror/gutter': path.resolve(__dirname, 'node_modules', '@codemirror', 'gutter'),
      '@codemirror/fold': path.resolve(__dirname, 'node_modules', '@codemirror', 'fold'),
      '@codemirror/language': path.resolve(__dirname, 'node_modules', '@codemirror', 'language'),
      '@codemirror/highlight': path.resolve(__dirname, 'node_modules', '@codemirror', 'highlight'),
      '@codemirror/lang-javascript': path.resolve(__dirname, 'node_modules', '@codemirror', 'lang-javascript'),
      '@codemirror/lang-rust': path.resolve(__dirname, 'node_modules', '@codemirror', 'lang-rust'),
      '@codemirror/lang-json': path.resolve(__dirname, 'node_modules', '@codemirror', 'lang-json'),
      'web-tree-sitter': path.resolve(__dirname, 'node_modules', 'web-tree-sitter'),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: true,
    open: true, // Automatically open the browser
    // Allow serving files from the frontend and the treesitter runtime directory
    // so the browser can fetch WASM grammars located under crates/zaroxi-lang-syntax/runtime/treesitter.
    fs: {
      allow: [
        path.resolve(__dirname, 'frontend'),
        path.resolve(__dirname, 'frontend/public'),
        path.resolve(__dirname, 'crates/zaroxi-lang-syntax/runtime/treesitter'),
      ],
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: ['es2021', 'chrome100', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    outDir: path.resolve(__dirname, 'dist'),
  },
  optimizeDeps: {
    // Keep Tauri plugins excluded from Vite dependency pre-bundling.
    exclude: [
      '@tauri-apps/api',
      '@tauri-apps/plugin-clipboard-manager',
      '@tauri-apps/plugin-global-shortcut',
      '@tauri-apps/plugin-notification',
      '@tauri-apps/plugin-shell',
    ],
    // Pre-bundle CodeMirror and web-tree-sitter so Vite can resolve them properly
    // from the desktop root node_modules when the frontend is served from apps/desktop/frontend.
    include: [
      '@codemirror/view',
      '@codemirror/state',
      '@codemirror/commands',
      '@codemirror/history',
      '@codemirror/gutter',
      '@codemirror/fold',
      '@codemirror/language',
      '@codemirror/highlight',
      '@codemirror/lang-javascript',
      '@codemirror/lang-rust',
      '@codemirror/lang-json',
      'web-tree-sitter',
    ],
  },
});
