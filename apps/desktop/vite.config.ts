import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import fs from 'fs';

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
      '@codemirror/language': path.resolve(__dirname, 'node_modules', '@codemirror', 'language'),
      '@codemirror/lang-javascript': path.resolve(__dirname, 'node_modules', '@codemirror', 'lang-javascript'),
      '@codemirror/lang-rust': path.resolve(__dirname, 'node_modules', '@codemirror', 'lang-rust'),
      '@codemirror/lang-json': path.resolve(__dirname, 'node_modules', '@codemirror', 'lang-json'),
      '@codemirror/lang-html': path.resolve(__dirname, 'node_modules', '@codemirror', 'lang-html'),
      '@codemirror/lang-css': path.resolve(__dirname, 'node_modules', '@codemirror', 'lang-css'),
      '@codemirror/lang-markdown': path.resolve(__dirname, 'node_modules', '@codemirror', 'lang-markdown'),
      '@codemirror/lang-yaml': path.resolve(__dirname, 'node_modules', '@codemirror', 'lang-yaml'),
      '@codemirror/legacy-modes': path.resolve(__dirname, 'node_modules', '@codemirror', 'legacy-modes'),
      'web-tree-sitter': path.resolve(__dirname, 'node_modules', 'web-tree-sitter'),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: true,
    open: true, // Automatically open the browser

    // Serve runtime wasm files directly from the repository `crates/zaroxi-lang-syntax/runtime/treesitter`
    // when requested under the /crates/zaroxi-lang-syntax/runtime/treesitter/ URL path.
    // This middleware ensures the correct Content-Type (application/wasm) and avoids Vite's
    // HTML fallback for missing files when the runtime directory lives outside the Vite root.
    configureServer(server) {
      const runtimePath = path.resolve(__dirname, '..', '..', 'crates/zaroxi-lang-syntax/runtime/treesitter');

      // Diagnostic middleware for serving runtime wasm files.
      // Adds verbose logging to help determine why requests might fall through to Vite's HTML handler.
      server.middlewares.use((req, res, next) => {
        const url = req.url || '';
        const prefix = '/crates/zaroxi-lang-syntax/runtime/treesitter/';
        if (!url.startsWith(prefix)) {
          return next();
        }

        const rel = decodeURIComponent(url.slice(prefix.length));
        const file = path.join(runtimePath, rel);

        // Log the incoming wasm request and the resolved filesystem path
        // so you can confirm the middleware runs and which file it attempts to serve.
        // This log appears in the terminal where `vite` runs.
        // eslint-disable-next-line no-console
        console.debug('[vite-middleware] wasm request:', url, '=>', file);

        fs.stat(file, (err, stat) => {
          if (err || !stat.isFile()) {
            // Diagnostics when the file is missing; log reasons before falling through.
            // eslint-disable-next-line no-console
            console.debug('[vite-middleware] file not found or not a file:', file, 'err=', err ? err.message : null);
            return next();
          }

          const ext = path.extname(file).toLowerCase();
          const contentType = ext === '.wasm' ? 'application/wasm' : 'application/octet-stream';
          res.setHeader('Content-Type', contentType);
          // eslint-disable-next-line no-console
          console.debug('[vite-middleware] serving file:', file, 'content-type=', contentType);
          const stream = fs.createReadStream(file);
          stream.on('error', (streamErr) => {
            // eslint-disable-next-line no-console
            console.debug('[vite-middleware] stream error for file:', file, streamErr);
            return next();
          });
          stream.pipe(res);
        });
      });
    },

    // Allow serving files from the frontend and the treesitter runtime directory
    // so the browser can fetch WASM grammars located under crates/zaroxi-lang-syntax/runtime/treesitter.
    fs: {
      allow: [
        path.resolve(__dirname, 'frontend'),
        path.resolve(__dirname, 'frontend/public'),
        path.resolve(__dirname, '..', '..', 'crates/zaroxi-lang-syntax/runtime/treesitter'),
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
    //
    // Note: we deliberately avoid pre-bundling the legacy `@codemirror/gutter` / `@codemirror/fold`
    // packages because they are merged into other packages in newer CM6 releases and can
    // cause version-mismatch import errors. We ensure we import the required symbols
    // deterministically from `@codemirror/view` / `@codemirror/language` in our code.
    include: [
      '@codemirror/view',
      '@codemirror/state',
      '@codemirror/commands',
      '@codemirror/language',
      '@lezer/highlight',
      '@codemirror/lang-javascript',
      '@codemirror/lang-rust',
      '@codemirror/lang-json',
      '@codemirror/lang-html',
      '@codemirror/lang-css',
      '@codemirror/lang-markdown',
      // Ensure these newly added language packages are pre-bundled by Vite so they resolve
      // in production builds as well as dev.
      '@lezer/toml',
      '@codemirror/lang-python',
      '@codemirror/lang-xml',
      'web-tree-sitter',
    ],
  },
});
