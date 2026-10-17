/**
 * Preferences (⌘,), shared by every window.
 *
 * `initPrefs()` loads the values from the shell, applies the theme, the
 * colour and size tokens and the page setup to the document, and keeps them
 * live through the `prefs:changed` event. Components read the reactive
 * `prefs` store; the Preferences window writes through `savePrefs`.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { writable, get } from 'svelte/store';

/** Mirrors `prefs::Prefs` in `src-tauri/src/prefs.rs`. */
export interface Prefs {
  color_unique: string;
  color_double: string;
  color_particular: string;
  color_begin: string;
  color_end: string;
  color_feature: string;
  seq_font_size: number;
  gmap_re_font_size: number;
  gmap_feature_font_size: number;
  smap_char_size: number;
  smap_print_size: number;
  smap_per_line: number;
  smap_show_reverse: boolean;
  smap_aa_middle: boolean;
  gmap_box_re_names: boolean;
  gmap_box_unique: boolean;
  autolock: boolean;
  recent_files: number;
  warn_import: boolean;
  orf_min_codons: number;
  pcr_ta_overhang: boolean;
  autosave_secs: number;
  check_updates: boolean;
  proxy: string;
  toolbar_shift: number;
  limit_window_width: boolean;
  theme: 'modern' | 'aqua';
  read_speed: number;
  page_landscape: boolean;
  page_margin_mm: number;
  codon_as_code: boolean;
}

export interface UpdateReport {
  current: string;
  latest: string | null;
  newer: boolean;
  url: string | null;
  error: string | null;
}

/** Download / install progress of an in-app update (`update:progress`). */
export interface UpdateProgress {
  text: string;
  /** 0..1 while downloading with a known size. */
  fraction: number | null;
  /** True once installed, failed or cancelled. */
  done: boolean;
}

export const DEFAULT_PREFS: Prefs = {
  color_unique: '#f2e14c',
  color_double: '#f0a53a',
  color_particular: '#7cb3e8',
  color_begin: '#2ea043',
  color_end: '#d1242f',
  color_feature: '#84a4c0',
  seq_font_size: 13,
  gmap_re_font_size: 11,
  gmap_feature_font_size: 10,
  smap_char_size: 12,
  smap_print_size: 8,
  smap_per_line: 100,
  smap_show_reverse: true,
  smap_aa_middle: false,
  gmap_box_re_names: true,
  gmap_box_unique: true,
  autolock: false,
  recent_files: 10,
  warn_import: true,
  orf_min_codons: 33,
  pcr_ta_overhang: false,
  autosave_secs: 30,
  check_updates: true,
  proxy: '',
  toolbar_shift: 0,
  limit_window_width: false,
  theme: 'modern',
  read_speed: 1,
  page_landscape: false,
  page_margin_mm: 12,
  codon_as_code: false,
};

/** The live preferences. Windows read this; it starts with the defaults. */
export const prefs = writable<Prefs>(DEFAULT_PREFS);

/** Resolves once the shell's values have been loaded (or failed to load). */
export let prefsReady: Promise<Prefs> = Promise.resolve(DEFAULT_PREFS);

/** Applies theme, design tokens and page setup to this window's document. */
export function applyToDocument(p: Prefs) {
  const root = document.documentElement;
  root.dataset.theme = p.theme;
  const set = (name: string, value: string) => root.style.setProperty(name, value);
  set('--site-unique', p.color_unique);
  set('--site-double', p.color_double);
  set('--site-particular', p.color_particular);
  set('--select-begin', p.color_begin);
  set('--select-end', p.color_end);
  set('--feature-default', p.color_feature);
  set('--seq-font-size', `${p.seq_font_size}px`);
  set('--smap-print-size', `${p.smap_print_size}px`);
  let page = document.getElementById('page-setup') as HTMLStyleElement | null;
  if (!page) {
    page = document.createElement('style');
    page.id = 'page-setup';
    document.head.appendChild(page);
  }
  page.textContent = `@page { size: ${p.page_landscape ? 'landscape' : 'portrait'}; margin: ${p.page_margin_mm}mm; }`;
}

/** Loads the preferences and keeps them live. Call once per window. */
export function initPrefs(): Promise<Prefs> {
  applyToDocument(DEFAULT_PREFS);
  prefsReady = (async () => {
    try {
      const p = await invoke<Prefs>('get_prefs');
      prefs.set(p);
      applyToDocument(p);
      await listen<Prefs>('prefs:changed', (e) => {
        prefs.set(e.payload);
        applyToDocument(e.payload);
      });
      return p;
    } catch {
      return DEFAULT_PREFS; // plain browser
    }
  })();
  return prefsReady;
}

/** Writes new values; the shell persists them and broadcasts the change. */
export async function savePrefs(p: Prefs): Promise<Prefs> {
  const saved = await invoke<Prefs>('set_prefs', { prefs: p });
  prefs.set(saved);
  applyToDocument(saved);
  return saved;
}

/** Reset Preferences. */
export async function resetPrefs(): Promise<Prefs> {
  const saved = await invoke<Prefs>('reset_prefs');
  prefs.set(saved);
  applyToDocument(saved);
  return saved;
}

/**
 * Preferences ▸ Check now. The shell runs the updater: a release of the same
 * major version is offered for download and install (progress arrives as
 * `update:progress`), any other newer release gets a notice with a link.
 */
export const checkForUpdates = (): Promise<UpdateReport> => invoke<UpdateReport>('check_for_updates_now');

/** Follows the progress of an in-app update. Resolves to the unlisten function. */
export const onUpdateProgress = (cb: (p: UpdateProgress) => void) =>
  listen<UpdateProgress>('update:progress', (e) => cb(e.payload));

/** The current values, synchronously. */
export const currentPrefs = (): Prefs => get(prefs);

/**
 * File ▸ Print. WKWebView and WebView2 ignore `window.print()`, so the
 * shell opens the native print dialog for this window; a plain browser
 * falls back to `window.print()`.
 */
export async function printPage(): Promise<void> {
  try {
    await invoke<void>('print_window');
  } catch {
    window.print();
  }
}
