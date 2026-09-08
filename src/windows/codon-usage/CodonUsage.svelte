<script lang="ts">
  /**
   * Manage / Import Codon Usage (Protein menu; changelog 2.5): the bundled
   * tables (Kazusa, with their source and retrieval date), the one selected
   * for reverse translation, the 64 rows of any table, and import from
   * pasted text (Kazusa or GCG layouts) or from a URL.
   */
  import { onMount } from 'svelte';
  import { message, confirm } from '@tauri-apps/plugin-dialog';
  import { api, type CodonTableView, type CodonUsageRow } from '$lib/doc';

  let tables = $state<CodonTableView[]>([]);
  let current = $state<string | null>(null);
  let rows = $state<CodonUsageRow[]>([]);
  let importing = $state(false);
  let name = $state('');
  let text = $state('');
  let url = $state('');
  let busy = $state(false);

  const table = $derived(tables.find((t) => t.slug === current) ?? null);

  async function load() {
    tables = await api.codonTables();
    if (!current || !tables.some((t) => t.slug === current)) current = tables.find((t) => t.selected)?.slug ?? tables[0]?.slug ?? null;
    await loadRows();
  }
  async function loadRows() {
    rows = current ? await api.codonTableRows(current) : [];
  }
  async function select() {
    if (!current) return;
    await api.selectCodonTable(current);
    await load();
  }
  async function remove() {
    if (!table || table.builtin) return;
    if (!(await confirm(`Delete the table "${table.name}"?`, { title: 'Delete Codon Usage table', kind: 'warning' }))) return;
    await api.deleteCodonTable(table.slug);
    current = null;
    await load();
  }
  async function doImport() {
    busy = true;
    try {
      const t = await api.importCodonTable(name, text.trim() ? text : null, url.trim() ? url : null);
      importing = false;
      name = text = url = '';
      current = t.slug;
      await load();
    } catch (e) {
      await message(String(e), { title: 'Import Codon Usage', kind: 'error' });
    } finally {
      busy = false;
    }
  }

  const grouped = $derived.by(() => {
    const byAa = new Map<string, CodonUsageRow[]>();
    for (const r of rows) {
      const list = byAa.get(r.amino_acid) ?? [];
      list.push(r);
      byAa.set(r.amino_acid, list);
    }
    return [...byAa.entries()].sort(([a], [b]) => a.localeCompare(b));
  });

  onMount(() => void load());
</script>

<div class="codon">
  <section class="list">
    <h2>Codon Usage tables</h2>
    <div class="tables" data-help-name="Tables" data-help="The codon usage tables: the five bundled ones (Kazusa Codon Usage Database, GenBank-derived) and the ones you imported. The selected one is used by Reverse Translate.">
      {#each tables as t (t.slug)}
        <button type="button" class="table" class:active={t.slug === current} onclick={() => { current = t.slug; void loadRows(); }}>
          <span>{t.selected ? '● ' : ''}{t.name}</span>
          <span class="hint">{t.cds ? `${t.cds} CDS` : ''}{t.builtin ? '' : ' · imported'}</span>
        </button>
      {/each}
    </div>
    <div class="tools">
      <button type="button" onclick={select} disabled={!table || table.selected} data-help-name="Use for reverse translation" data-help="Makes this the table used by Protein ▸ Reverse Translate (shown in the Protein menu).">Use for reverse translation</button>
      <button type="button" onclick={() => (importing = !importing)} data-help-name="Import" data-help="Imports a table pasted from the Kazusa Codon Usage Database (either layout) or a GCG CodonFrequency file, or fetched from a URL.">Import…</button>
      <button type="button" onclick={remove} disabled={!table || table.builtin}>Delete</button>
    </div>
    {#if table}
      <p class="source">{table.source}{table.retrieved ? ` — retrieved ${table.retrieved}` : ''}</p>
    {/if}
  </section>

  <section class="detail">
    {#if importing}
      <div class="import">
        <label>Name <input type="text" bind:value={name} placeholder="e.g. Pichia pastoris" /></label>
        <label>Paste a table <textarea rows="8" bind:value={text} placeholder="UUU 22.4( 22308)  UCU  8.5( 8489) … or Gly GGG 1743.00 24.35 0.19 …" spellcheck="false"></textarea></label>
        <label>or fetch from a URL <input type="text" bind:value={url} placeholder="https://www.kazusa.or.jp/codon/cgi-bin/showcodon.cgi?species=…&aa=1&style=N" /></label>
        <div class="tools">
          <button type="button" class="primary" onclick={doImport} disabled={busy}>Import</button>
          <button type="button" onclick={() => (importing = false)}>Cancel</button>
        </div>
      </div>
    {:else}
      <table data-help-name="Codons" data-help="The 64 codons of the selected table grouped by amino acid: fraction among synonymous codons, occurrences per thousand, and count. The most used codon of each amino acid (bold) is the one Reverse Translate picks.">
        <thead><tr><th>aa</th><th>codon</th><th>fraction</th><th>/1000</th><th>count</th></tr></thead>
        <tbody>
          {#each grouped as [aa, list] (aa)}
            {@const best = Math.max(...list.map((r) => r.per_thousand))}
            {#each list as r, i (r.codon)}
              <tr class:best={r.per_thousand === best}>
                <td>{i === 0 ? aa : ''}</td><td class="mono">{r.codon}</td><td class="num">{r.fraction.toFixed(2)}</td><td class="num">{r.per_thousand.toFixed(1)}</td><td class="num">{r.count}</td>
              </tr>
            {/each}
          {/each}
        </tbody>
      </table>
    {/if}
  </section>
</div>

<style>
  .codon {
    display: grid;
    grid-template-columns: 300px 1fr;
    gap: 10px;
    height: 100vh;
    box-sizing: border-box;
    padding: 8px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }
  h2 {
    margin: 0;
    font-size: var(--fs-ui);
    color: var(--fg-muted);
  }
  .tables {
    flex: 1;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
  }
  .table {
    display: flex;
    justify-content: space-between;
    width: 100%;
    padding: 4px 8px;
    border: none;
    border-bottom: 1px solid var(--border);
    background: transparent;
    text-align: left;
  }
  .table.active {
    background: var(--accent);
    color: #fff;
  }
  .tools {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  .source {
    margin: 0;
    font-size: var(--fs-small);
    color: var(--fg-muted);
    word-break: break-all;
  }
  .detail {
    overflow: auto;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    padding: 2px 8px;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }
  th {
    position: sticky;
    top: 0;
    background: var(--bg);
    font-size: var(--fs-small);
  }
  tr.best {
    font-weight: 700;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .num {
    font-family: var(--font-mono);
    text-align: right;
  }
  .import {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .import label {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .import textarea,
  .import input {
    font: inherit;
    font-family: var(--font-mono);
  }
  .primary {
    background: var(--accent);
    color: #fff;
  }
  .hint {
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
</style>
