/**
 * The full keyboard map of Serial Cloner 2.6.1, from the User Manual (§III)
 * and the changelog. `C` is ⌘ on macOS and Ctrl elsewhere, as in the manual's
 * "[C+…]" notation. Windows wire these up from M2 on; M0 only defines them so
 * the Toolbar's hover help can show the right key on the right platform.
 */

export type Modifier = 'C' | 'shift' | 'alt';

export interface Shortcut {
  /** Stable identifier, also the command name in the menu bar. */
  id: string;
  /** Menu label. */
  label: string;
  /** Menu the entry lives in. */
  menu: 'File' | 'Edit' | 'Sequence' | 'Restriction' | 'Generate' | 'Features' | 'Window';
  /** `KeyboardEvent.key`, lower-case. */
  key: string;
  modifiers: readonly Modifier[];
}

const C = ['C'] as const;
const CS = ['C', 'shift'] as const;
const CA = ['C', 'alt'] as const;

export const SHORTCUTS: readonly Shortcut[] = [
  { id: 'new', label: 'New', menu: 'File', key: 'n', modifiers: C },
  { id: 'open', label: 'Open…', menu: 'File', key: 'o', modifiers: C },
  { id: 'close', label: 'Close', menu: 'File', key: 'w', modifiers: C },
  { id: 'save', label: 'Save', menu: 'File', key: 's', modifiers: C },
  { id: 'print', label: 'Print…', menu: 'File', key: 'p', modifiers: C },
  { id: 'preferences', label: 'Preferences…', menu: 'File', key: ',', modifiers: C },
  { id: 'quit', label: 'Quit', menu: 'File', key: 'q', modifiers: C },
  { id: 'undo', label: 'Undo', menu: 'Edit', key: 'z', modifiers: C },
  { id: 'redo', label: 'Redo', menu: 'Edit', key: 'y', modifiers: C },
  { id: 'select-all', label: 'Select All', menu: 'Edit', key: 'a', modifiers: C },
  { id: 'cut', label: 'Cut', menu: 'Edit', key: 'x', modifiers: C },
  { id: 'copy', label: 'Copy', menu: 'Edit', key: 'c', modifiers: C },
  { id: 'copy-fasta', label: 'Copy as Fasta', menu: 'Edit', key: 'k', modifiers: C },
  { id: 'copy-formatted', label: 'Copy Formatted (Full Seq + Features)', menu: 'Edit', key: 'k', modifiers: CS },
  { id: 'copy-multifasta', label: 'Copy All Sequences To Multifasta', menu: 'Edit', key: 'j', modifiers: CS },
  { id: 'paste', label: 'Paste', menu: 'Edit', key: 'v', modifiers: C },
  { id: 'find', label: 'Find…', menu: 'Edit', key: 'f', modifiers: C },
  { id: 'find-orf', label: 'Find ORF', menu: 'Edit', key: 'r', modifiers: C },
  { id: 'show-info', label: 'Show Info…', menu: 'Sequence', key: 'i', modifiers: C },
  { id: 'copy-translation', label: 'Copy Translation', menu: 'Sequence', key: 't', modifiers: C },
  { id: 'uppercase', label: 'UPPERCASE', menu: 'Sequence', key: 'u', modifiers: C },
  { id: 'lowercase', label: 'Lowercase', menu: 'Sequence', key: 'l', modifiers: C },
  { id: 'antiparallel', label: 'Antiparallel', menu: 'Sequence', key: '/', modifiers: C },
  { id: 'graphic-map', label: 'Graphic Map', menu: 'Restriction', key: 'g', modifiers: C },
  { id: 'sequence-map', label: 'Sequence Map', menu: 'Restriction', key: 'm', modifiers: C },
  { id: 'construct', label: 'Build a Construct…', menu: 'Generate', key: '1', modifiers: C },
  { id: 'adaptor', label: 'Create an Adaptor…', menu: 'Generate', key: '2', modifiers: C },
  { id: 'pcr', label: 'Run a PCR…', menu: 'Generate', key: '3', modifiers: C },
  { id: 'shrna', label: 'Make shRNA…', menu: 'Generate', key: '4', modifiers: C },
  { id: 'gateway', label: 'Gateway™ Cloning…', menu: 'Generate', key: '5', modifiers: C },
  { id: 'extract', label: 'Extract Fragment', menu: 'Generate', key: '6', modifiers: C },
  { id: 'align', label: 'Align two Sequences…', menu: 'Generate', key: '7', modifiers: C },
  { id: 'feature-from-selection', label: 'New Feature From Selection', menu: 'Features', key: '@', modifiers: C },
  { id: 'scan', label: 'Scan Sequence', menu: 'Features', key: 's', modifiers: CA },
  { id: 'bring-all-to-front', label: 'Bring All To Front', menu: 'Window', key: 'b', modifiers: C },
];

export const isMac: boolean =
  typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform ?? navigator.userAgent);

/** Renders "⇧⌘K" on macOS and "Ctrl+Shift+K" elsewhere. */
export function formatShortcut(shortcut: Pick<Shortcut, 'key' | 'modifiers'>): string {
  const key = shortcut.key.length === 1 ? shortcut.key.toUpperCase() : shortcut.key;
  if (isMac) {
    const mods = ['alt', 'shift', 'C'] as const; // Apple's canonical order: ⌥ ⇧ ⌘
    const glyph = { alt: '⌥', shift: '⇧', C: '⌘' } as const;
    return mods.filter((m) => shortcut.modifiers.includes(m)).map((m) => glyph[m]).join('') + key;
  }
  const mods = ['C', 'alt', 'shift'] as const;
  const name = { C: 'Ctrl', alt: 'Alt', shift: 'Shift' } as const;
  return [...mods.filter((m) => shortcut.modifiers.includes(m)).map((m) => name[m]), key].join('+');
}

/** True if the keyboard event is this shortcut on the current platform. */
export function matchesShortcut(event: KeyboardEvent, shortcut: Pick<Shortcut, 'key' | 'modifiers'>): boolean {
  const primary = isMac ? event.metaKey : event.ctrlKey;
  return (
    event.key.toLowerCase() === shortcut.key &&
    primary === shortcut.modifiers.includes('C') &&
    event.shiftKey === shortcut.modifiers.includes('shift') &&
    event.altKey === shortcut.modifiers.includes('alt')
  );
}

export const shortcutById = (id: string): Shortcut | undefined => SHORTCUTS.find((s) => s.id === id);
