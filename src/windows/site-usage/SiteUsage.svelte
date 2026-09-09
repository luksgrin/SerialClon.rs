<script lang="ts">
  import { printPage } from '$lib/prefs';
  /**
   * Site Usage (User Manual §III.D): the interactive frequency table with
   * the filter against a second sequence ("unique here, absent there"), and
   * the printable tables (all / unique in bold / absent in italic).
   */
  import { onMount } from 'svelte';
  import ToolWindow from '$lib/components/ToolWindow.svelte';
  import { api, docIdFromUrl, onDocChanged, type DocumentView, type UsageRow } from '$lib/doc';

  const id = docIdFromUrl();
  let doc = $state<DocumentView | null>(null);
  let docs = $state<[number, string][]>([]);
  let other = $state<number | null>(null);
  let rows = $state<UsageRow[]>([]);
  let tab = $state<'interactive' | 'table'>('interactive');
  let filter = $state<'all' | 'unique' | 'absent' | 'n'>('all');
  let n = $state(2);
  let otherFilter = $state<'any' | 'absent' | 'unique'>('any');
  let columns = $state(3);

  const shown = $derived(
    rows.filter((r) => {
      const ok = filter === 'all' || (filter === 'unique' && r.count === 1) || (filter === 'absent' && r.count === 0) || (filter === 'n' && r.count === n);
      const okOther = other === null || otherFilter === 'any' || (otherFilter === 'absent' && r.other === 0) || (otherFilter === 'unique' && r.other === 1);
      return ok && okOther;
    }),
  );
  const uniques = $derived(rows.filter((r) => r.count === 1));
  const absents = $derived(rows.filter((r) => r.count === 0));

  async function refresh() {
    rows = await api.siteUsage(id, other);
  }
  onMount(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      doc = await api.getDocument(id);
      docs = (await api.listDocuments()).filter(([d]) => d !== id);
      await refresh();
      unlisten = await onDocChanged(id, async (d) => {
        doc = d;
        await refresh();
      });
    })();
    return () => unlisten?.();
  });
</script>

<ToolWindow title="Site Usage" subtitle={doc ? `${doc.name} — ${doc.bases.length} bp` : ''}>
  <div class="tabs no-print" role="tablist">
    <button type="button" role="tab" aria-selected={tab === 'interactive'} onclick={() => (tab = 'interactive')} data-help-name="Interactive" data-help="The dynamic site usage table: filter by frequency, and against a second sequence.">Interactive</button>
    <button type="button" role="tab" aria-selected={tab === 'table'} onclick={() => (tab = 'table')} data-help-name="Table" data-help="The printable site usage tables: all sites, unique sites (bold) and absent sites (italic).">Table</button>
  </div>
  {#if tab === 'interactive'}
    <fieldset class="no-print" data-help-name="Filter" data-help="Show all enzymes, only unique or absent sites, or those cutting exactly n times.">
      <legend>Show</legend>
      <label><input type="radio" value="all" bind:group={filter} /> all</label>
      <label><input type="radio" value="unique" bind:group={filter} /> unique</label>
      <label><input type="radio" value="absent" bind:group={filter} /> absent</label>
      <label><input type="radio" value="n" bind:group={filter} /> cutting <input type="number" min="0" bind:value={n} class="tiny" /> times</label>
    </fieldset>
    <fieldset class="no-print" data-help-name="Compare" data-help="Filter the site usage using another sequence's: e.g. sites unique here and absent there, to find a cloning site that will not cut the insert.">
      <legend>Compare with</legend>
      <select bind:value={other} onchange={refresh}><option value={null}>—</option>{#each docs as [d, name] (d)}<option value={d}>{name}</option>{/each}</select>
      {#if other !== null}
        <label><input type="radio" value="any" bind:group={otherFilter} /> any</label>
        <label><input type="radio" value="absent" bind:group={otherFilter} /> absent there</label>
        <label><input type="radio" value="unique" bind:group={otherFilter} /> unique there</label>
      {/if}
    </fieldset>
    <div class="scroll">
      <table>
        <thead><tr><th>Enzyme</th><th>Site</th><th>Cuts</th>{#if other !== null}<th>Other</th>{/if}</tr></thead>
        <tbody>
          {#each shown as r (r.name)}
            <tr class:bold={r.count === 1} class:italic={r.count === 0}><td>{r.name}</td><td class="mono">{r.site}</td><td class="num">{r.count}</td>{#if other !== null}<td class="num">{r.other}</td>{/if}</tr>
          {/each}
        </tbody>
      </table>
    </div>
    <span class="hint no-print">{shown.length} enzymes</span>
  {:else}
    <div class="row no-print">
      <label>Columns <select bind:value={columns}>{#each [2, 3, 4, 5, 6] as c (c)}<option value={c}>{c}</option>{/each}</select></label>
      <button type="button" onclick={() => void printPage()} data-help-name="Print" data-help="Prints the three tables. Choose the column count for landscape or portrait pages.">Print</button>
    </div>
    <div class="scroll printable">
      <h2>All sites — {doc?.name}</h2>
      <div class="cols" style:column-count={columns}>{#each rows as r (r.name)}<div class:bold={r.count === 1} class:italic={r.count === 0}>{r.name} <span class="mono">{r.site}</span> {r.count}</div>{/each}</div>
      <h2>Unique sites ({uniques.length})</h2>
      <div class="cols" style:column-count={columns}>{#each uniques as r (r.name)}<div class="bold">{r.name} <span class="mono">{r.site}</span></div>{/each}</div>
      <h2>Absent sites ({absents.length})</h2>
      <div class="cols" style:column-count={columns}>{#each absents as r (r.name)}<div class="italic">{r.name} <span class="mono">{r.site}</span></div>{/each}</div>
    </div>
  {/if}
</ToolWindow>

<style>
  .tabs {
    display: flex;
    gap: 2px;
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    padding: 3px 12px;
    border: 1px solid transparent;
    border-bottom: none;
    border-radius: var(--radius) var(--radius) 0 0;
    background: transparent;
  }
  .tabs button[aria-selected='true'] {
    border-color: var(--border);
    background: var(--bg-raised);
    margin-bottom: -1px;
  }
  .tiny {
    width: 48px;
  }
  .bold {
    font-weight: 700;
  }
  .italic {
    font-style: italic;
    color: var(--fg-muted);
  }
  .row {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .printable {
    padding: 8px;
    background: #fff;
    color: #111;
  }
  .printable h2 {
    font-size: 13px;
    margin: 8px 0 4px;
  }
  .cols {
    column-gap: 16px;
    font-size: var(--fs-small);
  }
  @media print {
    .no-print {
      display: none !important;
    }
    .scroll {
      overflow: visible;
      border: none;
    }
  }
</style>
