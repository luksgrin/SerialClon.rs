<script lang="ts">
  /** Gateway™ cloning (⌘5): BP and LR reactions, or yeast-style homologous recombination. */
  import { onMount } from 'svelte';
  import { message } from '@tauri-apps/plugin-dialog';
  import ToolWindow from '$lib/components/ToolWindow.svelte';
  import { api } from '$lib/doc';
  import { currentPrefs, prefsReady, savePrefs } from '$lib/prefs';

  let docs = $state<[number, string][]>([]);
  let insert = $state<number | null>(null);
  let vector = $state<number | null>(null);
  let reaction = $state<'bp' | 'lr' | 'hr'>('bp');
  let minHomology = $state(20);
  /** The original's "Use minimal att consensus" checkbox, kept in the preferences. */
  let minimalAtt = $state(currentPrefs().gateway_minimal_att);
  void prefsReady.then((p) => (minimalAtt = p.gateway_minimal_att));
  async function rememberMinimalAtt() {
    const p = await prefsReady;
    await savePrefs({ ...p, gateway_minimal_att: minimalAtt });
  }

  async function refreshDocs() {
    docs = await api.listDocuments();
  }
  async function run() {
    if (insert === null || vector === null) return;
    try {
      await api.gateway(insert, vector, reaction, reaction === 'hr' ? minHomology : null, minimalAtt);
    } catch (e) {
      await message(String(e), { title: 'Gateway cloning', kind: 'error' });
    }
  }
  onMount(() => {
    void refreshDocs();
    const t = setInterval(() => void refreshDocs(), 3000);
    return () => clearInterval(t);
  });
</script>

<ToolWindow title="Gateway™ cloning" subtitle="att sites are found with the Gateway Feature Collection">
  <fieldset data-help-name="Reaction" data-help="BP: an attB-flanked insert (PCR product or expression clone) into an attP donor vector gives an entry clone with attL sites. LR: an attL entry clone into an attR destination vector gives an expression clone with attB sites. Homologous recombination: the insert's two ends share homology with the vector (yeast-style).">
    <legend>Reaction</legend>
    <label><input type="radio" value="bp" bind:group={reaction} /> BP (attB × attP → attL)</label>
    <label><input type="radio" value="lr" bind:group={reaction} /> LR (attL × attR → attB)</label>
    <label><input type="radio" value="hr" bind:group={reaction} /> Homologous recombination</label>
    {#if reaction === 'hr'}<label>min. homology <input type="number" min="8" max="60" bind:value={minHomology} /> bp</label>{/if}
    {#if reaction !== 'hr'}<label data-help-name="Use minimal att consensus" data-help="Locates the att sites with the short core consensus the original uses for every kind of site (TTTGTACAAAAAAG and CTTTCTTGTACAAAGT) instead of the full definitions of the Gateway Feature Collection. Use it when a vector carries unusual att arms."><input type="checkbox" bind:checked={minimalAtt} onchange={rememberMinimalAtt} /> Use minimal att consensus</label>{/if}
  </fieldset>
  <fieldset data-help-name="Insert" data-help="The sequence carrying the insert: attB sites for BP, attL sites for LR, homology arms for recombination. An antisense orientation is handled.">
    <legend>{reaction === 'bp' ? 'attB insert (PCR product / expression clone)' : reaction === 'lr' ? 'Entry clone (attL)' : 'Insert with homology arms'}</legend>
    <select bind:value={insert}><option value={null}>—</option>{#each docs as [id, n] (id)}<option value={id}>{n}</option>{/each}</select>
  </fieldset>
  <fieldset data-help-name="Vector" data-help="The acceptor: an attP donor vector (pDONR) for BP, an attR destination vector (pDEST) for LR, or the vector to recombine into.">
    <legend>{reaction === 'bp' ? 'Donor vector (attP)' : reaction === 'lr' ? 'Destination vector (attR)' : 'Vector'}</legend>
    <select bind:value={vector}><option value={null}>—</option>{#each docs as [id, n] (id)}<option value={id}>{n}</option>{/each}</select>
  </fieldset>
  <div class="row">
    <button type="button" onclick={async () => { await api.openFiles(); await refreshDocs(); }}>Open…</button>
    <button type="button" class="primary" onclick={run} disabled={insert === null || vector === null} data-help-name="Recombine" data-help="Runs the reaction: the product opens in a new Sequence Window with the inherited Features (ccdB is gone) and the provenance in the comments.">Recombine</button>
  </div>
</ToolWindow>

<style>
  .row {
    display: flex;
    gap: 10px;
  }
</style>
