<script lang="ts">
  /**
   * The Find drawer (User Manual §IV.B): Sequence / Site / ORFs tabs, instant
   * search on both strands, a sortable hit table, and one click to select the
   * hit in the parent Sequence Window. Attached to its window by the shell.
   */
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { api, docIdFromUrl, onDocChanged, requestSelect, type DocumentView, type EnzymeEntry, type Hit, type OrfHit } from '$lib/doc';

  const id = docIdFromUrl();
  import { prefs } from '$lib/prefs';
  /** Minimum ORF size for the "all ORFs" search (Preferences). */
  const MIN_ORF_CODONS = $derived($prefs.orf_min_codons);

  let doc = $state<DocumentView | null>(null);
  let tab = $state<'sequence' | 'peptide' | 'site' | 'orfs'>('sequence');
  let query = $state('');
  let hits = $state<Hit[]>([]);
  let peptideQuery = $state('');
  let peptideHits = $state<OrfHit[]>([]);
  let orfs = $state<OrfHit[]>([]);
  let enzymes = $state<EnzymeEntry[]>([]);
  let enzyme = $state('');
  let siteHits = $state<Hit[]>([]);
  let sortKey = $state<'position' | 'length' | 'forward'>('position');
  let sortAsc = $state(true);
  let selectedRow = $state(-1);

  const currentEnzyme = $derived(enzymes.find((e) => e.name === enzyme));

  function sorted<T extends { position?: number; start?: number; length: number; forward: boolean }>(rows: T[]): T[] {
    const key = sortKey;
    const dir = sortAsc ? 1 : -1;
    return [...rows].sort((a, b) => {
      const av = key === 'position' ? (a.position ?? a.start ?? 0) : key === 'length' ? a.length : Number(a.forward);
      const bv = key === 'position' ? (b.position ?? b.start ?? 0) : key === 'length' ? b.length : Number(b.forward);
      return (av - bv) * dir;
    });
  }
  function sortBy(key: typeof sortKey) {
    if (sortKey === key) sortAsc = !sortAsc;
    else {
      sortKey = key;
      sortAsc = true;
    }
  }

  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  function onQuery() {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(runSearch, 120);
  }
  async function runSearch() {
    const q = query.replace(/[^A-Za-z]/g, '');
    hits = q.length ? await api.findText(id, q) : [];
    selectedRow = -1;
  }
  let peptideTimer: ReturnType<typeof setTimeout> | undefined;
  function onPeptideQuery() {
    clearTimeout(peptideTimer);
    peptideTimer = setTimeout(runPeptide, 120);
  }
  async function runPeptide() {
    const q = peptideQuery.replace(/[^A-Za-z*]/g, '');
    peptideHits = q.length ? await api.findPeptide(id, q) : [];
    selectedRow = -1;
  }
  async function runOrfs() {
    orfs = await api.findOrfs(id, MIN_ORF_CODONS);
    // The original lists ORFs from the largest to the smallest.
    sortKey = 'length';
    sortAsc = false;
    selectedRow = -1;
  }
  async function runSite() {
    siteHits = enzyme ? await api.findSite(id, enzyme) : [];
    selectedRow = -1;
  }
  async function loadEnzymes() {
    enzymes = await api.enzymeList(id);
  }

  function pick(row: number, from: number, to: number) {
    selectedRow = row;
    void requestSelect(id, from, to);
  }

  onMount(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      doc = await api.getDocument(id);
      await loadEnzymes();
      unlisten = await onDocChanged(id, async (d) => {
        doc = d;
        if (tab === 'sequence' && query) await runSearch();
        if (tab === 'peptide' && peptideQuery) await runPeptide();
        if (tab === 'orfs' && orfs.length) await runOrfs();
        await loadEnzymes();
        if (tab === 'site' && enzyme) await runSite();
      });
    })();
    return () => unlisten?.();
  });
</script>

<div class="drawer">
  <div class="tabs" role="tablist">
    <button type="button" role="tab" aria-selected={tab === 'sequence'} onclick={() => (tab = 'sequence')} data-help-name="Sequence tab" data-help="Click on this tab if you want to search for a sub-sequence string.">Sequence</button>
    <button type="button" role="tab" aria-selected={tab === 'peptide'} onclick={() => (tab = 'peptide')} data-help-name="Peptide tab" data-help="Click on this tab if you want to search for a peptide in the six translations of the sequence.">Peptide</button>
    <button type="button" role="tab" aria-selected={tab === 'site'} onclick={() => (tab = 'site')} data-help-name="Site tab" data-help="Click on this tab if you want to search for a restriction site.">Site</button>
    <button type="button" role="tab" aria-selected={tab === 'orfs'} onclick={() => (tab = 'orfs')} data-help-name="ORFs tab" data-help="Click on this tab if you want to search for Open Reading Frames (ORF).">ORFs</button>
  </div>

  {#if tab === 'sequence'}
    <div class="row">
      <input class="query" type="text" bind:value={query} oninput={onQuery} placeholder="nucleotide string (IUPAC accepted)" spellcheck="false" data-help-name="Search field" data-help="Enter here the nucleotide string you want to look for. The search is case-insensitive and done on both strands; degenerate letters are accepted." />
      <button type="button" onclick={runSearch} data-help-name="Find" data-help="Click here to start searching.">Find</button>
    </div>
    <div class="results" data-help-name="Occurrences" data-help="All the occurrences of the sequence you look for. Click on an occurrence to select it in the Sequence Window. Click a column header to sort by position, size or strand.">
      <table>
        <thead><tr><th onclick={() => sortBy('position')}>Position</th><th onclick={() => sortBy('length')}>Size</th><th onclick={() => sortBy('forward')}>Strand</th></tr></thead>
        <tbody>
          {#each sorted(hits) as h, i (h.position + ':' + h.forward)}
            <tr class:selected={selectedRow === i} onclick={() => pick(i, h.position - 1, h.position - 1 + h.length)}>
              <td class="num">{h.position}</td><td class="num">{h.length}</td><td>{h.forward ? '+' : '−'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if query && !hits.length}<p class="hint">Not found.</p>{/if}
    </div>
  {:else if tab === 'peptide'}
    <div class="row">
      <input class="query" type="text" bind:value={peptideQuery} oninput={onPeptideQuery} placeholder="peptide (one-letter code, X = any)" spellcheck="false" data-help-name="Peptide field" data-help="Enter here the amino-acid string (one-letter code) you want to look for. It is searched in the three frames of both strands; X matches any residue." />
      <button type="button" onclick={runPeptide} data-help-name="Find" data-help="Click here to start searching.">Find</button>
    </div>
    <div class="results" data-help-name="Occurrences" data-help="All the occurrences of the peptide, with the strand and the reading frame. Click on an occurrence to select the coding bases in the Sequence Window.">
      <table>
        <thead><tr><th onclick={() => sortBy('position')}>Position</th><th onclick={() => sortBy('length')}>Size</th><th onclick={() => sortBy('forward')}>Strand</th><th>Frame</th></tr></thead>
        <tbody>
          {#each sorted(peptideHits) as o, i (o.start + ':' + o.forward + ':' + o.frame)}
            <tr class:selected={selectedRow === i} onclick={() => pick(i, Math.min(o.start, o.end) - 1, Math.max(o.start, o.end))}>
              <td class="num">{o.position}</td><td class="num">{o.length}</td><td>{o.forward ? '+' : '−'}</td><td>{o.forward ? '+' : '−'}{o.frame}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if peptideQuery && !peptideHits.length}<p class="hint">Not found.</p>{/if}
    </div>
  {:else if tab === 'site'}
    <div class="site">
      <select size="8" bind:value={enzyme} onchange={runSite} ondblclick={runSite} data-help-name="Enzyme list" data-help="Select the restriction site to look for: scroll in the list or type the first letters of the enzyme name to jump to it. Unique sites are in bold, absent sites in italic.">
        {#each enzymes as e (e.name)}
          <option value={e.name} class:unique={e.count === 1} class:absent={e.count === 0}>{e.name}</option>
        {/each}
      </select>
      <div class="selected-enzyme" data-help-name="Selected Enzyme" data-help="The name of the selected enzyme and its recognition site, with the cut position marked by /.">
        <b>{currentEnzyme?.name ?? '—'}</b>
        <span class="mono">{currentEnzyme?.site ?? ''}</span>
        {#if currentEnzyme}<span class="hint">{currentEnzyme.count} site{currentEnzyme.count === 1 ? '' : 's'}</span>{/if}
      </div>
      <button type="button" onclick={runSite} disabled={!enzyme} data-help-name="Find Site" data-help="Finds every occurrence of the selected site. Non-palindromic sites show their orientation.">Find Site</button>
      <div class="results">
        <table>
          <thead><tr><th onclick={() => sortBy('position')}>Position</th><th onclick={() => sortBy('forward')}>Strand</th></tr></thead>
          <tbody>
            {#each sorted(siteHits) as h, i (h.position + ':' + h.forward)}
              <tr class:selected={selectedRow === i} onclick={() => pick(i, h.position - 1, h.position - 1 + h.length)}>
                <td class="num">{h.position}</td><td>{h.forward ? '>' : '<'}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {:else}
    <div class="row">
      <button type="button" onclick={runOrfs} data-help-name="Find ORFs" data-help="Finds every ORF (start codon to stop, nested ones included) on both strands, listed from the largest to the smallest. Position is the lower end of the ORF on either strand. Incomplete ORFs (no stop codon reached) are in italic. The start codon and the minimum size are set in Preferences ▸ ORF.">Find ORFs</button>
      <span class="hint">Start codon: {$prefs.orf_start_codons.replaceAll('|', ' | ')}{MIN_ORF_CODONS > 0 ? ` · ≥ ${MIN_ORF_CODONS} aa` : ''} · both strands</span>
    </div>
    <div class="results">
      <table>
        <thead><tr><th onclick={() => sortBy('position')}>Position</th><th onclick={() => sortBy('length')}>Size</th><th onclick={() => sortBy('forward')}>Strand</th><th>Frame</th></tr></thead>
        <tbody>
          {#each sorted(orfs) as o, i (o.start + ':' + o.forward)}
            <tr class:selected={selectedRow === i} class:incomplete={!o.complete} onclick={() => pick(i, Math.min(o.start, o.end) - 1, Math.max(o.start, o.end))}>
              <td class="num">{o.position}</td><td class="num">{o.length}</td><td>{o.forward ? '+' : '−'}</td><td>{o.forward ? '+' : '−'}{o.frame}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}

  <footer>
    <span class="hint">{doc ? `${doc.name} — ${doc.bases.length} bp` : ''}</span>
    <button type="button" onclick={() => void getCurrentWindow().close()} data-help-name="Close Drawer" data-help="Closes the Find drawer.">Close Drawer</button>
  </footer>
</div>

<style>
  .drawer {
    display: flex;
    flex-direction: column;
    height: 100vh;
    box-sizing: border-box;
    padding: 8px;
    gap: 6px;
  }
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
  .row {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .query {
    flex: 1;
    font: inherit;
    font-family: var(--font-mono);
  }
  .results {
    flex: 1;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
    min-height: 80px;
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
    cursor: default;
  }
  tr.selected {
    background: var(--accent);
    color: #fff;
  }
  tr.incomplete {
    font-style: italic;
  }
  .num {
    font-family: var(--font-mono);
    text-align: right;
  }
  .site {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }
  select {
    font: inherit;
    font-family: var(--font-mono);
  }
  option.unique {
    font-weight: 700;
  }
  option.absent {
    font-style: italic;
    color: var(--fg-muted);
  }
  .selected-enzyme {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .hint {
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
  footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
</style>
