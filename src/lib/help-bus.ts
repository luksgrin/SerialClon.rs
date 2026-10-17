/**
 * The contextual Help window (docs/design.md §1.1, item 2).
 *
 * Any element in any window can carry a `data-help` attribute. When the mouse
 * moves over it, a `help:hover` event is sent to the Help window, which shows
 * "<Window> : <Element>" and the text — exactly what the original did for the
 * front-most window ("Sequence Window : Sequence Field — Enter here the
 * actual sequence of the fragment…").
 *
 * Optional `data-help-name` overrides the element's visible label.
 */
import { emitTo, type UnlistenFn } from '@tauri-apps/api/event';
import { listenHere } from '$lib/doc';

export const HELP_EVENT = 'help:hover';
export const HELP_WINDOW = 'help';

export interface HelpMessage {
  /** Human-readable window name, e.g. "Sequence Window". */
  window: string;
  /** Element name, e.g. "Sequence Field". */
  element: string;
  /** The help text. */
  text: string;
}

/**
 * Installs the mouseover listener for one window. Returns a disposer.
 * Emission failures are swallowed: the Help window may be hidden or, outside
 * Tauri (plain `vite dev` in a browser), absent altogether.
 */
export function installHelpBus(windowName: string, root: Document = document): () => void {
  let last: Element | null = null;
  const onOver = (event: MouseEvent) => {
    const target = event.target instanceof Element ? event.target.closest('[data-help]') : null;
    if (!target || target === last) return;
    last = target;
    const message: HelpMessage = {
      window: windowName,
      element:
        target.getAttribute('data-help-name') ??
        target.getAttribute('aria-label') ??
        target.textContent?.trim() ??
        '',
      text: target.getAttribute('data-help') ?? '',
    };
    void emitTo(HELP_WINDOW, HELP_EVENT, message).catch(() => {});
  };
  root.addEventListener('mouseover', onOver);
  return () => root.removeEventListener('mouseover', onOver);
}

/** Subscribes the Help window itself. */
export function onHelp(handler: (message: HelpMessage) => void): Promise<UnlistenFn> {
  return listenHere<HelpMessage>(HELP_EVENT, (event) => handler(event.payload));
}
