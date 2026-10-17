<script lang="ts">
  /**
   * The permanent contextual Help window (docs/design.md §1.1, item 2). It narrates
   * whatever the mouse is over in the front-most window; the text comes over
   * the help bus (`$lib/help-bus`). Closing it only hides it.
   */
  import { onMount } from 'svelte';
  import { onHelp, type HelpMessage } from '$lib/help-bus';

  // The opening line of the original manual, shown until something is hovered.
  const idle: HelpMessage = {
    window: 'Help',
    element: 'Help window',
    text: 'Make the pointer fly over a button or a field of the front-most and active window to dynamically obtain help and hints.',
  };

  let current = $state<HelpMessage>(idle);

  onMount(() => {
    let unlisten: (() => void) | undefined;
    onHelp((message) => (current = message))
      .then((fn) => (unlisten = fn))
      .catch(() => {}); // plain browser, no Tauri
    return () => unlisten?.();
  });
</script>

<div class="help">
  <h1>{current.window} : {current.element}</h1>
  <p>{current.text}</p>
</div>

<style>
  .help {
    height: 100vh;
    box-sizing: border-box;
    padding: 8px 12px;
    overflow: auto;
    background: var(--bg-raised);
  }
  h1 {
    margin: 0 0 4px;
    font-size: var(--fs-ui);
    font-weight: 700;
  }
  p {
    margin: 0;
    -webkit-user-select: text;
    user-select: text;
  }
</style>
