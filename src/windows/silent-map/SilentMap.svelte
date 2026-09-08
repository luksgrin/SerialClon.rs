<script lang="ts">
  /** Silent Map (changelog 1.3): sites that could be introduced in a reading frame without changing the peptide. */
  import { onMount } from 'svelte';
  import { confirm, message } from '@tauri-apps/plugin-dialog';
  import ToolWindow from '$lib/components/ToolWindow.svelte';
  import { api, docIdFromUrl, onDocChanged, requestSelect, type DocumentView, type SilentRow } from '$lib/doc';

  const id = docIdFromUrl();
  const params = new URLSearchParams(window.location.search);
  let doc = $state<DocumentView | null>(null);
  let from = $state(Number(params.get('from') ?? 1));
  let to = $state(Number(params.get('to') ?? 0));
  let rows = $state<SilentRow[]>([]);
  let maxChanges = $state(2);
  let busy = $state(false);
  const shown = $derived(rows.filter((r) => r.changes <= maxChanges));

  async function scan() {
    busy = true;
    try {
      rows = await api.silentMap(id, from, to || (doc?.bases.length ?? 0));
    } catch (e) {
      await message(String(e), { title: 'Silent Map', kind: 'error' });
    } finally {
      busy = false;
    }
  }
  async function apply(r: SilentRow) {
    if (!(await confirm(`Introduce a ${r.enzyme} site at ${r.position} with ${r.changes} silent change${r.changes === 1 ? '' : 's'} (${r.mutated})?`, { title: 'Silent mutation' }))) return;
    try {
      await api.applySilentMutation(id, r.position, r.mutated);
      await scan();
    } catch (e) {
      await message(String(e), { title: 'Silent mutation', kind: 'error' });
    }
  }
  onMount(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      doc = await api.getDocument(id);
      if (!to) to = doc.bases.length;
      unlisten = await onDocChanged(id, (d) => (doc = d));
      if (params.get('from')) await scan();
    })();
    return () => unlisten?.();
  });
</script>

<ToolWindow title="Silent Map" subtitle={doc ? `${doc.name} — reading frame ${from}..${to}` : ''}>
  <fieldset data-help-name="Reading frame" data-help="The ORF to scan: 'From' must be the first base of a codon. Not to be used on a full plasmid: it can be long to generate.">
    <legend>Reading frame</legend>
    <label>From <input type="number" min="1" bind:value={from} /></label>
    <label>To <input type="number" min="1" bind:value={to} /></label>
    <button type="button" class="primary" onclick={scan} disabled={busy}>Scan</button>
    <label>max changes <input type="number" min="1" max="6" bind:value={maxChanges} /></label>
  </fieldset>
  <div class="scroll" data-help-name="Silent sites" data-help="Sites that could be introduced by the listed base changes without altering the translation. Double-click a row to select the position in the Sequence Window; Apply introduces the mutation.">
    <table>
      <thead><tr><th>Position</th><th>Enzyme</th><th>Changes</th><th>Mutated stretch</th><th></th></tr></thead>
      <tbody>
        {#each shown as r (r.enzyme + r.position)}
          <tr ondblclick={() => void requestSelect(id, r.position - 1, r.position - 1 + r.mutated.length)}>
            <td class="num">{r.position}</td><td>{r.enzyme}</td><td class="num">{r.changes}</td><td class="mono">{r.mutated}</td>
            <td><button type="button" onclick={() => void apply(r)} disabled={doc?.locked}>Apply</button></td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  <span class="hint">{rows.length ? `${shown.length} of ${rows.length} possibilities` : busy ? 'scanning…' : ''}</span>
</ToolWindow>
