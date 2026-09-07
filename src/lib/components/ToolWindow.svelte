<script lang="ts">
  /** Shared chrome of the cloning windows: a title strip and a body. */
  import type { Snippet } from 'svelte';
  let { title, subtitle = '', children }: { title: string; subtitle?: string; children: Snippet } = $props();
</script>

<div class="tool">
  <header>
    <h1>{title}</h1>
    {#if subtitle}<span class="sub">{subtitle}</span>{/if}
  </header>
  <div class="body">{@render children()}</div>
</div>

<style>
  .tool {
    display: flex;
    flex-direction: column;
    height: 100vh;
    box-sizing: border-box;
    padding: 8px 10px;
    gap: 8px;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 10px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 4px;
  }
  h1 {
    margin: 0;
    font-size: 16px;
  }
  .sub {
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  :global(.tool fieldset) {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 6px 10px;
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px 12px;
    align-items: center;
  }
  :global(.tool legend) {
    color: var(--fg-muted);
    font-size: var(--fs-small);
    padding: 0 4px;
  }
  :global(.tool label) {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  :global(.tool input[type='text']),
  :global(.tool textarea),
  :global(.tool select) {
    font: inherit;
  }
  :global(.tool .mono) {
    font-family: var(--font-mono);
  }
  :global(.tool .primary) {
    background: var(--accent);
    color: #fff;
    padding: 4px 18px;
  }
  :global(.tool .hint) {
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
  :global(.tool .error) {
    color: var(--select-end);
  }
  :global(.tool table) {
    width: 100%;
    border-collapse: collapse;
  }
  :global(.tool th),
  :global(.tool td) {
    padding: 2px 8px;
    text-align: left;
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
  }
  :global(.tool th) {
    position: sticky;
    top: 0;
    background: var(--bg);
    font-size: var(--fs-small);
  }
  :global(.tool .scroll) {
    flex: 1;
    min-height: 80px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
  }
  :global(.tool tr.selected) {
    background: var(--accent);
    color: #fff;
  }
  :global(.tool .num) {
    font-family: var(--font-mono);
    text-align: right;
  }
</style>
