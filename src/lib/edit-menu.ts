/**
 * Edit ▸ Undo / Redo / Select All / Cut / Copy / Paste for windows that are
 * not a Sequence Window. The shortcuts are menu accelerators, so the
 * keystroke never reaches the field; the shell forwards the command to the
 * focused window as a `menu` event and this applies it to the focused text
 * field — the behaviour of the original's standard Edit menu.
 */
import type { UnlistenFn } from '@tauri-apps/api/event';
import { api, listenHere } from '$lib/doc';

type TextField = HTMLInputElement | HTMLTextAreaElement;

function activeField(): TextField | null {
  const el = document.activeElement;
  return el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement ? el : null;
}

function replaceSelection(field: TextField, text: string) {
  field.setRangeText(text, field.selectionStart ?? 0, field.selectionEnd ?? 0, 'end');
  field.dispatchEvent(new Event('input', { bubbles: true }));
}

/** Applies one Edit command; returns false if nothing applicable had focus. */
export async function applyEditCommand(cmd: string): Promise<boolean> {
  const field = activeField();
  switch (cmd) {
    case 'edit.undo':
      return field ? document.execCommand('undo') : false;
    case 'edit.redo':
      return field ? document.execCommand('redo') : false;
    case 'edit.select_all':
      if (field) {
        field.select();
        return true;
      }
      return false;
    case 'edit.copy': {
      const text = field ? field.value.slice(field.selectionStart ?? 0, field.selectionEnd ?? 0) : (window.getSelection()?.toString() ?? '');
      if (!text) return false;
      await api.clipboardWrite(text);
      return true;
    }
    case 'edit.cut': {
      if (!field || field.readOnly || field.disabled) return false;
      const text = field.value.slice(field.selectionStart ?? 0, field.selectionEnd ?? 0);
      if (!text) return false;
      await api.clipboardWrite(text);
      replaceSelection(field, '');
      return true;
    }
    case 'edit.paste': {
      if (!field || field.readOnly || field.disabled) return false;
      let text = '';
      try {
        text = await api.clipboardRead();
      } catch {
        return false;
      }
      if (!text) return false;
      replaceSelection(field, text);
      return true;
    }
    default:
      return false;
  }
}

/** Installs the handler for this window. Returns a disposer. */
export function installEditMenu(): () => void {
  let unlisten: UnlistenFn | undefined;
  listenHere<string>('menu', (e) => void applyEditCommand(e.payload))
    .then((u) => (unlisten = u))
    .catch(() => {}); // plain browser, no Tauri
  return () => unlisten?.();
}
