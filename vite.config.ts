import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// One HTML entry per Serial Cloner window (docs/design.md §3). Each becomes its own
// Tauri `WebviewWindow`; the path is what `tauri.conf.json` / `windows.rs`
// load, and Vite keeps it identical in dev and in `dist/`.
const windows = [
  'toolbar', 'help', 'sequence', 'find', 'graphic-map', 'sequence-map', 'about', 'genetic-code', 'alphabet', 'features',
  'construct', 'adaptor', 'pcr', 'shrna', 'gateway', 'site-usage', 'enzyme-library', 'virtual-cutter', 'silent-map',
  'align', 'web', 'codon-usage',
  'msa', 'prefs'] as const;

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: { $lib: fileURLToPath(new URL('./src/lib', import.meta.url)) },
  },
  // Tauri expects a fixed port and fails if it is busy.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    watch: { ignored: ['**/src-tauri/**', '**/crates/**', '**/target/**', '**/reference/**'] },
  },
  build: {
    // WebKit on macOS 10.15 and WebView2 on Windows are the oldest engines we ship to.
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    minify: !process.env.TAURI_ENV_DEBUG,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    rollupOptions: {
      input: Object.fromEntries(windows.map((w) => [w, `src/windows/${w}/index.html`])),
    },
  },
});
