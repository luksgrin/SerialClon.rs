<script lang="ts">
  /**
   * The Toolbar: the thin always-on-top icon strip (PLAN.md §1.1, item 1).
   * Same buttons, same order and same groups as the original
   * (reference/ui/00-desktop-overview.png). In M0 only Help is live; the
   * others are wired up in the milestone that builds their window.
   */
  import { onMount } from 'svelte';
  import { appInfo, toggleHelpWindow, type AppInfo } from '$lib/tauri';
  import { api } from '$lib/doc';
  import { formatShortcut, shortcutById } from '$lib/shortcuts';

  interface Tool {
    id: string;
    label: string;
    help: string;
    /** Milestone that wires the button up; absent = live now. */
    milestone?: string;
    action?: (e: MouseEvent) => void;
  }

  const groups: Tool[][] = [
    [
      { id: 'new', label: 'New', help: 'Displays a new blank Sequence Window. Alt-click: New Degenerate DNA.', action: (e) => void api.newSequence(e.altKey) },
      { id: 'open', label: 'Open', help: 'Browses available storage devices and opens a sequence file into a Sequence Window (.xdna, FASTA, GenBank, text).', action: () => void api.openFiles() },
      { id: 'save', label: 'Save', help: 'Saves modifications done to the front-most sequence. Alt-click: Save As…', action: (e) => void api.toolbarAction(e.altKey ? 'file.save_as' : 'file.save') },
      { id: 'print', label: 'Print', help: 'Prints the front-most Graphic Map or Sequence Map.', action: () => void api.toolbarAction('file.print') },
    ],
    [
      { id: 'graphic-map', label: 'Graph Map', help: 'Displays the Graphic Map of the front-most sequence: circular or linear, with unique / double / particular sites and Features.', action: () => void api.toolbarAction('restriction.graphic_map') },
      { id: 'sequence-map', label: 'Seq Map', help: 'Displays the textual restriction map and multi-frame translation of the front-most sequence. A selection sets the translation range.', action: () => void api.toolbarAction('restriction.sequence_map') },
      { id: 'site-usage', label: 'Site Usage', help: 'Displays the site usage of the front-most sequence: the interactive table (filter against a second sequence) and the printable tables (all / unique / absent).', action: () => void api.toolbarAction('restriction.site_usage') },
      { id: 'find', label: 'Find', help: 'Opens the Find drawer of the front-most Sequence Window: sub-sequence, restriction site or ORF, on both strands. Press again to close it.', action: () => void api.toolbarAction('edit.find') },
    ],
    [
      { id: 'construct', label: 'Construct', help: 'Opens the Build a Construct window: fragments chosen between restriction sites (or on the Graphic Map), blunted if needed, and ligated into a new construct.', action: () => void api.toolbarAction('generate.construct') },
      { id: 'adaptor', label: 'Adaptor', help: 'Opens the Create an Adaptor window: core double-stranded sequence plus optional cohesive extensions, giving the oligonucleotides to order.', action: () => void api.toolbarAction('generate.adaptor') },
      { id: 'pcr', label: 'PCR', help: 'Opens the Run a PCR window: virtual amplification of a matrix with two primers that may carry un-annealed tails.', action: () => void api.toolbarAction('generate.pcr') },
      { id: 'shrna', label: 'shRNA', help: 'Opens the Make shRNA window: oligonucleotides for a shRNA insert from a stored layout.', action: () => void api.toolbarAction('generate.shrna') },
      { id: 'gateway', label: 'Gateway', help: 'Opens the Gateway™ cloning window: BP and LR reactions between donor and acceptor vectors, or homologous recombination.', action: () => void api.toolbarAction('generate.gateway') },
      { id: 'extract', label: 'Extract', help: 'Extracts the selected fragment (or the fragment between two sites on the Graphic Map) into a new Sequence Window, cohesive ends included.', action: () => void api.toolbarAction('generate.extract') },
      { id: 'align', label: 'Align', help: 'Opens the Align two Sequences window: local alignment with 3-frame translation.', milestone: 'M6' },
      { id: 'scan', label: 'Scan', help: 'Scans the front-most sequence for the Features of the enabled Collections (Features ▸ Scan Sequence). Features you edited are kept.', action: () => void api.toolbarAction('features.scan') },
    ],
    [
      { id: 'help', label: 'Help', help: 'Shows or hides the contextual Help window. Point at any button or field to get help and hints.', action: () => void toggleHelpWindow() },
    ],
  ];

  let info = $state<AppInfo | null>(null);
  onMount(async () => {
    try {
      info = await appInfo();
    } catch {
      info = null; // running in a plain browser (vite dev without Tauri)
    }
  });

  const shortcutOf = (id: string): string => {
    const s = shortcutById(id);
    return s ? ` [${formatShortcut(s)}]` : '';
  };

  const versionHelp = $derived(
    info
      ? `${info.name} ${info.version} — Tauri ${info.tauri_version}. Enzyme database: REBASE ${info.rebase_release ?? '?'} (${info.enzyme_count} enzymes).`
      : 'SerialClon.rs — an independent reimplementation of Serial Cloner.',
  );
</script>

<div class="toolbar" role="toolbar" aria-label="SerialClon toolbar">
  {#each groups as group, i (i)}
    {#if i > 0}<div class="sep" aria-hidden="true"></div>{/if}
    {#each group as tool (tool.id)}
      <button
        type="button"
        class="tool"
        disabled={!tool.action}
        data-help-name={tool.label}
        data-help={tool.help + shortcutOf(tool.id) + (tool.milestone ? ` (arrives in ${tool.milestone})` : '')}
        onclick={tool.action}
      >
        <span class="glyph" aria-hidden="true">{tool.label.charAt(0)}</span>
        <span class="label">{tool.label}</span>
      </button>
    {/each}
  {/each}
  <div class="spacer"></div>
  <div class="version" data-help-name="Version" data-help={versionHelp}>
    {info ? `v${info.version}` : ''}
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 100vh;
    padding: 0 8px;
    box-sizing: border-box;
    background: var(--bg);
  }
  .sep {
    width: 1px;
    height: 36px;
    margin: 0 6px;
    background: var(--border);
  }
  .tool {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    width: 58px;
    padding: 4px 0;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: transparent;
    cursor: default;
  }
  .tool:not(:disabled):hover {
    background: var(--bg-raised);
    border-color: var(--border);
  }
  .tool:not(:disabled):active {
    background: var(--border);
  }
  .tool:disabled {
    opacity: 0.45;
  }
  .glyph {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 6px;
    background: var(--accent);
    color: #fff;
    font-weight: 600;
    font-size: 14px;
  }
  .label {
    font-size: var(--fs-small);
    white-space: nowrap;
  }
  .spacer {
    flex: 1;
  }
  .version {
    color: var(--fg-muted);
    font-size: var(--fs-small);
    font-family: var(--font-mono);
    padding: 0 4px;
  }
</style>
